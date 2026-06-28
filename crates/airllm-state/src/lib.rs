//! Persistent state for autonomous agents — SQLite-backed.
//!
//! Stores: agent state, execution cycles, audit trail, checkpoints.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::Result;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::Mutex;

pub use chrono::{DateTime, Utc};

#[derive(Debug, Error)]
pub enum StateError {
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),
}

pub type StateResult<T> = std::result::Result<T, StateError>;

/// Agent execution status.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AgentStatus {
    Idle,
    Running,
    Paused,
    Stopped,
    Error,
}

impl std::fmt::Display for AgentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentStatus::Idle => write!(f, "idle"),
            AgentStatus::Running => write!(f, "running"),
            AgentStatus::Paused => write!(f, "paused"),
            AgentStatus::Stopped => write!(f, "stopped"),
            AgentStatus::Error => write!(f, "error"),
        }
    }
}

impl AgentStatus {
    pub fn parse(s: &str) -> Self {
        match s {
            "running" => AgentStatus::Running,
            "paused" => AgentStatus::Paused,
            "stopped" => AgentStatus::Stopped,
            "error" => AgentStatus::Error,
            _ => AgentStatus::Idle,
        }
    }
}

/// A single execution cycle of an autonomous agent.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AgentCycle {
    pub id: i64,
    pub agent_name: String,
    pub cycle_number: i64,
    pub action: String,
    pub result: String,
    pub decision: String,
    pub tokens_consumed: i64,
    pub duration_ms: i64,
    pub error: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// A checkpoint for resume after failure.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Checkpoint {
    pub id: i64,
    pub agent_name: String,
    pub cycle_number: i64,
    pub state_json: String,
    pub created_at: DateTime<Utc>,
}

/// Audit trail entry for permission-sensitive actions.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuditEntry {
    pub id: i64,
    pub agent_name: String,
    pub tool_name: String,
    pub action: String,
    pub approved: bool,
    pub approver: Option<String>,
    pub result: String,
    pub created_at: DateTime<Utc>,
}

/// Thread-safe SQLite state store.
#[derive(Clone)]
pub struct StateStore {
    conn: Arc<Mutex<Connection>>,
    db_path: PathBuf,
}

impl StateStore {
    /// Open or create a SQLite database at the given path.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(&path)?;
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA foreign_keys = ON;",
        )?;
        Self::init_schema_blocking(&conn)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            db_path: path,
        })
    }

    /// In-memory database for tests.
    pub fn in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        Self::init_schema_blocking(&conn)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            db_path: PathBuf::from(":memory:"),
        })
    }

    fn init_schema_blocking(conn: &Connection) -> Result<()> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS agent_state (
                agent_name TEXT PRIMARY KEY,
                status TEXT NOT NULL DEFAULT 'idle',
                last_cycle INTEGER DEFAULT 0,
                updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS agent_cycles (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                agent_name TEXT NOT NULL,
                cycle_number INTEGER NOT NULL,
                action TEXT NOT NULL,
                result TEXT NOT NULL,
                decision TEXT NOT NULL,
                tokens_consumed INTEGER DEFAULT 0,
                duration_ms INTEGER DEFAULT 0,
                error TEXT,
                created_at TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_cycles_agent
                ON agent_cycles(agent_name, cycle_number);

            CREATE TABLE IF NOT EXISTS checkpoints (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                agent_name TEXT NOT NULL,
                cycle_number INTEGER NOT NULL,
                state_json TEXT NOT NULL,
                created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS audit_trail (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                agent_name TEXT NOT NULL,
                tool_name TEXT NOT NULL,
                action TEXT NOT NULL,
                approved INTEGER NOT NULL DEFAULT 0,
                approver TEXT,
                result TEXT NOT NULL,
                created_at TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_audit_agent
                ON audit_trail(agent_name, created_at);
            ",
        )?;
        Ok(())
    }

    /// Get or set agent status.
    pub async fn get_agent_status(&self, agent_name: &str) -> StateResult<AgentStatus> {
        let conn = self.conn.lock().await;
        let row: Option<String> = conn
            .query_row(
                "SELECT status FROM agent_state WHERE agent_name = ?1",
                rusqlite::params![agent_name],
                |r| r.get(0),
            )
            .ok();
        Ok(row
            .map(|s| AgentStatus::parse(&s))
            .unwrap_or(AgentStatus::Idle))
    }

    pub async fn set_agent_status(
        &self,
        agent_name: &str,
        status: AgentStatus,
    ) -> StateResult<()> {
        let conn = self.conn.lock().await;
        conn.execute(
            "INSERT INTO agent_state (agent_name, status, updated_at)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(agent_name) DO UPDATE SET
                status = excluded.status,
                updated_at = excluded.updated_at",
            rusqlite::params![agent_name, status.to_string(), Utc::now().to_rfc3339()],
        )?;
        Ok(())
    }

    /// Record a completed cycle.
    pub async fn record_cycle(&self, cycle: &AgentCycle) -> StateResult<i64> {
        let conn = self.conn.lock().await;
        conn.execute(
            "INSERT INTO agent_cycles
                (agent_name, cycle_number, action, result, decision,
                 tokens_consumed, duration_ms, error, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            rusqlite::params![
                cycle.agent_name,
                cycle.cycle_number,
                cycle.action,
                cycle.result,
                cycle.decision,
                cycle.tokens_consumed,
                cycle.duration_ms,
                cycle.error,
                cycle.created_at.to_rfc3339(),
            ],
        )?;
        let id = conn.last_insert_rowid();
        // Upsert agent_state with last_cycle
        conn.execute(
            "INSERT INTO agent_state (agent_name, status, last_cycle, updated_at)
             VALUES (?1, 'idle', ?2, ?3)
             ON CONFLICT(agent_name) DO UPDATE SET
                last_cycle = excluded.last_cycle,
                updated_at = excluded.updated_at",
            rusqlite::params![cycle.agent_name, cycle.cycle_number, Utc::now().to_rfc3339()],
        )?;
        Ok(id)
    }

    /// Get the last cycle number for an agent.
    pub async fn get_last_cycle(&self, agent_name: &str) -> StateResult<i64> {
        let conn = self.conn.lock().await;
        let row: Option<i64> = conn
            .query_row(
                "SELECT last_cycle FROM agent_state WHERE agent_name = ?1",
                rusqlite::params![agent_name],
                |r| r.get(0),
            )
            .ok();
        Ok(row.unwrap_or(0))
    }

    /// Save a checkpoint.
    pub async fn save_checkpoint(&self, cp: &Checkpoint) -> StateResult<i64> {
        let conn = self.conn.lock().await;
        conn.execute(
            "INSERT INTO checkpoints (agent_name, cycle_number, state_json, created_at)
             VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![
                cp.agent_name,
                cp.cycle_number,
                cp.state_json,
                cp.created_at.to_rfc3339(),
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    /// Get the latest checkpoint for an agent.
    pub async fn get_latest_checkpoint(&self, agent_name: &str) -> StateResult<Option<Checkpoint>> {
        let conn = self.conn.lock().await;
        let row = conn.query_row(
            "SELECT id, agent_name, cycle_number, state_json, created_at
             FROM checkpoints
             WHERE agent_name = ?1
             ORDER BY id DESC LIMIT 1",
            rusqlite::params![agent_name],
            |r| {
                Ok(Checkpoint {
                    id: r.get(0)?,
                    agent_name: r.get(1)?,
                    cycle_number: r.get(2)?,
                    state_json: r.get(3)?,
                    created_at: DateTime::parse_from_rfc3339(&r.get::<_, String>(4)?)
                        .map_err(|e| rusqlite::Error::FromSqlConversionFailure(
                            4,
                            rusqlite::types::Type::Text,
                            Box::new(e),
                        ))?
                        .with_timezone(&Utc),
                })
            },
        );
        match row {
            Ok(cp) => Ok(Some(cp)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Record an audit entry.
    pub async fn record_audit(&self, entry: &AuditEntry) -> StateResult<i64> {
        let conn = self.conn.lock().await;
        conn.execute(
            "INSERT INTO audit_trail
                (agent_name, tool_name, action, approved, approver, result, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                entry.agent_name,
                entry.tool_name,
                entry.action,
                entry.approved as i32,
                entry.approver,
                entry.result,
                entry.created_at.to_rfc3339(),
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    /// List recent audit entries for an agent.
    pub async fn list_audit(&self, agent_name: &str, limit: i64) -> StateResult<Vec<AuditEntry>> {
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare(
            "SELECT id, agent_name, tool_name, action, approved, approver, result, created_at
             FROM audit_trail
             WHERE agent_name = ?1
             ORDER BY id DESC LIMIT ?2",
        )?;
        let entries = stmt
            .query_map(rusqlite::params![agent_name, limit], |r| {
                let created_str: String = r.get(7)?;
                Ok(AuditEntry {
                    id: r.get(0)?,
                    agent_name: r.get(1)?,
                    tool_name: r.get(2)?,
                    action: r.get(3)?,
                    approved: r.get::<_, i32>(4)? != 0,
                    approver: r.get(5)?,
                    result: r.get(6)?,
                    created_at: DateTime::parse_from_rfc3339(&created_str)
                        .map_err(|e| rusqlite::Error::FromSqlConversionFailure(
                            7,
                            rusqlite::types::Type::Text,
                            Box::new(e),
                        ))?
                        .with_timezone(&Utc),
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(entries)
    }

    /// List all agents and their statuses.
    pub async fn list_agents(&self) -> StateResult<Vec<(String, AgentStatus, i64)>> {
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare(
            "SELECT agent_name, status, last_cycle FROM agent_state ORDER BY agent_name",
        )?;
        let agents = stmt
            .query_map([], |r| {
                let name: String = r.get(0)?;
                let status: String = r.get(1)?;
                let last_cycle: i64 = r.get(2)?;
                Ok((name, AgentStatus::parse(&status), last_cycle))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(agents)
    }

    /// Get the database path.
    pub fn db_path(&self) -> &Path {
        &self.db_path
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_agent_status_lifecycle() {
        let store = StateStore::in_memory().unwrap();
        assert_eq!(
            store.get_agent_status("test-agent").await.unwrap(),
            AgentStatus::Idle
        );
        store
            .set_agent_status("test-agent", AgentStatus::Running)
            .await
            .unwrap();
        assert_eq!(
            store.get_agent_status("test-agent").await.unwrap(),
            AgentStatus::Running
        );
    }

    #[tokio::test]
    async fn test_record_and_get_cycle() {
        let store = StateStore::in_memory().unwrap();
        let cycle = AgentCycle {
            id: 0,
            agent_name: "test-agent".into(),
            cycle_number: 1,
            action: "post_social".into(),
            result: "posted successfully".into(),
            decision: "continue to next task".into(),
            tokens_consumed: 150,
            duration_ms: 3200,
            error: None,
            created_at: Utc::now(),
        };
        let id = store.record_cycle(&cycle).await.unwrap();
        assert!(id > 0);
        assert_eq!(store.get_last_cycle("test-agent").await.unwrap(), 1);
    }

    #[tokio::test]
    async fn test_checkpoint_save_and_load() {
        let store = StateStore::in_memory().unwrap();
        let cp = Checkpoint {
            id: 0,
            agent_name: "test-agent".into(),
            cycle_number: 5,
            state_json: r#"{"task":"post","step":3}"#.into(),
            created_at: Utc::now(),
        };
        store.save_checkpoint(&cp).await.unwrap();
        let loaded = store.get_latest_checkpoint("test-agent").await.unwrap();
        assert!(loaded.is_some());
        let loaded = loaded.unwrap();
        assert_eq!(loaded.cycle_number, 5);
        assert!(loaded.state_json.contains("post"));
    }

    #[tokio::test]
    async fn test_audit_trail() {
        let store = StateStore::in_memory().unwrap();
        let entry = AuditEntry {
            id: 0,
            agent_name: "social-media".into(),
            tool_name: "post_social".into(),
            action: "post to twitter".into(),
            approved: true,
            approver: Some("erik".into()),
            result: "success".into(),
            created_at: Utc::now(),
        };
        store.record_audit(&entry).await.unwrap();
        let entries = store.list_audit("social-media", 10).await.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].tool_name, "post_social");
        assert!(entries[0].approved);
    }

    #[tokio::test]
    async fn test_list_agents() {
        let store = StateStore::in_memory().unwrap();
        store
            .set_agent_status("agent-a", AgentStatus::Running)
            .await
            .unwrap();
        store
            .set_agent_status("agent-b", AgentStatus::Idle)
            .await
            .unwrap();
        let agents = store.list_agents().await.unwrap();
        assert_eq!(agents.len(), 2);
    }
}