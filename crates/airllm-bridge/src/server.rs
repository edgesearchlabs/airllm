use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use airllm_orchestrator::Orchestrator;
use airllm_ollama::{ChatOptions, Message, OllamaClient, StreamEvent};
use axum::{
    body::Body,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Json, Response},
    routing::post,
    Router,
};
use futures::stream::{self, StreamExt};
use serde_json::json;
use tower_http::cors::CorsLayer;
use tracing::info;

use crate::types::{
    ChatCompletionRequest, ChatCompletionResponse, ChatChoice, ChatMessage, ModelInfo,
    ModelsResponse, Usage,
};

/// State shared across all request handlers.
#[derive(Clone)]
pub struct BridgeState {
    pub ollama: OllamaClient,
    // Orchestrator kept for potential future use (CLI path), but NOT used
    // in the bridge proxy path — the frontend manages its own tools.
    #[allow(dead_code)]
    pub orchestrator: Arc<Orchestrator>,
}

/// The bridge server — an OpenAI-compatible HTTP API backed by our Rust orchestrator.
pub struct BridgeServer {
    state: BridgeState,
    addr: SocketAddr,
}

impl BridgeServer {
    pub fn new(orchestrator: Orchestrator, ollama: OllamaClient, addr: SocketAddr) -> Self {
        Self {
            state: BridgeState {
                ollama,
                orchestrator: Arc::new(orchestrator),
            },
            addr,
        }
    }

    /// Build the Axum router with all endpoints.
    pub fn router(&self) -> Router {
        Router::new()
            .route("/v1/chat/completions", post(chat_completions))
            .route("/v1/models", axum::routing::get(list_models))
            .route("/api/chat", post(ollama_chat))
            .route("/api/tags", axum::routing::get(ollama_tags))
            .route("/health", axum::routing::get(health))
            .layer(CorsLayer::permissive())
            .with_state(self.state.clone())
    }

    /// Start serving on the configured address.
    pub async fn serve(self) -> anyhow::Result<()> {
        let router = self.router();
        let listener = tokio::net::TcpListener::bind(self.addr).await?;
        info!("Bridge server listening on {}", self.addr);
        axum::serve(listener, router).await?;
        Ok(())
    }
}

// ── Handlers ────────────────────────────────────────────────────────────────

/// POST /v1/chat/completions — OpenAI-compatible chat completions.
///
/// Supports both streaming (SSE) and non-streaming responses.
/// When `stream: true`, returns Server-Sent Events in OpenAI format.
/// The bridge is a transparent proxy to Ollama — the frontend manages
/// its own tool calling, system prompts, and permission dialogs.
async fn chat_completions(
    State(state): State<BridgeState>,
    Json(req): Json<ChatCompletionRequest>,
) -> Result<Response, (StatusCode, String)> {
    info!(
        model = %req.model,
        messages = req.messages.len(),
        tools = req.tools.as_ref().map(|t| t.len()).unwrap_or(0),
        stream = req.stream,
        "bridge: /v1/chat/completions"
    );

    // Force a single model for all requests to avoid the frontend making
    // parallel calls with a heavy model (e.g. qwen2.5-coder:14b takes 90s).
    // The AIRLLM_FORCE_MODEL env var, if set, overrides the requested model.
    // This ensures ALL calls (including background/title/summary) use the
    // same fast model.
    let effective_model = std::env::var("AIRLLM_FORCE_MODEL")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| req.model.clone());
    if effective_model != req.model {
        info!(
            requested = %req.model,
            forced = %effective_model,
            "Forcing model for performance"
        );
    }

    // Convert OpenAI messages to Ollama messages.
    // Replace the frontend's system prompt with a minimal one for local models.
    // The OpenAirLLM/Ink frontend sends 3000+ token system prompts that kill
    // performance on small local models. We replace it with a concise prompt
    // that preserves the essential instructions.
    let messages: Vec<Message> = req
        .messages
        .iter()
        .map(|m| {
            if m.role == "system" {
                // Replace any system prompt > 200 chars with a minimal version
                // that keeps the key instructions but removes the bloat.
                if m.content.len() > 200 {
                    tracing::info!(
                        original_len = m.content.len(),
                        "Replacing large system prompt with minimal version for local model"
                    );
                    Message::system(
                        "You are a helpful coding assistant. Be concise. \
                         Use available tools when needed. Answer directly."
                    )
                } else {
                    Message::system(&m.content)
                }
            } else {
                match m.role.as_str() {
                    "assistant" => Message::assistant(&m.content),
                    _ => Message::user(&m.content),
                }
            }
        })
        .collect();

    // Use num_ctx that matches Ollama's default (4096) to avoid model reload.
    // Changing num_ctx forces Ollama to reload the model with a different
    // context window, which adds 5-10s latency on every request.
    let chat_options = ChatOptions {
        temperature: req.temperature.unwrap_or(0.7),
        ..ChatOptions::default()
    };

    // Convert OpenAI tools to Ollama format.
    // OpenAI: {"type":"function","function":{"name":"...","parameters":{...}}}
    // Ollama: {"type":"function","function":{"name":"...","parameters":{...}}}
    // They're the same format! Just pass through.
    let ollama_tools: Option<Vec<serde_json::Value>> = req.tools.map(|t| {
        t.into_iter()
            .map(|tool| {
                // Ollama expects the same format as OpenAI
                if tool.get("type").and_then(|v| v.as_str()) == Some("function") {
                    tool
                } else {
                    // Wrap in function format if not already
                    json!({"type": "function", "function": tool})
                }
            })
            .collect()
    });

    if req.stream {
        // Streaming SSE response — real token-by-token streaming from Ollama.
        let model = effective_model.clone();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let chat_id = format!("chatcmpl-{}", now);

        let tools_ref = ollama_tools.as_deref();
        let ollama_stream = state
            .ollama
            .chat_stream_with_tools(&effective_model, &messages, chat_options, tools_ref)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        // Transform each Ollama event into an OpenAI SSE chunk
        let sse_model = model.clone();
        let sse_chat_id = chat_id.clone();
        let had_tool_calls = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let had_tool_calls_clone = had_tool_calls.clone();

        // First chunk: send role:assistant to initialize the message
        // (OpenAI SSE format requires this before content chunks)
        let init_chunk = format!(
            "data: {}\n\n",
            json!({
                "id": &chat_id,
                "object": "chat.completion.chunk",
                "created": now,
                "model": &model,
                "choices": [{
                    "index": 0,
                    "delta": {"role": "assistant", "content": ""},
                    "finish_reason": null
                }]
            })
        );

        let sse_stream = ollama_stream.map(move |result| {
            match result {
                Ok(StreamEvent::Content(token)) => {
                    let data = json!({
                        "id": &sse_chat_id,
                        "object": "chat.completion.chunk",
                        "created": now,
                        "model": &sse_model,
                        "choices": [{
                            "index": 0,
                            "delta": {"content": token},
                            "finish_reason": null
                        }]
                    });
                    Ok(format!("data: {}\n\n", data))
                }
                Ok(StreamEvent::ToolCalls(tool_calls)) => {
                    had_tool_calls_clone.store(true, std::sync::atomic::Ordering::Relaxed);
                    // Convert Ollama tool_calls to OpenAI format
                    let openai_tool_calls: Vec<serde_json::Value> = tool_calls
                        .as_array()
                        .map(|arr| {
                            arr.iter().enumerate().map(|(i, tc)| {
                                json!({
                                    "index": i,
                                    "id": tc.get("id").and_then(|v| v.as_str()).unwrap_or(&format!("call_{}", i)),
                                    "type": "function",
                                    "function": {
                                        "name": tc.get("function").and_then(|f| f.get("name")).and_then(|n| n.as_str()).unwrap_or(""),
                                        "arguments": tc.get("function").and_then(|f| f.get("arguments")).map(|a| a.to_string()).unwrap_or_else(|| "{}".to_string())
                                    }
                                })
                            }).collect()
                        })
                        .unwrap_or_default();

                    // Send tool_calls with null finish_reason (not done yet)
                    let data = json!({
                        "id": &sse_chat_id,
                        "object": "chat.completion.chunk",
                        "created": now,
                        "model": &sse_model,
                        "choices": [{
                            "index": 0,
                            "delta": {"tool_calls": openai_tool_calls},
                            "finish_reason": null
                        }]
                    });
                    Ok(format!("data: {}\n\n", data))
                }
                Err(e) => {
                    let data = json!({
                        "id": &sse_chat_id,
                        "object": "chat.completion.chunk",
                        "created": now,
                        "model": &sse_model,
                        "choices": [{
                            "index": 0,
                            "delta": {},
                            "finish_reason": "stop"
                        }],
                        "error": e.to_string()
                    });
                    Ok(format!("data: {}\n\ndata: [DONE]\n\n", data))
                }
            }
        });

        // Prepend the init chunk (role:assistant) before the content stream
        let init_stream = stream::once(async move {
            Ok::<String, std::convert::Infallible>(init_chunk)
        });
        let combined_stream = init_stream.chain(sse_stream);

        // Append the final done event
        let final_model = model.clone();
        let final_chat_id = chat_id.clone();
        let final_had_tc = had_tool_calls.clone();
        let final_stream = combined_stream.chain(stream::once(async move {
            let finish_reason = if final_had_tc.load(std::sync::atomic::Ordering::Relaxed) { "tool_calls" } else { "stop" };
            let final_data = json!({
                "id": &final_chat_id,
                "object": "chat.completion.chunk",
                "created": now,
                "model": &final_model,
                "choices": [{
                    "index": 0,
                    "delta": {},
                    "finish_reason": finish_reason
                }]
            });
            Ok::<String, std::convert::Infallible>(format!("data: {}\n\ndata: [DONE]\n\n", final_data))
        }));

        let body = Body::from_stream(final_stream);

        Ok(Response::builder()
            .status(StatusCode::OK)
            .header("content-type", "text/event-stream")
            .header("cache-control", "no-cache")
            .header("connection", "keep-alive")
            .body(body)
            .unwrap())
    } else {
        // Non-streaming JSON response
        let tools_ref = ollama_tools.as_deref();
        let content = state
            .ollama
            .chat_with_tools(&effective_model, &messages, chat_options, tools_ref)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let response = ChatCompletionResponse {
            id: format!("chatcmpl-{}", now),
            object: "chat.completion".to_string(),
            created: now,
            model: effective_model,
            choices: vec![ChatChoice {
                index: 0,
                message: ChatMessage {
                    role: "assistant".to_string(),
                    content,
                },
                finish_reason: "stop".to_string(),
            }],
            usage: Usage {
                prompt_tokens: 0,
                completion_tokens: 0,
                total_tokens: 0,
            },
        };

        Ok(Json(response).into_response())
    }
}

/// GET /v1/models — list available models (OpenAI-compatible).
async fn list_models(State(state): State<BridgeState>) -> Json<ModelsResponse> {
    let models = state
        .ollama
        .list_models()
        .await
        .map(|m| {
            m.into_iter()
                .map(|model| ModelInfo {
                    id: model.name,
                    object: "model".to_string(),
                    created: 0,
                    owned_by: "edgesearch".to_string(),
                })
                .collect()
        })
        .unwrap_or_default();

    Json(ModelsResponse {
        object: "list".to_string(),
        data: models,
    })
}

/// POST /api/chat — Ollama-compatible chat endpoint (for direct Ollama passthrough).
async fn ollama_chat(
    State(state): State<BridgeState>,
    Json(req): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let model = req
        .get("model")
        .and_then(|v| v.as_str())
        .unwrap_or("qwen3.5:4b")
        .to_string();

    let messages: Vec<Message> = req
        .get("messages")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|m| {
                    let role = m.get("role")?.as_str()?;
                    let content = m.get("content")?.as_str()?;
                    let msg = match role {
                        "system" => Message::system(content),
                        "assistant" => Message::assistant(content),
                        _ => Message::user(content),
                    };
                    Some(msg)
                })
                .collect()
        })
        .unwrap_or_default();

    let result = state
        .ollama
        .chat(&model, &messages, ChatOptions::default())
        .await;

    match result {
        Ok(content) => Json(json!({
            "message": {"role": "assistant", "content": content},
            "done": true
        })),
        Err(e) => Json(json!({
            "error": e.to_string(),
            "done": true
        })),
    }
}

/// GET /api/tags — Ollama-compatible tags endpoint.
async fn ollama_tags(State(state): State<BridgeState>) -> Json<serde_json::Value> {
    let models = state
        .ollama
        .list_models()
        .await
        .map(|m| {
            m.into_iter()
                .map(|model| {
                    json!({
                        "name": model.name,
                        "size": model.size,
                        "details": {"quantization_level": model.quantization}
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    Json(json!({"models": models}))
}

/// GET /health — health check.
async fn health() -> impl IntoResponse {
    Json(json!({"status": "ok", "service": "OpenAirLLM Bridge by EdgeSearch"}))
}