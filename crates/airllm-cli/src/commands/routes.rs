use airllm_ollama::Complexity;
use airllm_orchestrator::Orchestrator;
use anyhow::Result;

pub async fn run(orchestrator: &Orchestrator) -> Result<()> {
    let router = orchestrator.router();
    println!("Routing rules (by complexity):");
    println!("- Low    → {}", router.select_model(&Complexity::Low));
    println!("- Medium → {}", router.select_model(&Complexity::Medium));
    println!("- High   → {}", router.select_model(&Complexity::High));
    println!("- Cloud  → {}", router.select_model(&Complexity::Cloud));
    Ok(())
}
