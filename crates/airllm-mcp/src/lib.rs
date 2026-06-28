//! Minimal stdio MCP server exposing AirLLM tools.

mod error;
mod server;
mod tools;

pub use crate::error::McpError;
pub use crate::server::run_stdio;
pub use crate::tools::{available_tools, ToolDefinition};