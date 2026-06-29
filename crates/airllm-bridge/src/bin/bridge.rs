use std::net::SocketAddr;
use airllm_bridge::BridgeServer;
use airllm_ollama::OllamaClient;
use airllm_orchestrator::Orchestrator;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_target(false)
        .init();

    let ollama_url = std::env::var("OLLAMA_BASE_URL")
        .unwrap_or_else(|_| "http://localhost:11434".to_string());

    let bind_addr: SocketAddr = std::env::var("AIRLLM_BRIDGE_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:18080".to_string())
        .parse()?;

    let ollama = OllamaClient::new_with_timeout(
        &ollama_url,
        std::time::Duration::from_secs(120),
    );
    let orchestrator = Orchestrator::new(ollama.clone());

    let server = BridgeServer::new(orchestrator, ollama, bind_addr);
    server.serve().await
}