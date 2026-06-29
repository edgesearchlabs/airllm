use airllm_ollama::OllamaClient;
use airllm_orchestrator::{decompose_request, Agent, AgentConfig, CodeRequest};
use mockito::Server;

fn architect_agent() -> Agent {
    Agent::from_config(
        AgentConfig {
            name: "architect".to_string(),
            default_model: "qwen3-coder-next:q8_0".to_string(),
            fallback_model: Some("qwen3.6:27b".to_string()),
            system_prompt: String::new(),
            parallelizable: false,
            max_concurrent: 1,
            temperature: 0.1,
            top_p: 0.9,
            routing_patterns: Vec::new(),
        },
        "architect prompt".to_string(),
    )
}

#[tokio::test]
async fn decompose_parses_json_array() {
    let mut server = Server::new_async().await;
    let _mock = server
        .mock("POST", "/api/chat")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{"message":{"content":"[{\"id\":\"t1\",\"description\":\"Implement API\",\"agent_name\":\"coder\",\"input_files\":[\"src/lib.rs\"]}]"},"done":true}"#,
        )
        .create_async()
        .await;

    let request = CodeRequest {
        task: "Implement API".to_string(),
        language: Some("rust".to_string()),
        files: vec!["src/lib.rs".to_string()],
        model_override: None,
        permission_mode: "bypass".to_string(),
        max_rounds: 5,
    };

    let subtasks = decompose_request(
        &request,
        &architect_agent(),
        "qwen3-coder-next:q8_0",
        &OllamaClient::new(&server.url()),
    )
    .await
    .expect("subtasks");
    assert_eq!(subtasks.len(), 1);
    assert_eq!(subtasks[0].agent_name, "coder");
}

#[tokio::test]
async fn decompose_falls_back_when_response_is_not_json() {
    let mut server = Server::new_async().await;
    let _mock = server
        .mock("POST", "/api/chat")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"message":{"content":"not json"},"done":true}"#)
        .create_async()
        .await;

    let request = CodeRequest {
        task: "Fix bug".to_string(),
        language: None,
        files: vec!["src/lib.rs".to_string()],
        model_override: None,
        permission_mode: "bypass".to_string(),
        max_rounds: 5,
    };

    let subtasks = decompose_request(
        &request,
        &architect_agent(),
        "qwen3-coder-next:q8_0",
        &OllamaClient::new(&server.url()),
    )
    .await
    .expect("subtasks");
    assert_eq!(subtasks.len(), 1);
    assert_eq!(subtasks[0].agent_name, "debugger");
}