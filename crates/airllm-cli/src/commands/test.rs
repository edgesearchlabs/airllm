use airllm_orchestrator::Orchestrator;
use anyhow::Result;
use clap::Args;

#[derive(Args, Debug)]
pub struct TestCmd {
    /// Files or directories to test
    pub files: Vec<String>,
    /// Test framework hint (e.g. pytest, cargo)
    #[arg(long)]
    pub framework: Option<String>,
}

pub async fn run(cmd: TestCmd, orchestrator: &Orchestrator) -> Result<()> {
    let resp = orchestrator
        .test(cmd.files.clone(), cmd.framework.clone())
        .await?;
    println!("{}", resp.output);
    Ok(())
}
