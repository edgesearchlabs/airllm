use airllm_mcp::run_stdio;
use airllm_ollama::OllamaClient;
use airllm_orchestrator::Orchestrator;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let ollama_url = std::env::var("OLLAMA_URL")
        .unwrap_or_else(|_| "http://localhost:11434".to_string());
    let orchestrator = Orchestrator::new(OllamaClient::new(&ollama_url));
    run_stdio(orchestrator).await
}