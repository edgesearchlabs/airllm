use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CodeRequest {
    pub task: String,
    pub language: Option<String>,
    pub files: Vec<String>,
    pub model_override: Option<String>,
    /// Permission mode for tool execution. Default: prompt for each action.
    #[serde(default)]
    pub permission_mode: String,
    /// Maximum number of tool-call rounds before giving up.
    #[serde(default = "default_max_rounds")]
    pub max_rounds: u32,
}

fn default_max_rounds() -> u32 {
    5
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CodeResponse {
    pub output: String,
    pub files_written: Vec<String>,
    pub agent_used: String,
    pub model_used: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ReviewRequest {
    pub files: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReviewResponse {
    pub output: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TestRequest {
    pub files: Vec<String>,
    pub framework: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TestResponse {
    pub output: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct RefactorRequest {
    pub files: Vec<String>,
    pub goal: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RefactorResponse {
    pub output: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SubTask {
    pub id: String,
    pub description: String,
    pub agent_name: String,
    pub input_files: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AgentResult {
    pub task_id: String,
    pub output: String,
    pub files: Vec<String>,
    pub success: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AgentConfig {
    pub name: String,
    pub default_model: String,
    #[serde(default)]
    pub fallback_model: Option<String>,
    pub system_prompt: String,
    #[serde(default = "default_parallelizable")]
    pub parallelizable: bool,
    #[serde(default = "default_max_concurrent")]
    pub max_concurrent: usize,
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    #[serde(default = "default_top_p")]
    pub top_p: f32,
    #[serde(default)]
    pub routing_patterns: Vec<String>,
}

impl AgentConfig {
    pub fn preferred_model(&self) -> &str {
        &self.default_model
    }
}

fn default_parallelizable() -> bool {
    true
}

fn default_max_concurrent() -> usize {
    2
}

fn default_temperature() -> f32 {
    0.2
}

fn default_top_p() -> f32 {
    0.9
}