//! RBAC permission engine for autonomous agents.
//!
//! Validates tool access, manages approval queue, and enforces rate limits.

use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::Mutex;

#[derive(Debug, Error)]
pub enum PermissionError {
    #[error("permission denied: agent {agent} cannot use tool {tool}")]
    Denied { agent: String, tool: String },
    #[error("approval required: agent {agent} action {action} pending approval")]
    ApprovalRequired { agent: String, action: String },
    #[error("rate limit exceeded: agent {agent} tool {tool} max {max_per_hour}/h")]
    RateLimit { agent: String, tool: String, max_per_hour: u32 },
    #[error("config error: {0}")]
    Config(String),
    #[error("toml parse error: {0}")]
    Toml(#[from] toml::de::Error),
}

pub type PermResult<T> = std::result::Result<T, PermissionError>;

/// A role defines a set of allowed tools and approval policy.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Role {
    pub tools: Vec<String>,
    pub approval_required: bool,
}

/// Per-agent permission overrides.
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct AgentOverrides {
    pub tools: Option<Vec<String>>,
    pub approval_required: Option<bool>,
    pub max_actions_per_hour: Option<u32>,
}

/// Full permissions configuration loaded from TOML.
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct PermissionsConfig {
    pub roles: HashMap<String, Role>,
    pub agents: HashMap<String, AgentAssignment>,
}

/// Maps an agent to a role with optional overrides.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AgentAssignment {
    pub role: String,
    #[serde(default)]
    pub overrides: AgentOverrides,
}

/// A pending approval entry.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PendingApproval {
    pub id: String,
    pub agent_name: String,
    pub tool_name: String,
    pub action: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Result of a permission check.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CheckResult {
    /// Action is allowed immediately.
    Allow,
    /// Action requires human approval first.
    RequireApproval(PendingApproval),
    /// Action is denied.
    Deny(String),
}

/// Rate limiter state per agent.
#[derive(Clone, Debug)]
struct RateLimitState {
    actions: Vec<Instant>,
    max_per_hour: u32,
}

/// Thread-safe permission engine.
#[derive(Clone)]
pub struct PermissionEngine {
    config: Arc<PermissionsConfig>,
    rate_limits: Arc<Mutex<HashMap<String, RateLimitState>>>,
    pending_approvals: Arc<Mutex<Vec<PendingApproval>>>,
}

impl PermissionEngine {
    /// Load permissions from a TOML file.
    pub fn load(path: impl AsRef<Path>) -> PermResult<Self> {
        let raw = std::fs::read_to_string(path.as_ref())
            .map_err(|e| PermissionError::Config(format!("read error: {e}")))?;
        Self::from_toml(&raw)
    }

    /// Parse permissions from TOML string.
    pub fn from_toml(raw: &str) -> PermResult<Self> {
        let config: PermissionsConfig = toml::from_str(raw)?;
        Ok(Self {
            config: Arc::new(config),
            rate_limits: Arc::new(Mutex::new(HashMap::new())),
            pending_approvals: Arc::new(Mutex::new(Vec::new())),
        })
    }

    /// Create an empty (deny-all) engine.
    pub fn empty() -> Self {
        Self {
            config: Arc::new(PermissionsConfig::default()),
            rate_limits: Arc::new(Mutex::new(HashMap::new())),
            pending_approvals: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Check if an agent can use a tool for a given action.
    pub async fn check(&self, agent_name: &str, tool_name: &str, action: &str) -> PermResult<CheckResult> {
        // 1. Find agent assignment
        let assignment = self.config.agents.get(agent_name).ok_or_else(|| {
            PermissionError::Denied {
                agent: agent_name.to_string(),
                tool: tool_name.to_string(),
            }
        })?;

        // 2. Resolve effective tools list
        let tools: HashSet<&str> = if let Some(ref tools) = assignment.overrides.tools {
            tools.iter().map(|s| s.as_str()).collect()
        } else if let Some(role) = self.config.roles.get(&assignment.role) {
            role.tools.iter().map(|s| s.as_str()).collect()
        } else {
            HashSet::new()
        };

        // 3. Check tool access (wildcard "*" allows all)
        if !tools.contains("*") && !tools.contains(tool_name) {
            return Ok(CheckResult::Deny(format!(
                "agent {agent_name} (role {}) does not have access to tool {tool_name}",
                assignment.role
            )));
        }

        // 4. Check rate limit
        let max_per_hour = assignment.overrides.max_actions_per_hour.unwrap_or(100);
        let mut limits = self.rate_limits.lock().await;
        let state = limits
            .entry(agent_name.to_string())
            .or_insert_with(|| RateLimitState {
                actions: Vec::new(),
                max_per_hour,
            });
        let now = Instant::now();
        // Prune actions older than 1 hour
        state
            .actions
            .retain(|t| now.duration_since(*t) < Duration::from_secs(3600));
        if state.actions.len() as u32 >= state.max_per_hour {
            return Err(PermissionError::RateLimit {
                agent: agent_name.to_string(),
                tool: tool_name.to_string(),
                max_per_hour: state.max_per_hour,
            });
        }

        // 5. Check approval requirement
        let needs_approval = assignment.overrides.approval_required
            .unwrap_or_else(|| {
                self.config
                    .roles
                    .get(&assignment.role)
                    .map(|r| r.approval_required)
                    .unwrap_or(false)
            });

        if needs_approval {
            let pending = PendingApproval {
                id: format!("{}_{}_{}", agent_name, tool_name, now.elapsed().as_nanos()),
                agent_name: agent_name.to_string(),
                tool_name: tool_name.to_string(),
                action: action.to_string(),
                created_at: chrono::Utc::now(),
            };
            self.pending_approvals.lock().await.push(pending.clone());
            return Ok(CheckResult::RequireApproval(pending));
        }

        // 6. Record action for rate limiting
        state.actions.push(now);
        Ok(CheckResult::Allow)
    }

    /// List pending approvals.
    pub async fn pending_approvals(&self) -> Vec<PendingApproval> {
        self.pending_approvals.lock().await.clone()
    }

    /// Approve a pending action by ID.
    pub async fn approve(&self, id: &str) -> PermResult<()> {
        let mut pending = self.pending_approvals.lock().await;
        let before = pending.len();
        pending.retain(|p| p.id != id);
        if pending.len() == before {
            return Err(PermissionError::Config(format!(
                "approval {id} not found"
            )));
        }
        Ok(())
    }

    /// Reject (remove) a pending action by ID.
    pub async fn reject(&self, id: &str) -> PermResult<()> {
        self.approve(id).await
    }

    /// Validate a config file without creating an engine.
    pub fn validate_config(path: impl AsRef<Path>) -> PermResult<()> {
        let raw = std::fs::read_to_string(path.as_ref())
            .map_err(|e| PermissionError::Config(format!("read error: {e}")))?;
        let config: PermissionsConfig = toml::from_str(&raw)?;
        // Check that all agent roles exist
        for (name, assignment) in &config.agents {
            if !config.roles.contains_key(&assignment.role) {
                return Err(PermissionError::Config(format!(
                    "agent {name} references unknown role: {}",
                    assignment.role
                )));
            }
        }
        Ok(())
    }

    /// Get the config (for inspection).
    pub fn config(&self) -> &PermissionsConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_CONFIG: &str = r#"
[roles.admin]
tools = ["*"]
approval_required = false

[roles.automated]
tools = ["post_social", "send_message", "webhook_call"]
approval_required = true

[roles.researcher]
tools = ["web_search", "web_fetch", "code", "list_models"]
approval_required = false

[agents.social-media]
role = "automated"
overrides = { tools = ["post_social", "webhook_call"], approval_required = false, max_actions_per_hour = 10 }

[agents.messaging]
role = "automated"

[agents.research]
role = "researcher"
overrides = { max_actions_per_hour = 50 }
"#;

    #[tokio::test]
    async fn test_allow_tool() {
        let engine = PermissionEngine::from_toml(TEST_CONFIG).unwrap();
        let result = engine
            .check("social-media", "post_social", "post to twitter")
            .await
            .unwrap();
        assert_eq!(result, CheckResult::Allow);
    }

    #[tokio::test]
    async fn test_deny_tool() {
        let engine = PermissionEngine::from_toml(TEST_CONFIG).unwrap();
        let result = engine.check("social-media", "send_email", "send email").await.unwrap();
        assert!(matches!(result, CheckResult::Deny(_)));
    }

    #[tokio::test]
    async fn test_require_approval() {
        let engine = PermissionEngine::from_toml(TEST_CONFIG).unwrap();
        let result = engine
            .check("messaging", "send_message", "send telegram")
            .await
            .unwrap();
        match result {
            CheckResult::RequireApproval(p) => {
                assert_eq!(p.agent_name, "messaging");
                assert_eq!(p.tool_name, "send_message");
            }
            _ => panic!("expected RequireApproval"),
        }
    }

    #[tokio::test]
    async fn test_rate_limit() {
        let engine = PermissionEngine::from_toml(TEST_CONFIG).unwrap();
        // social-media has max 10/hour
        for _ in 0..10 {
            engine
                .check("social-media", "post_social", "post")
                .await
                .unwrap();
        }
        let result = engine.check("social-media", "post_social", "post 11").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_wildcard_role() {
        // admin role has "*" — but no agent is assigned admin in test config
        // Add an admin agent
        let config = format!(
            "{TEST_CONFIG}\n[agents.admin-bot]\nrole = \"admin\"\n"
        );
        let engine = PermissionEngine::from_toml(&config).unwrap();
        let result = engine
            .check("admin-bot", "any_tool", "anything")
            .await
            .unwrap();
        assert_eq!(result, CheckResult::Allow);
    }

    #[tokio::test]
    async fn test_approve_pending() {
        let engine = PermissionEngine::from_toml(TEST_CONFIG).unwrap();
        let result = engine
            .check("messaging", "send_message", "send")
            .await
            .unwrap();
        let id = match result {
            CheckResult::RequireApproval(p) => p.id,
            _ => panic!("expected approval"),
        };
        engine.approve(&id).await.unwrap();
        assert!(engine.pending_approvals().await.is_empty());
    }

    #[tokio::test]
    async fn test_unknown_agent_denied() {
        let engine = PermissionEngine::from_toml(TEST_CONFIG).unwrap();
        let result = engine.check("unknown", "code", "test").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_validate_config() {
        let tmp = std::env::temp_dir().join("test_perms.toml");
        std::fs::write(&tmp, TEST_CONFIG).unwrap();
        assert!(PermissionEngine::validate_config(&tmp).is_ok());
    }
}