use airllm_ollama::OllamaClient;
use airllm_orchestrator::{AgentRegistry, CodeRequest, Orchestrator};
use futures::StreamExt;
use mockito::{Matcher, Server};

#[tokio::test]
async fn registry_loads_toml_configs() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("agents");
    let registry = AgentRegistry::load_from_dir(&dir).expect("registry");
    assert!(registry.get("coder").is_some());
    assert!(registry.get("architect").is_some());
    assert!(registry.get("planner").is_some());
    assert!(registry.get("security").is_some());
    assert!(registry.get("performance").is_some());
}

#[tokio::test]
async fn orchestrator_code_executes_against_ollama_api() {
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
        .match_body(Matcher::Regex("Implement hello".to_string()))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"message":{"content":"I'll create the file.\n[TOOL_CALL]\n{\"name\": \"file_write\", \"arguments\": {\"file_path\": \"src/main.rs\", \"content\": \"fn main() { println!(\\\"hi\\\"); }\"}}\n[/TOOL_CALL]"},"done":true}"#)
        .create_async()
        .await;
    // Second call: LLM receives tool result and responds without tool calls
    let _chat2 = server
        .mock("POST", "/api/chat")
        .match_body(Matcher::Regex("Tool results".to_string()))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"message":{"content":"Done! Created src/main.rs with fn main()."},"done":true}"#)
        .create_async()
        .await;

    let orchestrator = Orchestrator::new(OllamaClient::new(&server.url()));
    let response = orchestrator
        .code(CodeRequest {
            task: "Implement hello".to_string(),
            language: Some("rust".to_string()),
            files: vec!["src/main.rs".to_string()],
            model_override: None,
            permission_mode: "bypass".to_string(),
            max_rounds: 5,
        })
        .await
        .expect("code response");

    assert!(response.output.contains("Done"));
    // file_write tool expands relative paths to absolute, so check for the filename
    assert!(response.files_written.iter().any(|f| f.ends_with("src/main.rs")));
}

#[tokio::test]
async fn code_stream_yields_tokens() {
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
        .with_body(r#"{"message":{"content":"hello world from orchestrator"},"done":true}"#)
        .create_async()
        .await;

    let orchestrator = Orchestrator::new(OllamaClient::new(&server.url()));
    let request = CodeRequest {
        task: "Write greeting".to_string(),
        language: Some("rust".to_string()),
        files: vec!["src/main.rs".to_string()],
        model_override: None,
        permission_mode: "bypass".to_string(),
        max_rounds: 5,
    };
    let mut stream = orchestrator.code_stream(request).await.expect("stream");
    let mut count = 0;
    while let Some(token) = stream.next().await {
        token.expect("token ok");
        count += 1;
    }
    assert!(count > 0);
}