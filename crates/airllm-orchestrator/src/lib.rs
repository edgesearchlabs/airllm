//! Multi-agent coding orchestrator for AirLLM v3.0.

mod agent;
mod consolidate;
mod decompose;
mod error;
mod orchestrator;
mod registry;
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