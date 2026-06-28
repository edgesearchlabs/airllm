//! Tool registry and implementations for autonomous agents.
//!
//! Every tool implements the `Tool` trait and is registered in `ToolRegistry`.
//! Tools are exposed via MCP and gated by the permission engine.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ToolError {
    #[error("tool not found: {0}")]
    NotFound(String),
    #[error("invalid arguments: {0}")]
    InvalidArgs(String),
    #[error("execution error: {0}")]
    Execution(String),
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),
}

pub type ToolResult<T> = std::result::Result<T, ToolError>;

/// Input schema for a tool (JSON Schema).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolSchema {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

/// Core trait for all agent tools.
#[async_trait]
pub trait Tool: Send + Sync {
    /// Tool name (unique identifier).
    fn name(&self) -> &str;

    /// Human-readable description.
    fn description(&self) -> &str;

    /// JSON Schema for input arguments.
    fn input_schema(&self) -> Value;

    /// Execute the tool with the given arguments.
    async fn execute(&self, args: Value) -> ToolResult<Value>;
}

/// Registry of available tools.
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    /// Register a tool.
    pub fn register(&mut self, tool: Arc<dyn Tool>) {
        self.tools.insert(tool.name().to_string(), tool);
    }

    /// Get a tool by name.
    pub fn get(&self, name: &str) -> Option<&Arc<dyn Tool>> {
        self.tools.get(name)
    }

    /// List all registered tool schemas.
    pub fn list_schemas(&self) -> Vec<ToolSchema> {
        self.tools
            .values()
            .map(|t| ToolSchema {
                name: t.name().to_string(),
                description: t.description().to_string(),
                input_schema: t.input_schema(),
            })
            .collect()
    }

    /// Execute a tool by name.
    pub async fn execute(&self, name: &str, args: Value) -> ToolResult<Value> {
        let tool = self
            .tools
            .get(name)
            .ok_or_else(|| ToolError::NotFound(name.to_string()))?;
        tool.execute(args).await
    }

    /// List all tool names.
    pub fn tool_names(&self) -> Vec<String> {
        self.tools.keys().cloned().collect()
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ── Built-in tools ──────────────────────────────────────────────────────

pub mod webhook;
pub mod social_media;
pub mod messaging;
pub mod email;

/// Create a registry with all built-in tools.
pub fn default_registry() -> ToolRegistry {
    let mut reg = ToolRegistry::new();
    reg.register(Arc::new(webhook::WebhookTool::new()));
    reg.register(Arc::new(social_media::SocialMediaTool::new()));
    reg.register(Arc::new(messaging::MessagingTool::new()));
    reg.register(Arc::new(email::EmailTool::new()));
    reg
}

#[cfg(test)]
mod tests {
    use super::*;

    struct EchoTool;

    #[async_trait]
    impl Tool for EchoTool {
        fn name(&self) -> &str {
            "echo"
        }
        fn description(&self) -> &str {
            "Echoes the input back"
        }
        fn input_schema(&self) -> Value {
            serde_json::json!({
                "type": "object",
                "properties": {"message": {"type": "string"}},
                "required": ["message"]
            })
        }
        async fn execute(&self, args: Value) -> ToolResult<Value> {
            let msg = args
                .get("message")
                .and_then(Value::as_str)
                .ok_or_else(|| ToolError::InvalidArgs("message is required".into()))?;
            Ok(serde_json::json!({"echoed": msg}))
        }
    }

    #[tokio::test]
    async fn test_registry_register_and_execute() {
        let mut reg = ToolRegistry::new();
        reg.register(Arc::new(EchoTool));
        let result = reg
            .execute("echo", serde_json::json!({"message": "hello"}))
            .await
            .unwrap();
        assert_eq!(result["echoed"], "hello");
    }

    #[tokio::test]
    async fn test_registry_not_found() {
        let reg = ToolRegistry::new();
        let result = reg.execute("nonexistent", serde_json::json!({})).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_default_registry_has_tools() {
        let reg = default_registry();
        let names = reg.tool_names();
        assert!(names.contains(&"webhook_call".to_string()));
        assert!(names.contains(&"post_social".to_string()));
        assert!(names.contains(&"send_message".to_string()));
        assert!(names.contains(&"send_email".to_string()));
    }
}