use std::fmt;

use serde::{Deserialize, Serialize};

/// Role of a chat message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    System,
    User,
    Assistant,
}

impl fmt::Display for MessageRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MessageRole::System => write!(f, "system"),
            MessageRole::User => write!(f, "user"),
            MessageRole::Assistant => write!(f, "assistant"),
        }
    }
}

/// A single chat message sent to / received from Ollama.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: MessageRole,
    pub content: String,
    /// Tool calls made by the assistant (for function calling).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<serde_json::Value>>,
}

impl Message {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::System,
            content: content.into(),
            tool_calls: None,
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::User,
            content: content.into(),
            tool_calls: None,
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: content.into(),
            tool_calls: None,
        }
    }
}

/// Generation options passed to the Ollama chat endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatOptions {
    pub temperature: f32,
    pub top_p: f32,
    pub top_k: u32,
    pub num_ctx: u32,
}

impl Default for ChatOptions {
    fn default() -> Self {
        Self {
            temperature: 0.7,
            top_p: 0.9,
            top_k: 40,
            num_ctx: 4096,
        }
    }
}

/// Information about a model available in the Ollama instance.
#[derive(Debug, Clone, Deserialize)]
pub struct ModelInfo {
    pub name: String,
    pub size: String,
    pub quantization: String,
}

/// Metrics collected during a chat request.
#[derive(Debug, Clone, Default)]
pub struct ChatMetrics {
    /// Wall-clock time from request start to response received (ms).
    pub latency_ms: u64,
    /// Number of tokens in the response (approximate: words/whitespace).
    pub output_tokens: u64,
    /// Number of tokens in the input messages (approximate).
    pub input_tokens: u64,
    /// Tokens per second (output_tokens / latency_s).
    pub tokens_per_second: f64,
    /// Model used.
    pub model: String,
    /// Context window configured.
    pub num_ctx: u32,
    /// Temperature used.
    pub temperature: f32,
    /// Top-p used.
    pub top_p: f32,
}

impl ChatMetrics {
    /// Estimate token count from text (rough: 1 token ≈ 4 chars).
    pub fn estimate_tokens(text: &str) -> u64 {
        (text.len() as f64 / 4.0).ceil() as u64
    }

    /// Build metrics from a completed request.
    pub fn from_request(
        model: &str,
        messages: &[Message],
        options: &ChatOptions,
        latency_ms: u64,
        output: &str,
    ) -> Self {
        let input_text: String = messages.iter().map(|m| m.content.as_str()).collect::<Vec<_>>().join(" ");
        let input_tokens = Self::estimate_tokens(&input_text);
        let output_tokens = Self::estimate_tokens(output);
        let latency_s = latency_ms as f64 / 1000.0;
        let tps = if latency_s > 0.0 {
            output_tokens as f64 / latency_s
        } else {
            0.0
        };
        Self {
            latency_ms,
            output_tokens,
            input_tokens,
            tokens_per_second: tps,
            model: model.to_string(),
            num_ctx: options.num_ctx,
            temperature: options.temperature,
            top_p: options.top_p,
        }
    }
}

/// Complexity level used by the model router to select the appropriate model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Complexity {
    Low,
    Medium,
    High,
    Cloud,
}

impl fmt::Display for Complexity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Complexity::Low => write!(f, "low"),
            Complexity::Medium => write!(f, "medium"),
            Complexity::High => write!(f, "high"),
            Complexity::Cloud => write!(f, "cloud"),
        }
    }
}