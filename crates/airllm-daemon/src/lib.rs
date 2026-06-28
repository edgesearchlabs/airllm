//! Autonomous agent daemon — runs agents in continuous loops with
//! scheduler, permissions, and persistent state.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use airllm_orchestrator::{Orchestrator, CodeRequest};
use airllm_permissions::{CheckResult, PermissionEngine};
use airllm_scheduler::Scheduler;
use airllm_state::{AgentStatus, AgentCycle, StateStore};
use airllm_tools::ToolRegistry;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::time::sleep;
use tracing::{error, info, warn};

#[derive(Debug, Error)]
pub enum DaemonError {
    #[error("state error: {0}")]
    State(String),
    #[error("permission error: {0}")]
    Permission(String),
    #[error("orchestrator error: {0}")]
    Orchestrator(String),
    #[error("config error: {0}")]
    Config(String),
}

pub type DaemonResult<T> = std::result::Result<T, DaemonError>;

/// Daemon configuration.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DaemonConfig {
    pub ollama_url: String,
    pub db_path: PathBuf,
    pub permissions_path: Option<PathBuf>,
    pub schedule_path: Option<PathBuf>,
    pub check_interval_secs: u64,
    pub max_cycles_per_agent: u32,
    pub cycle_timeout_secs: u64,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            ollama_url: "http://localhost:11434".into(),
            db_path: PathBuf::from("data/airllm.db"),
            permissions_path: None,
            schedule_path: None,
            check_interval_secs: 60,
            max_cycles_per_agent: 100,
            cycle_timeout_secs: 300,
        }
    }
}

/// The autonomous daemon.
pub struct Daemon {
    config: DaemonConfig,
    state: Arc<StateStore>,
    permissions: Arc<PermissionEngine>,
    scheduler: Arc<Scheduler>,
    tools: Arc<ToolRegistry>,
    orchestrator: Arc<Orchestrator>,
}

impl Daemon {
    /// Create a new daemon with the given configuration.
    pub fn new(config: DaemonConfig) -> DaemonResult<Self> {
        let state = Arc::new(
            StateStore::open(&config.db_path)
                .map_err(|e| DaemonError::State(e.to_string()))?,
        );

        let permissions = Arc::new(
            config
                .permissions_path
                .as_ref()
                .and_then(|p| PermissionEngine::load(p).ok())
                .unwrap_or_else(PermissionEngine::empty),
        );

        let scheduler = Arc::new(
            config
                .schedule_path
                .as_ref()
                .and_then(|p| Scheduler::load(p).ok())
                .unwrap_or_else(Scheduler::empty),
        );

        let tools = Arc::new(airllm_tools::default_registry());

        let ollama = airllm_ollama::OllamaClient::new(&config.ollama_url);
        let orchestrator = Arc::new(Orchestrator::new(ollama));

        Ok(Self {
            config,
            state,
            permissions,
            scheduler,
            tools,
            orchestrator,
        })
    }

    /// Run the daemon loop.
    pub async fn run(&self) -> DaemonResult<()> {
        info!("daemon starting — checking scheduler every {}s", self.config.check_interval_secs);

        loop {
            // 1. Check cron jobs and enqueue due ones
            if let Ok(triggered) = self.scheduler.check_cron_jobs().await {
                if !triggered.is_empty() {
                    info!(jobs = triggered.len(), "cron jobs triggered");
                }
            }

            // 2. Dequeue and execute jobs
            while let Some(entry) = self.scheduler.dequeue().await {
                info!(job_id = %entry.job_id, agent = %entry.agent_name, "executing job");
                match self.execute_agent_cycle(&entry.agent_name, &entry.task).await {
                    Ok(_) => info!(job_id = %entry.job_id, "job completed"),
                    Err(e) => {
                        error!(job_id = %entry.job_id, error = %e, "job failed");
                        // Re-queue with backoff
                        let requeued = self
                            .scheduler
                            .requeue_with_backoff(&entry, 3)
                            .await;
                        if !requeued {
                            warn!(job_id = %entry.job_id, "max retries exceeded, dropping");
                        }
                    }
                }
            }

            // 3. Wait for next check interval
            sleep(Duration::from_secs(self.config.check_interval_secs)).await;
        }
    }

    /// Execute a single agent cycle.
    pub async fn execute_agent_cycle(
        &self,
        agent_name: &str,
        task: &str,
    ) -> DaemonResult<()> {
        let start = std::time::Instant::now();

        // 1. Set agent status to running
        self.state
            .set_agent_status(agent_name, AgentStatus::Running)
            .await
            .map_err(|e| DaemonError::State(e.to_string()))?;

        // 2. Get last cycle number
        let last_cycle = self
            .state
            .get_last_cycle(agent_name)
            .await
            .map_err(|e| DaemonError::State(e.to_string()))?;
        let cycle_number = last_cycle + 1;

        // 3. Check permissions for the "code" tool (default action)
        let perm_check = self
            .permissions
            .check(agent_name, "code", task)
            .await
            .map_err(|e| DaemonError::Permission(e.to_string()))?;

        let (action_result, decision, error) = match perm_check {
            CheckResult::Allow => {
                // Execute via orchestrator
                let req = CodeRequest {
                    task: task.to_string(),
                    language: None,
                    files: Vec::new(),
                    model_override: None,
                };
                match self.orchestrator.code(req).await {
                    Ok(resp) => (resp.output, "continue".into(), None),
                    Err(e) => (String::new(), "error".into(), Some(e.to_string())),
                }
            }
            CheckResult::RequireApproval(pending) => {
                let msg = format!("approval required: {}", pending.id);
                (msg, "awaiting approval".into(), None)
            }
            CheckResult::Deny(reason) => {
                (format!("denied: {reason}"), "denied".into(), None)
            }
        };

        // 4. Record cycle
        let cycle = AgentCycle {
            id: 0,
            agent_name: agent_name.to_string(),
            cycle_number,
            action: task.to_string(),
            result: action_result,
            decision,
            tokens_consumed: 0,
            duration_ms: start.elapsed().as_millis() as i64,
            error,
            created_at: Utc::now(),
        };
        self.state
            .record_cycle(&cycle)
            .await
            .map_err(|e| DaemonError::State(e.to_string()))?;

        // 5. Save checkpoint
        let checkpoint = airllm_state::Checkpoint {
            id: 0,
            agent_name: agent_name.to_string(),
            cycle_number,
            state_json: serde_json::json!({"task": task, "cycle": cycle_number}).to_string(),
            created_at: Utc::now(),
        };
        self.state
            .save_checkpoint(&checkpoint)
            .await
            .map_err(|e| DaemonError::State(e.to_string()))?;

        // 6. Set status back to idle
        self.state
            .set_agent_status(agent_name, AgentStatus::Idle)
            .await
            .map_err(|e| DaemonError::State(e.to_string()))?;

        Ok(())
    }

    /// Get daemon status (all agents).
    pub async fn status(&self) -> DaemonResult<Vec<(String, AgentStatus, i64)>> {
        self.state
            .list_agents()
            .await
            .map_err(|e| DaemonError::State(e.to_string()))
    }

    /// Get the state store (for external access).
    pub fn state(&self) -> &StateStore {
        &self.state
    }

    /// Get the scheduler.
    pub fn scheduler(&self) -> &Scheduler {
        &self.scheduler
    }

    /// Get the permission engine.
    pub fn permissions(&self) -> &PermissionEngine {
        &self.permissions
    }

    /// Get the tool registry.
    pub fn tools(&self) -> &ToolRegistry {
        &self.tools
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_daemon_creation() {
        let config = DaemonConfig {
            db_path: std::env::temp_dir().join("test_daemon.db"),
            ollama_url: "http://localhost:11434".into(),
            ..Default::default()
        };
        let daemon = Daemon::new(config).unwrap();
        let status = daemon.status().await.unwrap();
        assert!(status.is_empty()); // no agents yet
    }

    #[tokio::test]
    async fn test_execute_cycle_records_state() {
        let config = DaemonConfig {
            db_path: std::env::temp_dir().join("test_cycle.db"),
            ollama_url: "http://localhost:11434".into(),
            ..Default::default()
        };
        let daemon = Daemon::new(config).unwrap();
        // This will try to call Ollama — if not running, it records an error cycle
        let result = daemon.execute_agent_cycle("test-agent", "hello").await;
        // Should not panic even if Ollama is down
        let _ = result;
        let status = daemon.status().await.unwrap();
        // Agent should be registered after a cycle
        assert!(status.iter().any(|(name, _, _)| name == "test-agent"));
    }
}