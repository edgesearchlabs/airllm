use std::io::{self, Write};

use airllm_orchestrator::Orchestrator;
use anyhow::Result;
use clap::Args;

#[derive(Args, Debug)]
pub struct ChatCmd {
    /// Optional one-shot prompt; if omitted, enter interactive mode
    #[arg(long)]
    pub prompt: Option<String>,
    /// Model override
    #[arg(long)]
    pub model: Option<String>,
}

pub async fn run(cmd: ChatCmd, orchestrator: &Orchestrator) -> Result<()> {
    if let Some(prompt) = cmd.prompt {
        let resp = orchestrator.chat(&prompt, cmd.model.as_deref()).await?;
        println!("{}", resp);
        return Ok(());
    }

    println!("Entering interactive chat. Type /exit to quit.\n");
    let mut input = String::new();
    loop {
        print!("you> ");
        io::stdout().flush()?;
        input.clear();
        io::stdin().read_line(&mut input)?;
        let line = input.trim();
        if line.eq_ignore_ascii_case("/exit") {
            break;
        }
        if line.is_empty() {
            continue;
        }
        let resp = orchestrator.chat(line, cmd.model.as_deref()).await?;
        println!("assistant> {}\n", resp);
    }

    Ok(())
}
