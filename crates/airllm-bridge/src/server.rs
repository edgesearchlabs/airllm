use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use airllm_orchestrator::Orchestrator;
use airllm_ollama::{ChatOptions, Message, OllamaClient};
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::post,
    Router,
};
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
/// This is a **transparent proxy** to Ollama. The frontend (OpenAirLLM/Ink)
/// manages its own tool calling, system prompts, and permission dialogs.
/// The bridge just forwards messages straight to Ollama and formats the
/// response in OpenAI format. This avoids double-processing (frontend
/// system prompt + orchestrator system prompt) which was causing extreme
/// latency with local models.
async fn chat_completions(
    State(state): State<BridgeState>,
    Json(req): Json<ChatCompletionRequest>,
) -> Result<Json<ChatCompletionResponse>, (StatusCode, String)> {
    info!(
        model = %req.model,
        messages = req.messages.len(),
        tools = req.tools.as_ref().map(|t| t.len()).unwrap_or(0),
        "bridge: /v1/chat/completions"
    );

    // Convert OpenAI messages to Ollama messages.
    // Truncate system prompt to 500 chars for local models — the full
    // OpenAirLLM system prompt is 3000+ tokens which kills performance
    // on small models. The frontend's CLAUDE_CODE_SIMPLE=1 already
    // reduces this, but we truncate as a safety net.
    const MAX_SYSTEM_PROMPT: usize = 800;
    let messages: Vec<Message> = req
        .messages
        .iter()
        .map(|m| {
            let content = if m.role == "system" && m.content.len() > MAX_SYSTEM_PROMPT {
                tracing::warn!(
                    original_len = m.content.len(),
                    truncated_len = MAX_SYSTEM_PROMPT,
                    "System prompt truncated for local model performance"
                );
                format!("{}\n\n[Be concise. Use tools when needed.]", &m.content[..MAX_SYSTEM_PROMPT])
            } else {
                m.content.clone()
            };
            match m.role.as_str() {
                "system" => Message::system(&content),
                "assistant" => Message::assistant(&content),
                _ => Message::user(&content),
            }
        })
        .collect();

    // Forward directly to Ollama — no orchestrator, no double system prompt.
    let chat_options = ChatOptions {
        temperature: req.temperature.unwrap_or(0.7),
        ..ChatOptions::default()
    };

    let content = state
        .ollama
        .chat(&req.model, &messages, chat_options)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    Ok(Json(ChatCompletionResponse {
        id: format!("chatcmpl-{}", now),
        object: "chat.completion".to_string(),
        created: now,
        model: req.model,
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
    }))
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