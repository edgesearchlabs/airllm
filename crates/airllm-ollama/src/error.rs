use thiserror::Error;

/// All errors that can be returned by the Ollama client.
#[derive(Debug, Error)]
pub enum OllamaError {
    #[error("connection error: {0}")]
    Connection(String),

    #[error("HTTP error: {status} — {body}")]
    Http { status: u16, body: String },

    #[error("JSON serialization/deserialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("model '{0}' not found")]
    ModelNotFound(String),

    #[error("stream parse error: {0}")]
    StreamParse(String),

    #[error("request error: {0}")]
    Request(#[from] reqwest::Error),
}

pub type Result<T> = std::result::Result<T, OllamaError>;