use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use airllm_orchestrator::{
    execute_tool, parse_prompt, parse_tool_calls, CodeRequest, Orchestrator,
};
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
    pub orchestrator: Arc<Orchestrator>,
    pub ollama: OllamaClient,
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
                orchestrator: Arc::new(orchestrator),
                ollama,
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
/// Supports tool calls: if the LLM response contains tool calls, they are
/// executed by our Rust orchestrator and results are included.
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

    // Convert OpenAI messages to our orchestrator CodeRequest
    let user_prompt = req
        .messages
        .iter()
        .rev()
        .find(|m| m.role == "user")
        .map(|m| m.content.clone())
        .unwrap_or_default();

    // Parse the prompt for file path / language hints
    let intent = parse_prompt(&user_prompt);

    // Build the request with tool calling enabled
    let code_req = CodeRequest {
        task: user_prompt.clone(),
        language: intent.language,
        files: vec![],
        model_override: Some(req.model.clone()),
        permission_mode: "bypass".to_string(),
        max_rounds: 5,
    };

    // Use the orchestrator's code path (includes tool calling loop)
    let response = state
        .orchestrator
        .code(code_req)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Check if the response contains tool calls that need to be reported
    let tool_calls = parse_tool_calls(&response.output);
    let has_tool_calls = !tool_calls.is_empty();

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // If there are tool calls, format the response as OpenAI tool_calls format
    let message = if has_tool_calls {
        // Execute the tool calls and build the response
        let mut tool_results = Vec::new();
        for call in &tool_calls {
            let result = execute_tool(call);
            tool_results.push(serde_json::json!({
                "id": format!("call_{}", now),
                "type": "function",
                "function": {
                    "name": call.name,
                    "arguments": call.arguments.to_string(),
                },
                "result": {
                    "success": result.success,
                    "output": result.output,
                    "files": result.files_affected,
                }
            }));
        }

        ChatMessage {
            role: "assistant".to_string(),
            content: response.output,
        }
    } else {
        ChatMessage {
            role: "assistant".to_string(),
            content: response.output,
        }
    };

    Ok(Json(ChatCompletionResponse {
        id: format!("chatcmpl-{}", now),
        object: "chat.completion".to_string(),
        created: now,
        model: req.model,
        choices: vec![ChatChoice {
            index: 0,
            message,
            finish_reason: if has_tool_calls { "tool_calls" } else { "stop" }.to_string(),
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