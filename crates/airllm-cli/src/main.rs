mod commands;
mod config;
mod tui;

use std::path::PathBuf;

use airllm_ollama::OllamaClient;
use airllm_orchestrator::Orchestrator;
use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing::Level;
use tracing_subscriber::fmt;

use crate::commands::{chat, code, models, refactor, review, routes, test};
use crate::config::Config;

#[derive(Parser, Debug)]
#[command(name = "airllm", about = "AirLLM CLI/TUI", version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Ollama base URL (env: OLLAMA_URL)
    #[arg(long, env = "OLLAMA_URL", default_value = "http://localhost:11434")]
    ollama_url: String,

    /// Optional config file path (TOML)
    #[arg(long)]
    config: Option<PathBuf>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Code(code::CodeCmd),
    Review(review::ReviewCmd),
    Test(test::TestCmd),
    Refactor(refactor::RefactorCmd),
    Chat(chat::ChatCmd),
    Models,
    Routes,
}

#[tokio::main]
async fn main() -> Result<()> {
    let _ = fmt().with_max_level(Level::INFO).try_init();

    let cli = Cli::parse();
    let cfg = Config::load(cli.config.as_deref())?;

    let mut ollama_url = cli.ollama_url.clone();
    if ollama_url == "http://localhost:11434" {
        if let Some(cfg_url) = cfg.ollama_url.clone() {
            ollama_url = cfg_url;
        }
    }

    let orchestrator = Orchestrator::new(OllamaClient::new(&ollama_url));

    match cli.command {
        Commands::Code(cmd) => code::run(cmd, &orchestrator).await?,
        Commands::Review(cmd) => review::run(cmd, &orchestrator).await?,
        Commands::Test(cmd) => test::run(cmd, &orchestrator).await?,
        Commands::Refactor(cmd) => refactor::run(cmd, &orchestrator).await?,
        Commands::Chat(cmd) => chat::run(cmd, &orchestrator).await?,
        Commands::Models => models::run(&orchestrator).await?,
        Commands::Routes => routes::run(&orchestrator).await?,
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clap_parses_commands() {
        let args = vec!["airllm", "models"]; // simple smoke test
        let cli = Cli::parse_from(args);
        match cli.command {
            Commands::Models => {}
            _ => panic!("parsed wrong command"),
        }
    }
}