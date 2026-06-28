use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use airllm_ollama::OllamaClient;
use airllm_orchestrator::{Agent, AgentConfig, SubTask};
use mockito::{Matcher, Server};

fn temp_file_path() -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    std::env::temp_dir().join(format!("airllm_orchestrator_test_{nanos}.txt"))
}

#[tokio::test]
async fn agent_execute_reads_context_and_returns_output() {
    let mut server = Server::new_async().await;
    let _mock = server
        .mock("POST", "/api/chat")
        .match_body(Matcher::Regex("hello world".to_string()))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"message":{"content":"File: src/main.rs\nfn main() {}"},"done":true}"#)
        .create_async()
        .await;

    let file = temp_file_path();
    fs::write(&file, "hello world").expect("write temp file");

    let config = AgentConfig {
        name: "coder".to_string(),
        default_model: "qwen3.6:27b".to_string(),
        fallback_model: Some("qwen3.5:4b".to_string()),
        system_prompt: String::new(),
        parallelizable: true,
        max_concurrent: 2,
        temperature: 0.2,
        top_p: 0.9,
        routing_patterns: Vec::new(),
    };
    let agent = Agent::from_config(config, "You are a coder".to_string());
    let task = SubTask {
        id: "t1".to_string(),
        description: "Implement main".to_string(),
        agent_name: "coder".to_string(),
        input_files: vec![file.display().to_string()],
    };

    let result = agent
        .execute(&task, &OllamaClient::new(&server.url()))
        .await
        .expect("agent result");
    assert!(result.output.contains("fn main"));
    assert_eq!(result.files, vec!["src/main.rs".to_string()]);

    let _ = fs::remove_file(file);
}