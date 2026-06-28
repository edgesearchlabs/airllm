//! Daemon binary entry point.

use std::path::PathBuf;

use airllm_daemon::{Daemon, DaemonConfig};
use clap::{Parser, Subcommand};
use tracing::Level;
use tracing_subscriber::fmt;

#[derive(Parser)]
#[command(name = "airllm-daemon", about = "AirLLM autonomous agent daemon")]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Ollama base URL
    #[arg(long, env = "OLLAMA_URL", default_value = "http://localhost:11434")]
    ollama_url: String,

    /// SQLite database path
    #[arg(long, default_value = "data/airllm.db")]
    db: PathBuf,

    /// Permissions config path
    #[arg(long)]
    permissions: Option<PathBuf>,

    /// Schedule config path
    #[arg(long)]
    schedule: Option<PathBuf>,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the daemon loop
    Run,
    /// Show daemon status
    Status,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = fmt().with_max_level(Level::INFO).try_init();
    let cli = Cli::parse();

    let config = DaemonConfig {
        ollama_url: cli.ollama_url,
        db_path: cli.db,
        permissions_path: cli.permissions,
        schedule_path: cli.schedule,
        ..Default::default()
    };

    let daemon = Daemon::new(config)?;

    match cli.command {
        Commands::Run => {
            daemon.run().await?;
        }
        Commands::Status => {
            let agents = daemon.status().await?;
            if agents.is_empty() {
                println!("No agents registered.");
            } else {
                println!("{:<20} {:<10} Last Cycle", "Agent", "Status");
                println!("{:-<50}", "");
                for (name, status, last_cycle) in agents {
                    println!("{:<20} {:<10} {}", name, status, last_cycle);
                }
            }
        }
    }

    Ok(())
}