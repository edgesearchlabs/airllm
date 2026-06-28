use thiserror::Error;

pub type Result<T> = std::result::Result<T, OrchestratorError>;

#[derive(Debug, Error)]
pub enum OrchestratorError {
    #[error("ollama error: {0}")]
    Ollama(#[from] airllm_ollama::OllamaError),

    #[error("i/o error: {0}")]
    Io(#[from] std::io::Error),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("toml error: {0}")]
    Toml(#[from] toml::de::Error),

    #[error("missing agent '{0}'")]
    MissingAgent(String),

    #[error("invalid request: {0}")]
    InvalidRequest(String),

    #[error("invalid response: {0}")]
    InvalidResponse(String),

    #[error("join error: {0}")]
    Join(String),
}