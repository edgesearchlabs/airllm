use airllm_orchestrator::Orchestrator;
use anyhow::Result;

pub async fn run(orchestrator: &Orchestrator) -> Result<()> {
    let models = orchestrator.list_models().await?;
    println!("Available models:");
    for name in models {
        println!("- {}", name);
    }
    Ok(())
}
