//! # airllm-ollama
//!
//! Async Ollama HTTP client and model router for AirLLM v3.0.
//!
//! ## Quick start
//!
//! ```no_run
//! use airllm_ollama::{OllamaClient, ModelRouter, Message, ChatOptions};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = OllamaClient::new("http://localhost:11434");
//!
//! let messages = vec![
//!     Message::system("You are a helpful coding assistant."),
//!     Message::user("Write a hello world in Rust."),
//! ];
//!
//! let response = client.chat("qwen3.6:27b", &messages, ChatOptions::default()).await?;
//! println!("{response}");
//!
//! let router = ModelRouter::new();
//! let model = router.route("implement a REST API endpoint");
//! println!("Selected model: {model}");
//! # Ok(())
//! # }
//! ```

mod client;
mod error;
mod router;
mod stream;
mod types;

pub use client::{OllamaClient, StreamEvent};
pub use error::{OllamaError, Result};
pub use router::ModelRouter;
pub use stream::chat_stream;
pub use types::{ChatMetrics, ChatOptions, Complexity, Message, MessageRole, ModelInfo};