use airllm_orchestrator::Orchestrator;
use anyhow::Result;
use clap::Args;

#[derive(Args, Debug)]
pub struct RefactorCmd {
    /// Files or directories to refactor
    pub files: Vec<String>,
    /// Refactor goal description
    #[arg(long)]
    pub goal: String,
}

pub async fn run(cmd: RefactorCmd, orchestrator: &Orchestrator) -> Result<()> {
    let resp = orchestrator.refactor(cmd.files.clone(), &cmd.goal).await?;
    println!("{}", resp.output);
    Ok(())
}
