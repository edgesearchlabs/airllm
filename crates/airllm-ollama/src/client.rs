use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::RwLock;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::error::{OllamaError, Result};
use crate::types::{ChatMetrics, ChatOptions, Message, ModelInfo};

/// Async client for the Ollama HTTP API.
#[derive(Clone)]
pub struct OllamaClient {
    base_url: String,
    http: Client,
    model_cache: Arc<RwLock<Option<CachedModels>>>,
    default_keep_alive: String,
}

struct CachedModels {
    models: Vec<ModelInfo>,
    fetched_at: Instant,
}

const MODELS_CACHE_TTL: Duration = Duration::from_secs(30);
const DEFAULT_KEEP_ALIVE: &str = "30m";

// ── Internal request / response types ──────────────────────────────────────

#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: &'a [Message],
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    keep_alive: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    options: Option<ChatOptions>,
}

#[derive(Serialize)]
struct WarmRequest<'a> {
    model: &'a str,
    prompt: &'a str,
    stream: bool,
    keep_alive: &'a str,
    options: serde_json::Value,
}

#[derive(Deserialize)]
struct ChatResponse {
    message: ChatMessage,
    #[allow(dead_code)]
    done: bool,
}

#[derive(Deserialize)]
struct ChatMessage {
    #[allow(dead_code)]
    role: Option<String>,
    content: String,
}

#[derive(Deserialize)]
struct ModelsResponse {
    models: Vec<ModelEntry>,
}

#[derive(Deserialize)]
struct ModelEntry {
    name: String,
    size: u64,
    details: ModelDetails,
}

#[derive(Deserialize)]
struct ModelDetails {
    #[serde(rename = "quantization_level")]
    quantization: Option<String>,
}

// ── Public API ──────────────────────────────────────────────────────────────

impl OllamaClient {
    /// Create a new client pointing at the given Ollama base URL
    /// (e.g. `http://localhost:11434`) with a 300s default timeout.
    pub fn new(base_url: &str) -> Self {
        Self::new_with_timeout(base_url, Duration::from_secs(300))
    }

    /// Create a new client with a custom request timeout.
    pub fn new_with_timeout(base_url: &str, timeout: Duration) -> Self {
        let http = Client::builder()
            .timeout(timeout)
            .build()
            .expect("failed to build reqwest client");
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            http,
            model_cache: Arc::new(RwLock::new(None)),
            default_keep_alive: DEFAULT_KEEP_ALIVE.to_string(),
        }
    }

    /// Send a non-streaming chat request and return the full response text.
    pub async fn chat(
        &self,
        model: &str,
        messages: &[Message],
        options: ChatOptions,
    ) -> Result<String> {
        let req = ChatRequest {
            model,
            messages,
            stream: false,
            keep_alive: Some(&self.default_keep_alive),
            options: Some(options),
        };

        let url = format!("{}/api/chat", self.base_url);
        debug!("POST {} (model={})", url, model);

        let resp = self.http.post(&url).json(&req).send().await?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            if status == 404 {
                return Err(OllamaError::ModelNotFound(model.to_string()));
            }
            return Err(OllamaError::Http { status, body });
        }

        let chat_resp: ChatResponse = resp.json().await?;
        Ok(chat_resp.message.content)
    }

    /// Send a streaming chat request. Returns an async stream of token chunks
    /// as they arrive from Ollama (NDJSON stream: one JSON object per line).
    pub async fn chat_stream(
        &self,
        model: &str,
        messages: &[Message],
        options: ChatOptions,
    ) -> Result<impl futures::Stream<Item = Result<String>>> {
        let req = ChatRequest {
            model,
            messages,
            stream: true,
            keep_alive: Some(&self.default_keep_alive),
            options: Some(options),
        };

        let url = format!("{}/api/chat", self.base_url);
        debug!("POST {} (model={}, stream=true)", url, model);

        let resp = self.http.post(&url).json(&req).send().await?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            if status == 404 {
                return Err(OllamaError::ModelNotFound(model.to_string()));
            }
            return Err(OllamaError::Http { status, body });
        }

        // Ollama streams NDJSON: {"message":{"content":"tok"},"done":false}\n
        // We convert this into a stream of content strings.
        let byte_stream = resp.bytes_stream();

        // Use a buffer to accumulate partial NDJSON lines
        let stream = futures::stream::unfold(
            (byte_stream, String::new()),
            |(mut byte_stream, mut buffer)| async move {
                use futures::StreamExt;

                loop {
                    // Try to parse a complete line from the buffer
                    if let Some(newline_pos) = buffer.find('\n') {
                        let line = buffer[..newline_pos].to_string();
                        buffer = buffer[newline_pos + 1..].to_string();

                        if line.is_empty() {
                            continue;
                        }

                        // Parse the NDJSON line
                        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&line) {
                            let content = v
                                .get("message")
                                .and_then(|m| m.get("content"))
                                .and_then(|c| c.as_str())
                                .unwrap_or("")
                                .to_string();
                            let done = v.get("done").and_then(|d| d.as_bool()).unwrap_or(false);

                            if !content.is_empty() {
                                return Some((Ok(content), (byte_stream, buffer)));
                            }
                            if done {
                                return None;
                            }
                        }
                        continue;
                    }

                    // Need more data from the byte stream
                    match byte_stream.next().await {
                        Some(Ok(chunk)) => {
                            buffer.push_str(&String::from_utf8_lossy(&chunk));
                        }
                        Some(Err(e)) => {
                            return Some((
                                Err(OllamaError::Http {
                                    status: 500,
                                    body: e.to_string(),
                                }),
                                (byte_stream, buffer),
                            ));
                        }
                        None => {
                            // Stream ended — try to parse any remaining buffer
                            if buffer.trim().is_empty() {
                                return None;
                            }
                            // Treat remaining buffer as a final line
                            let line = std::mem::take(&mut buffer);
                            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&line) {
                                let content = v
                                    .get("message")
                                    .and_then(|m| m.get("content"))
                                    .and_then(|c| c.as_str())
                                    .unwrap_or("")
                                    .to_string();
                                if !content.is_empty() {
                                    return Some((Ok(content), (byte_stream, buffer)));
                                }
                            }
                            return None;
                        }
                    }
                }
            },
        );

        Ok(stream)
    }

    /// Send a non-streaming chat request and return both the response text
    /// and detailed metrics (latency, tokens, tokens/s).
    pub async fn chat_with_metrics(
        &self,
        model: &str,
        messages: &[Message],
        options: ChatOptions,
    ) -> Result<(String, ChatMetrics)> {
        let start = Instant::now();

        let req = ChatRequest {
            model,
            messages,
            stream: false,
            keep_alive: Some(&self.default_keep_alive),
            options: Some(options.clone()),
        };

        let url = format!("{}/api/chat", self.base_url);
        debug!("POST {} (model={})", url, model);

        let resp = self.http.post(&url).json(&req).send().await?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            if status == 404 {
                return Err(OllamaError::ModelNotFound(model.to_string()));
            }
            return Err(OllamaError::Http { status, body });
        }

        let chat_resp: ChatResponse = resp.json().await?;
        let output = chat_resp.message.content;
        let latency_ms = start.elapsed().as_millis() as u64;
        let metrics = ChatMetrics::from_request(model, messages, &options, latency_ms, &output);
        Ok((output, metrics))
    }

    /// List all models available in the Ollama instance.
    pub async fn list_models(&self) -> Result<Vec<ModelInfo>> {
        if let Some(models) = self.cached_models() {
            return Ok(models);
        }

        let url = format!("{}/api/tags", self.base_url);
        debug!("GET {}", url);

        let resp = self.http.get(&url).send().await?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            return Err(OllamaError::Http { status, body });
        }

        let models_resp: ModelsResponse = resp.json().await?;

        let models: Vec<ModelInfo> = models_resp
            .models
            .into_iter()
            .map(|m| ModelInfo {
                name: m.name,
                size: format_size(m.size),
                quantization: m.details.quantization.unwrap_or_default(),
            })
            .collect();

        self.store_models_cache(models.clone());

        Ok(models)
    }

    /// Check whether a specific model is available.
    pub async fn model_available(&self, model: &str) -> Result<bool> {
        let models = self.list_models().await?;
        Ok(models.iter().any(|m| m.name == model))
    }

    /// Ask Ollama to load a model into memory and keep it alive.
    pub async fn prewarm_model(&self, model: &str) -> Result<()> {
        let req = WarmRequest {
            model,
            prompt: "",
            stream: false,
            keep_alive: &self.default_keep_alive,
            options: serde_json::json!({
                "num_predict": 0,
            }),
        };

        let url = format!("{}/api/generate", self.base_url);
        debug!("POST {} (warm model={})", url, model);

        let resp = self.http.post(&url).json(&req).send().await?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            if status == 404 {
                return Err(OllamaError::ModelNotFound(model.to_string()));
            }
            return Err(OllamaError::Http { status, body });
        }

        Ok(())
    }

    /// Return the base URL this client is configured with.
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    fn cached_models(&self) -> Option<Vec<ModelInfo>> {
        let cache = self.model_cache.read();
        let cached = cache.as_ref()?;
        if cached.fetched_at.elapsed() <= MODELS_CACHE_TTL {
            Some(cached.models.clone())
        } else {
            None
        }
    }

    fn store_models_cache(&self, models: Vec<ModelInfo>) {
        let mut cache = self.model_cache.write();
        *cache = Some(CachedModels {
            models,
            fetched_at: Instant::now(),
        });
    }
}

/// Human-readable file size from bytes.
fn format_size(bytes: u64) -> String {
    const GB: u64 = 1_073_741_824;
    const MB: u64 = 1_048_576;
    const KB: u64 = 1_024;

    if bytes >= GB {
        format!("{:.1}GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1}MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1}KB", bytes as f64 / KB as f64)
    } else {
        format!("{}B", bytes)
    }
}