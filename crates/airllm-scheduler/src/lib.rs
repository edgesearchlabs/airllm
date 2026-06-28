//! Scheduler for autonomous agents — cron jobs, webhook triggers, execution queue.

use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use cron::Schedule;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use thiserror::Error;
use tokio::sync::Mutex;

#[derive(Debug, Error)]
pub enum SchedulerError {
    #[error("invalid cron expression: {0}")]
    InvalidCron(String),
    #[error("job not found: {0}")]
    JobNotFound(String),
    #[error("config error: {0}")]
    Config(String),
    #[error("toml parse error: {0}")]
    Toml(#[from] toml::de::Error),
}

pub type SchedResult<T> = std::result::Result<T, SchedulerError>;

/// A scheduled job.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScheduledJob {
    pub id: String,
    pub agent_name: String,
    pub task: String,
    pub trigger: Trigger,
    pub enabled: bool,
    pub priority: i32,
    pub max_retries: u32,
    pub retry_delay_secs: u64,
}

/// How a job is triggered.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Trigger {
    /// Cron expression (e.g. "0 9 * * * *" = every day at 9am).
    Cron {
        expression: String,
        timezone: String,
    },
    /// Webhook HTTP endpoint (e.g. "/trigger/messaging").
    Webhook {
        endpoint: String,
    },
    /// Run once immediately.
    Once,
}

/// Execution queue entry.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QueueEntry {
    pub job_id: String,
    pub agent_name: String,
    pub task: String,
    pub priority: i32,
    pub attempts: u32,
    pub queued_at: DateTime<Utc>,
}

/// Scheduler configuration loaded from TOML.
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct SchedulerConfig {
    pub jobs: Vec<ScheduledJob>,
}

/// The scheduler manages jobs and an execution queue.
#[derive(Clone)]
pub struct Scheduler {
    config: Arc<Mutex<SchedulerConfig>>,
    queue: Arc<Mutex<Vec<QueueEntry>>>,
}

impl Scheduler {
    /// Load scheduler config from a TOML file.
    pub fn load(path: impl AsRef<std::path::Path>) -> SchedResult<Self> {
        let raw = std::fs::read_to_string(path.as_ref())
            .map_err(|e| SchedulerError::Config(format!("read error: {e}")))?;
        Self::from_toml(&raw)
    }

    /// Parse config from TOML string.
    pub fn from_toml(raw: &str) -> SchedResult<Self> {
        let config: SchedulerConfig = toml::from_str(raw)?;
        Ok(Self {
            config: Arc::new(Mutex::new(config)),
            queue: Arc::new(Mutex::new(Vec::new())),
        })
    }

    /// Create an empty scheduler.
    pub fn empty() -> Self {
        Self {
            config: Arc::new(Mutex::new(SchedulerConfig::default())),
            queue: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Add a job to the scheduler.
    pub async fn add_job(&self, job: ScheduledJob) -> SchedResult<()> {
        // Validate cron if applicable
        if let Trigger::Cron { ref expression, .. } = job.trigger {
            Self::parse_cron(expression)?;
        }
        self.config.lock().await.jobs.push(job);
        Ok(())
    }

    /// Remove a job by ID.
    pub async fn remove_job(&self, id: &str) -> SchedResult<()> {
        let mut config = self.config.lock().await;
        let before = config.jobs.len();
        config.jobs.retain(|j| j.id != id);
        if config.jobs.len() == before {
            return Err(SchedulerError::JobNotFound(id.to_string()));
        }
        Ok(())
    }

    /// List all jobs.
    pub async fn list_jobs(&self) -> Vec<ScheduledJob> {
        self.config.lock().await.jobs.clone()
    }

    /// Trigger a job manually by ID.
    pub async fn trigger(&self, id: &str) -> SchedResult<()> {
        let config = self.config.lock().await;
        let job = config
            .jobs
            .iter()
            .find(|j| j.id == id)
            .ok_or_else(|| SchedulerError::JobNotFound(id.to_string()))?;
        self.enqueue(&job.id, &job.agent_name, &job.task, job.priority).await;
        Ok(())
    }

    /// Trigger by webhook endpoint path.
    pub async fn trigger_webhook(&self, endpoint: &str) -> SchedResult<Vec<String>> {
        let jobs_to_trigger: Vec<(String, String, String, i32)> = {
            let config = self.config.lock().await;
            config
                .jobs
                .iter()
                .filter(|j| j.enabled)
                .filter_map(|job| {
                    if let Trigger::Webhook { endpoint: ref ep } = job.trigger {
                        if ep == endpoint {
                            return Some((
                                job.id.clone(),
                                job.agent_name.clone(),
                                job.task.clone(),
                                job.priority,
                            ));
                        }
                    }
                    None
                })
                .collect()
        };
        let mut triggered = Vec::new();
        for (id, agent, task, priority) in jobs_to_trigger {
            self.enqueue(&id, &agent, &task, priority).await;
            triggered.push(id);
        }
        Ok(triggered)
    }

    /// Get the next job from the queue (highest priority first).
    pub async fn dequeue(&self) -> Option<QueueEntry> {
        let mut queue = self.queue.lock().await;
        if queue.is_empty() {
            return None;
        }
        // Sort by priority (higher = first), then by queued_at (earlier = first)
        queue.sort_by(|a, b| {
            b.priority
                .cmp(&a.priority)
                .then(a.queued_at.cmp(&b.queued_at))
        });
        Some(queue.remove(0))
    }

    /// Get the current queue length.
    pub async fn queue_len(&self) -> usize {
        self.queue.lock().await.len()
    }

    /// Check cron jobs and enqueue due ones.
    pub async fn check_cron_jobs(&self) -> SchedResult<Vec<String>> {
        let config = self.config.lock().await;
        let now = Utc::now();
        let mut triggered = Vec::new();
        let jobs_to_trigger: Vec<(String, String, String, i32)> = config
            .jobs
            .iter()
            .filter(|j| j.enabled)
            .filter_map(|job| {
                if let Trigger::Cron { ref expression, .. } = job.trigger {
                    let schedule = Self::parse_cron(expression).ok()?;
                    if let Some(next) = schedule.upcoming(Utc).next() {
                        if next <= now {
                            return Some((
                                job.id.clone(),
                                job.agent_name.clone(),
                                job.task.clone(),
                                job.priority,
                            ));
                        }
                    }
                }
                None
            })
            .collect();
        drop(config);
        for (id, agent, task, priority) in jobs_to_trigger {
            self.enqueue(&id, &agent, &task, priority).await;
            triggered.push(id);
        }
        Ok(triggered)
    }

    async fn enqueue(&self, id: &str, agent: &str, task: &str, priority: i32) {
        self.queue.lock().await.push(QueueEntry {
            job_id: id.to_string(),
            agent_name: agent.to_string(),
            task: task.to_string(),
            priority,
            attempts: 0,
            queued_at: Utc::now(),
        });
    }

    /// Re-queue a failed job with backoff.
    pub async fn requeue_with_backoff(&self, entry: &QueueEntry, max_retries: u32) -> bool {
        if entry.attempts >= max_retries {
            return false;
        }
        let delay = Duration::from_secs(2u64.pow(entry.attempts));
        tokio::time::sleep(delay).await;
        let mut queue = self.queue.lock().await;
        queue.push(QueueEntry {
            attempts: entry.attempts + 1,
            ..entry.clone()
        });
        true
    }

    fn parse_cron(expr: &str) -> SchedResult<Schedule> {
        Schedule::from_str(expr).map_err(|e| SchedulerError::InvalidCron(format!("{expr}: {e}")))
    }

    /// Validate a config file.
    pub fn validate_config(path: impl AsRef<std::path::Path>) -> SchedResult<()> {
        let raw = std::fs::read_to_string(path.as_ref())
            .map_err(|e| SchedulerError::Config(format!("read error: {e}")))?;
        let config: SchedulerConfig = toml::from_str(&raw)?;
        for job in &config.jobs {
            if let Trigger::Cron { ref expression, .. } = job.trigger {
                Self::parse_cron(expression)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_add_and_list_job() {
        let sched = Scheduler::empty();
        let job = ScheduledJob {
            id: "job-1".into(),
            agent_name: "social-media".into(),
            task: "post daily".into(),
            trigger: Trigger::Cron {
                expression: "0 9 * * * *".into(),
                timezone: "America/Sao_Paulo".into(),
            },
            enabled: true,
            priority: 1,
            max_retries: 3,
            retry_delay_secs: 60,
        };
        sched.add_job(job).await.unwrap();
        let jobs = sched.list_jobs().await;
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].id, "job-1");
    }

    #[tokio::test]
    async fn test_invalid_cron() {
        let sched = Scheduler::empty();
        let job = ScheduledJob {
            id: "bad".into(),
            agent_name: "test".into(),
            task: "test".into(),
            trigger: Trigger::Cron {
                expression: "invalid cron".into(),
                timezone: "UTC".into(),
            },
            enabled: true,
            priority: 0,
            max_retries: 1,
            retry_delay_secs: 10,
        };
        let result = sched.add_job(job).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_trigger_and_dequeue() {
        let sched = Scheduler::empty();
        sched
            .add_job(ScheduledJob {
                id: "job-1".into(),
                agent_name: "test".into(),
                task: "do something".into(),
                trigger: Trigger::Once,
                enabled: true,
                priority: 5,
                max_retries: 1,
                retry_delay_secs: 10,
            })
            .await
            .unwrap();
        sched.trigger("job-1").await.unwrap();
        assert_eq!(sched.queue_len().await, 1);
        let entry = sched.dequeue().await.unwrap();
        assert_eq!(entry.job_id, "job-1");
        assert_eq!(entry.priority, 5);
    }

    #[tokio::test]
    async fn test_webhook_trigger() {
        let sched = Scheduler::empty();
        sched
            .add_job(ScheduledJob {
                id: "wh-1".into(),
                agent_name: "messaging".into(),
                task: "send message".into(),
                trigger: Trigger::Webhook {
                    endpoint: "/trigger/messaging".into(),
                },
                enabled: true,
                priority: 1,
                max_retries: 2,
                retry_delay_secs: 30,
            })
            .await
            .unwrap();
        let triggered = sched.trigger_webhook("/trigger/messaging").await.unwrap();
        assert_eq!(triggered.len(), 1);
        assert_eq!(sched.queue_len().await, 1);
    }

    #[tokio::test]
    async fn test_priority_ordering() {
        let sched = Scheduler::empty();
        // Enqueue low priority first
        sched.enqueue("low", "agent", "task", 1).await;
        sched.enqueue("high", "agent", "task", 10).await;
        let first = sched.dequeue().await.unwrap();
        assert_eq!(first.job_id, "high"); // higher priority dequeued first
    }

    #[tokio::test]
    async fn test_remove_job() {
        let sched = Scheduler::empty();
        sched
            .add_job(ScheduledJob {
                id: "to-remove".into(),
                agent_name: "test".into(),
                task: "test".into(),
                trigger: Trigger::Once,
                enabled: true,
                priority: 0,
                max_retries: 1,
                retry_delay_secs: 10,
            })
            .await
            .unwrap();
        sched.remove_job("to-remove").await.unwrap();
        assert_eq!(sched.list_jobs().await.len(), 0);
    }
}