use airllm_orchestrator::{CodeRequest, Orchestrator};
use anyhow::Result;
use clap::Args;

use crate::tui::stream::render_stream;

#[derive(Args, Debug)]
pub struct CodeCmd {
    /// High-level coding task description
    pub task: String,
    /// Language hint (optional)
    #[arg(long)]
    pub language: Option<String>,
    /// Output directory or file root
    #[arg(long, default_value = ".")]
    pub output: String,
    /// Force a specific model
    #[arg(long)]
    pub model: Option<String>,
    /// Render streaming output in the TUI
    #[arg(long)]
    pub stream: bool,
    /// Permission mode: default (prompt), accept (auto-approve files), bypass (no prompts)
    #[arg(long, default_value = "default")]
    pub permissions: String,
    /// Max tool call rounds
    #[arg(long, default_value_t = 5)]
    pub max_rounds: u32,
}

pub async fn run(cmd: CodeCmd, orchestrator: &Orchestrator) -> Result<()> {
    let request = CodeRequest {
        task: cmd.task.clone(),
        language: cmd.language.clone(),
        files: vec![cmd.output.clone()],
        model_override: cmd.model.clone(),
        permission_mode: cmd.permissions.clone(),
        max_rounds: cmd.max_rounds,
    };

    if cmd.stream {
        let mut stream = orchestrator.code_stream(request).await?;
        render_stream(&mut stream).await?;
    } else {
        let resp = orchestrator.code(request).await?;
        println!("{}", resp.output);

        if !resp.files_written.is_empty() {
            println!("\n---");
            println!("📁 Files: {}", resp.files_written.join(", "));
        }
    }

    Ok(())
}
