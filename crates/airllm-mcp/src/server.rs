use std::io::{self, BufRead, Write};
use std::sync::Arc;

use airllm_orchestrator::{CodeRequest, Orchestrator};
use anyhow::Result;
use serde_json::{json, Value};
use tracing::info;

use crate::error::McpError;
use crate::tools::available_tools;

/// Run a minimal stdio-based MCP loop. It accepts JSON lines:
/// {"tool":"code","args":{...}}
pub async fn run_stdio(orchestrator: Orchestrator) -> Result<()> {
    let orchestrator = Arc::new(orchestrator);
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    let ready = json!({
        "status": "ready",
        "tools": available_tools(),
    });
    writeln!(stdout, "{}", serde_json::to_string(&ready)?)?;
    stdout.flush()?;

    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let resp = handle_line(line, orchestrator.clone()).await;
        let serialized = serde_json::to_string(&resp).unwrap_or_else(|e| format!("{{\"error\":\"{e}\"}}"));
        writeln!(stdout, "{}", serialized)?;
        stdout.flush()?;
    }
    Ok(())
}

async fn handle_line(line: String, orchestrator: Arc<Orchestrator>) -> Value {
    let parsed: Value = match serde_json::from_str(&line) {
        Ok(v) => v,
        Err(err) => return json!({"error": format!("invalid json: {err}")}),
    };
    let tool = parsed.get("tool").and_then(Value::as_str).unwrap_or("").to_string();
    let args = parsed.get("args").cloned().unwrap_or_else(|| json!({}));

    match dispatch(&tool, args, orchestrator).await {
        Ok(v) => {
            info!("tool" = %tool, "status" = "ok");
            json!({"result": v})
        }
        Err(err) => {
            info!("tool" = %tool, "status" = "error", "error" = %err);
            json!({"error": err.to_string()})
        }
    }
}

async fn dispatch(tool: &str, args: Value, orchestrator: Arc<Orchestrator>) -> Result<Value, McpError> {
    match tool {
        "code" => {
            let task = args
                .get("task")
                .and_then(Value::as_str)
                .ok_or_else(|| McpError::InvalidArgs("task is required".into()))?;
            let language = args.get("language").and_then(Value::as_str).map(|s| s.to_string());
            let files = args
                .get("files")
                .and_then(Value::as_array)
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                .unwrap_or_default();
            let model_override = args.get("model_override").and_then(Value::as_str).map(|s| s.to_string());

            let req = CodeRequest {
                task: task.to_string(),
                language,
                files,
                model_override,
                permission_mode: "bypass".to_string(),
                max_rounds: 5,
            };
            let resp = orchestrator.code(req).await.map_err(|e| McpError::Orchestrator(e.to_string()))?;
            Ok(json!({"output": resp.output, "files_written": resp.files_written, "agent_used": resp.agent_used, "model_used": resp.model_used}))
        }
        "review" => {
            let files: Vec<String> = args
                .get("files")
                .and_then(Value::as_array)
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                .unwrap_or_default();
            let resp = orchestrator
                .review(files)
                .await
                .map_err(|e| McpError::Orchestrator(e.to_string()))?;
            Ok(json!({"output": resp.output}))
        }
        "test" => {
            let files: Vec<String> = args
                .get("files")
                .and_then(Value::as_array)
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                .unwrap_or_default();
            let framework = args.get("framework").and_then(Value::as_str).map(|s| s.to_string());
            let resp = orchestrator
                .test(files, framework)
                .await
                .map_err(|e| McpError::Orchestrator(e.to_string()))?;
            Ok(json!({"output": resp.output}))
        }
        "list_models" => {
            let models = orchestrator
                .list_models()
                .await
                .map_err(|e| McpError::Orchestrator(e.to_string()))?;
            Ok(json!({"models": models}))
        }
        _ => Err(McpError::ToolNotFound(tool.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use airllm_ollama::OllamaClient;
    use mockito::Server;

    #[tokio::test]
    async fn dispatch_code_works() {
        let mut server = Server::new_async().await;
        let _models = server
            .mock("GET", "/api/tags")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"models":[{"name":"qwen3.6:27b","size":1000,"details":{"quantization_level":"Q4_K_M"}}]}"#)
            .create_async()
            .await;
        let _chat = server
            .mock("POST", "/api/chat")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"message":{"content":"File: src/main.rs\nfn main() {}"},"done":true}"#)
            .create_async()
            .await;

        let orch = Arc::new(Orchestrator::new(OllamaClient::new(&server.url())));
        let args = json!({"task": "hello", "language": "rust", "files": ["src/main.rs"]});
        let resp = dispatch("code", args, orch).await.expect("ok");
        assert!(resp.get("output").is_some());
    }
}
