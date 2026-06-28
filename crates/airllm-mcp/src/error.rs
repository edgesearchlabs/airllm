use thiserror::Error;

#[derive(Debug, Error)]
pub enum McpError {
    #[error("tool not found: {0}")]
    ToolNotFound(String),
    #[error("invalid arguments: {0}")]
    InvalidArgs(String),
    #[error("orchestrator: {0}")]
    Orchestrator(String),
    #[error("io: {0}")]
    Io(String),
}
