//! Multi-agent coding orchestrator for AirLLM v3.0.

mod agent;
mod consolidate;
mod decompose;
mod error;
mod orchestrator;
mod permissions;
mod prompt_parser;
mod registry;
mod system_prompt;
mod tools;
mod types;

pub use agent::Agent;
pub use consolidate::consolidate_results;
pub use decompose::decompose_request;
pub use error::{OrchestratorError, Result};
pub use orchestrator::{Orchestrator, OrchestratorLike};
pub use registry::AgentRegistry;
pub use types::{
    AgentConfig, AgentResult, CodeRequest, CodeResponse, RefactorRequest,
    RefactorResponse, ReviewRequest, ReviewResponse, SubTask, TestRequest,
    TestResponse,
};
pub use tools::{
    execute_tool, expand_path, extract_visible_text, has_tool_calls,
    parse_tool_calls, tool_instructions, ToolCall, ToolResult,
};
pub use permissions::{check_permission, is_dangerous_path, PermissionAction, PermissionDecision, PermissionMode};
pub use prompt_parser::{extension_for_language, language_from_extension, parse_prompt, ParsedIntent};
pub use system_prompt::{build_chat_prompt, build_system_prompt, for_agent as system_prompt_for_agent};