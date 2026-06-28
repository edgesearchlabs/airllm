use airllm_ollama::OllamaClient;
use airllm_orchestrator::{consolidate_results, Agent, AgentConfig, AgentResult};

#[tokio::test]
async fn consolidate_single_result_returns_direct_output() {
    let results = vec![AgentResult {
        task_id: "task-1".to_string(),
        output: "impl done".to_string(),
        files: vec!["src/lib.rs".to_string()],
        success: true,
    }];

    let response = consolidate_results(
        results,
        None,
        None,
        &OllamaClient::new("http://localhost:11434"),
    )
    .await
    .expect("response");

    assert_eq!(response.output, "impl done");
    assert_eq!(response.files_written, vec!["src/lib.rs".to_string()]);
}

#[tokio::test]
async fn consolidate_multiple_results_merges_outputs_without_conflict() {
    let reviewer = Agent::from_config(
        AgentConfig {
            name: "reviewer".to_string(),
            default_model: "qwen3.6:27b".to_string(),
            fallback_model: None,
            system_prompt: String::new(),
            parallelizable: false,
            max_concurrent: 1,
            temperature: 0.1,
            top_p: 0.9,
            routing_patterns: Vec::new(),
        },
        "review prompt".to_string(),
    );

    let response = consolidate_results(
        vec![
            AgentResult {
                task_id: "task-1".to_string(),
                output: "part one".to_string(),
                files: vec!["src/a.rs".to_string()],
                success: true,
            },
            AgentResult {
                task_id: "task-2".to_string(),
                output: "part two".to_string(),
                files: vec!["src/b.rs".to_string()],
                success: true,
            },
        ],
        Some(&reviewer),
        Some("qwen3.6:27b"),
        &OllamaClient::new("http://localhost:11434"),
    )
    .await
    .expect("response");

    assert!(response.output.contains("part one"));
    assert!(response.output.contains("part two"));
    assert_eq!(response.files_written.len(), 2);
}