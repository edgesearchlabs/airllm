//! OpenAI-compatible HTTP bridge for AirLLM.
//!
//! Exposes endpoints that the OpenAirLLM frontend (OpenClaude fork) can talk to
//! as if it were an OpenAI/Ollama API server. Internally delegates to our Rust
//! orchestrator with tool calling, permissions, and structured system prompts.

mod server;
mod types;

pub use server::BridgeServer;
pub use types::{ChatCompletionRequest, ChatCompletionResponse, ModelInfo};