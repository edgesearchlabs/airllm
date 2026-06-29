mod commands;
mod config;

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
#[command(name = "airllm", about = "AirLLM CLI/TUI — multi-agent code orchestration + autonomous platform", version)]
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
    /// Start the autonomous daemon
    Daemon {
        /// SQLite database path
        #[arg(long, default_value = "data/airllm.db")]
        db: PathBuf,
        /// Permissions config path
        #[arg(long)]
        permissions: Option<PathBuf>,
        /// Schedule config path
        #[arg(long)]
        schedule: Option<PathBuf>,
    },
    /// Show daemon status
    Status {
        #[arg(long, default_value = "data/airllm.db")]
        db: PathBuf,
    },
    /// Agent management
    Agent {
        #[command(subcommand)]
        action: AgentAction,
    },
    /// Schedule management
    Schedule {
        #[command(subcommand)]
        action: ScheduleAction,
    },
    /// Model training pipeline
    Train {
        /// Base model to fine-tune
        #[arg(long)]
        base: String,
        /// New model name
        #[arg(long)]
        name: String,
        /// Dataset path (JSONL)
        #[arg(long)]
        dataset: PathBuf,
        /// Training method
        #[arg(long, default_value = "lora")]
        method: String,
        /// Number of epochs
        #[arg(long, default_value_t = 3)]
        epochs: u32,
        /// Output directory
        #[arg(long, default_value = "models")]
        output: PathBuf,
    },
    /// List fine-tuned models
    Trained,
    /// Permissions management
    Permissions {
        #[command(subcommand)]
        action: PermissionsAction,
    },
}

#[derive(Subcommand, Debug)]
enum AgentAction {
    /// List configured agents
    List {
        #[arg(long, default_value = "data/airllm.db")]
        db: PathBuf,
    },
    /// Run an agent once
    Run {
        /// Agent name
        name: String,
        /// Task description
        #[arg(long)]
        task: String,
        #[arg(long, default_value = "data/airllm.db")]
        db: PathBuf,
    },
}

#[derive(Subcommand, Debug)]
enum ScheduleAction {
    /// List scheduled jobs
    List {
        #[arg(long)]
        config: Option<PathBuf>,
    },
    /// Trigger a job manually
    Trigger {
        /// Job ID
        id: String,
        #[arg(long)]
        config: Option<PathBuf>,
    },
}

#[derive(Subcommand, Debug)]
enum PermissionsAction {
    /// Validate permissions config
    Validate {
        /// Path to permissions.toml
        config: PathBuf,
    },
    /// List pending approvals
    Pending,
    /// Approve a pending action
    Approve {
        /// Approval ID
        id: String,
    },
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
        Commands::Daemon { db, permissions, schedule } => {
            let config = airllm_daemon::DaemonConfig {
                ollama_url: ollama_url.clone(),
                db_path: db,
                permissions_path: permissions,
                schedule_path: schedule,
                ..Default::default()
            };
            let daemon = airllm_daemon::Daemon::new(config)
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            daemon.run().await.map_err(|e| anyhow::anyhow!("{e}"))?;
        }
        Commands::Status { db } => {
            let config = airllm_daemon::DaemonConfig {
                ollama_url: ollama_url.clone(),
                db_path: db,
                ..Default::default()
            };
            let daemon = airllm_daemon::Daemon::new(config)
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            let agents = daemon.status().await.map_err(|e| anyhow::anyhow!("{e}"))?;
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
        Commands::Agent { action } => match action {
            AgentAction::List { db } => {
                let store = airllm_state::StateStore::open(&db)
                    .map_err(|e| anyhow::anyhow!("{e}"))?;
                let agents = store.list_agents().await
                    .map_err(|e| anyhow::anyhow!("{e}"))?;
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
            AgentAction::Run { name, task, db } => {
                let config = airllm_daemon::DaemonConfig {
                    ollama_url: ollama_url.clone(),
                    db_path: db,
                    ..Default::default()
                };
                let daemon = airllm_daemon::Daemon::new(config)
                    .map_err(|e| anyhow::anyhow!("{e}"))?;
                daemon.execute_agent_cycle(&name, &task).await
                    .map_err(|e| anyhow::anyhow!("{e}"))?;
                println!("Agent '{name}' executed task: {task}");
            }
        },
        Commands::Schedule { action } => match action {
            ScheduleAction::List { config } => {
                let sched = if let Some(p) = config {
                    airllm_scheduler::Scheduler::load(&p)
                        .map_err(|e| anyhow::anyhow!("{e}"))?
                } else {
                    airllm_scheduler::Scheduler::empty()
                };
                let jobs = sched.list_jobs().await;
                if jobs.is_empty() {
                    println!("No scheduled jobs.");
                } else {
                    println!("{:<10} {:<15} {:<30} Trigger", "ID", "Agent", "Task");
                    println!("{:-<80}", "");
                    for job in jobs {
                        let trigger = match &job.trigger {
                            airllm_scheduler::Trigger::Cron { expression, .. } => format!("cron: {expression}"),
                            airllm_scheduler::Trigger::Webhook { endpoint } => format!("webhook: {endpoint}"),
                            airllm_scheduler::Trigger::Once => "once".into(),
                        };
                        println!("{:<10} {:<15} {:<30} {}", job.id, job.agent_name, job.task, trigger);
                    }
                }
            }
            ScheduleAction::Trigger { id, config } => {
                let sched = if let Some(p) = config {
                    airllm_scheduler::Scheduler::load(&p)
                        .map_err(|e| anyhow::anyhow!("{e}"))?
                } else {
                    airllm_scheduler::Scheduler::empty()
                };
                sched.trigger(&id).await.map_err(|e| anyhow::anyhow!("{e}"))?;
                println!("Job '{id}' triggered.");
            }
        },
        Commands::Train { base, name, dataset, method, epochs, output } => {
            let pipeline = airllm_training::TrainingPipeline::new(&ollama_url);
            let config = airllm_training::TrainingConfig {
                base_model: base,
                new_model_name: name,
                method: match method.as_str() {
                    "dpo" => airllm_training::TrainingMethod::Dpo,
                    _ => airllm_training::TrainingMethod::Lora,
                },
                dataset_path: dataset,
                epochs,
                output_dir: output,
                ..Default::default()
            };
            let result = pipeline.train(&config).await
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            println!("Training complete: {}", result.model_name);
            println!("  Modelfile: {}", result.modelfile_path.display());
            println!("  Adapter: {}", result.adapter_path.display());
            println!("  Dataset entries: {}/{}", result.dataset_stats.valid_entries, result.dataset_stats.total_entries);
            println!("  Method: {:?}", result.method);
            println!("  Epochs: {}", result.epochs);
        }
        Commands::Trained => {
            let pipeline = airllm_training::TrainingPipeline::new(&ollama_url);
            let models = pipeline.list_fine_tuned().await
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            if models.is_empty() {
                println!("No fine-tuned models found.");
            } else {
                println!("Fine-tuned models:");
                for m in models {
                    println!("  - {m}");
                }
            }
        }
        Commands::Permissions { action } => match action {
            PermissionsAction::Validate { config } => {
                airllm_permissions::PermissionEngine::validate_config(&config)
                    .map_err(|e| anyhow::anyhow!("{e}"))?;
                println!("Permissions config is valid.");
            }
            PermissionsAction::Pending => {
                println!("No daemon running — pending approvals are managed by the daemon.");
            }
            PermissionsAction::Approve { id } => {
                println!("Approval '{id}' — run this against a running daemon.");
            }
        },
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