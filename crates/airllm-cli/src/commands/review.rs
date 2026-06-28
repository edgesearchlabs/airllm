use airllm_orchestrator::Orchestrator;
use anyhow::Result;
use clap::Args;

#[derive(Args, Debug)]
pub struct ReviewCmd {
    /// Files or glob patterns to review
    pub files: Vec<String>,
    /// Optional model override
    #[arg(long)]
    pub model: Option<String>,
}

pub async fn run(cmd: ReviewCmd, orchestrator: &Orchestrator) -> Result<()> {
    let resp = orchestrator.review(cmd.files.clone()).await?;
    println!("{}", resp.output);
    if let Some(model) = cmd.model {
        println!("\n(model override requested: {model})");
    }
    Ok(())
}
