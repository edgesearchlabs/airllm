use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use airllm_ollama::{ChatOptions, Complexity, Message, ModelRouter, OllamaClient};
use async_trait::async_trait;
use futures::{stream, Stream};
use tokio::sync::Semaphore;
use tracing::{info, warn};

use crate::agent::Agent;
use crate::consolidate::consolidate_results;
use crate::decompose::decompose_request;
use crate::error::{OrchestratorError, Result};
use crate::registry::AgentRegistry;
use crate::types::{
    AgentResult, CodeRequest, CodeResponse, RefactorResponse, ReviewResponse,
    SubTask, TestResponse,
};

pub struct Orchestrator {
    ollama: OllamaClient,
    router: ModelRouter,
    agents: Arc<AgentRegistry>,
}

impl Clone for Orchestrator {
    fn clone(&self) -> Self {
        Self {
            ollama: self.ollama.clone(),
            router: ModelRouter::new(),
            agents: self.agents.clone(),
        }
    }
}

impl Orchestrator {
    pub fn new(ollama: OllamaClient) -> Self {
        let agents_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("agents");
        let registry = AgentRegistry::load_or_builtin(&agents_dir);
        Self {
            ollama,
            router: ModelRouter::new(),
            agents: Arc::new(registry),
        }
    }

    pub fn router(&self) -> &ModelRouter {
        &self.router
    }

    pub fn agents(&self) -> Arc<AgentRegistry> {
        self.agents.clone()
    }

    pub async fn code(&self, request: CodeRequest) -> Result<CodeResponse> {
        // Fast path: for short tasks, skip decomposition and go direct to model
        if request.task.len() < 200 && !request.task.contains("architect") && !request.task.contains("design") {
            return self.code_fast(request).await;
        }
        self.code_full(request).await
    }

    /// Fast code generation — single model call, no decomposition.
    pub async fn code_fast(&self, request: CodeRequest) -> Result<CodeResponse> {
        if request.task.trim().is_empty() {
            return Err(OrchestratorError::InvalidRequest(
                "code task cannot be empty".to_string(),
            ));
        }

        // Skip decomposition — go direct to coder agent with selected model
        let agent = self
            .agents
            .get("coder")
            .ok_or_else(|| OrchestratorError::MissingAgent("coder".into()))?;

        let model = if let Some(ref m) = request.model_override {
            m.clone()
        } else {
            self.resolve_agent_model(agent, request.model_override.as_deref()).await
        };

        let subtask = SubTask {
            id: "fast-1".into(),
            description: request.task.clone(),
            agent_name: "coder".into(),
            input_files: request.files.clone(),
        };

        let result = agent
            .execute_with_model(&subtask, &self.ollama, Some(&model))
            .await?;

        Ok(CodeResponse {
            output: result.output,
            files_written: result.files,
            agent_used: agent.name.clone(),
            model_used: model,
        })
    }

    /// Full code generation with decomposition and parallel execution.
    pub async fn code_full(&self, request: CodeRequest) -> Result<CodeResponse> {
        if request.task.trim().is_empty() {
            return Err(OrchestratorError::InvalidRequest(
                "code task cannot be empty".to_string(),
            ));
        }

        let complexity = self.router.classify(&request.task);
        let subtasks = self.prepare_subtasks(&request, complexity).await?;

        if subtasks.len() == 1 {
            let subtask = subtasks.into_iter().next().expect("single subtask");
            let agent = self
                .agents
                .get(&subtask.agent_name)
                .or_else(|| self.agents.get("coder"))
                .ok_or_else(|| OrchestratorError::MissingAgent(subtask.agent_name.clone()))?;
            let model = self.resolve_agent_model(agent, request.model_override.as_deref()).await;
            let result = agent
                .execute_with_model(&subtask, &self.ollama, Some(&model))
                .await?;
            return Ok(CodeResponse {
                output: result.output,
                files_written: result.files,
                agent_used: agent.name.clone(),
                model_used: model,
            });
        }

        let results = self
            .execute_parallel(subtasks, request.model_override.as_deref())
            .await?;

        let reviewer = self.agents.get("reviewer");
        let reviewer_model = match reviewer {
            Some(agent) => Some(self.resolve_agent_model(agent, None).await),
            None => None,
        };
        consolidate_results(results, reviewer, reviewer_model.as_deref(), &self.ollama).await
    }

    pub async fn prewarm_models(&self, models: Option<Vec<String>>) -> Result<Vec<String>> {
        let requested = match models {
            Some(models) if !models.is_empty() => models,
            _ => {
                let mut defaults: Vec<String> = self
                    .agents
                    .list()
                    .into_iter()
                    .map(|agent| agent.model.clone())
                    .collect();
                defaults.sort();
                defaults.dedup();
                defaults
            }
        };

        let available = self.list_models().await?;
        let mut warmed = Vec::new();
        for model in requested {
            if available.iter().any(|candidate| candidate == &model) {
                self.ollama.prewarm_model(&model).await?;
                warmed.push(model);
            }
        }

        Ok(warmed)
    }

    pub async fn code_stream(
        &self,
        request: CodeRequest,
    ) -> Result<impl Stream<Item = Result<String>>> {
        let response = self.code(request).await?;
        let tokens: Vec<String> = response
            .output
            .split_whitespace()
            .map(|token| format!("{} ", token))
            .collect();
        Ok(stream::iter(tokens.into_iter().map(Ok)))
    }

    pub async fn review(&self, files: Vec<String>) -> Result<ReviewResponse> {
        let output = self
            .run_single_agent_task(
                "reviewer",
                "Review the provided files. Focus on bugs, regressions, security, and missing tests.",
                files,
                None,
            )
            .await?;
        Ok(ReviewResponse { output })
    }

    pub async fn test(&self, files: Vec<String>, framework: Option<String>) -> Result<TestResponse> {
        let description = format!(
            "Write or suggest focused tests for these files. Preferred framework: {}.",
            framework.unwrap_or_else(|| "auto".to_string())
        );
        let output = self
            .run_single_agent_task("tester", &description, files, None)
            .await?;
        Ok(TestResponse { output })
    }

    pub async fn refactor(&self, files: Vec<String>, goal: &str) -> Result<RefactorResponse> {
        let description = format!(
            "Refactor the provided files without changing behavior. Goal: {}.",
            goal
        );
        let output = self
            .run_single_agent_task("refactorer", &description, files, None)
            .await?;
        Ok(RefactorResponse { output })
    }

    pub async fn chat(&self, prompt: &str, model: Option<&str>) -> Result<String> {
        let complexity = self.router.classify(prompt);
        let preferred = model
            .map(str::to_string)
            .unwrap_or_else(|| self.router.select_model(&complexity).to_string());
        let resolved = self.resolve_model_candidates(&preferred, None).await;
        info!(task = %prompt, model = %resolved, complexity = %complexity, "chat request");
        let messages = vec![
            Message::system("You are AirLLM, a multi-agent coding assistant powered by local Ollama models."),
            Message::user(prompt),
        ];
        self.ollama
            .chat(&resolved, &messages, ChatOptions::default())
            .await
            .map_err(Into::into)
    }

    pub async fn list_models(&self) -> Result<Vec<String>> {
        Ok(self
            .ollama
            .list_models()
            .await?
            .into_iter()
            .map(|model| model.name)
            .collect())
    }

    async fn prepare_subtasks(
        &self,
        request: &CodeRequest,
        complexity: Complexity,
    ) -> Result<Vec<SubTask>> {
        let needs_decomposition = matches!(complexity, Complexity::High | Complexity::Cloud)
            || request.files.len() > 1
            || request.task.len() > 160;
        if !needs_decomposition {
            return Ok(vec![SubTask {
                id: "task-1".to_string(),
                description: request.task.clone(),
                agent_name: inferred_agent_for_task(&request.task).to_string(),
                input_files: request.files.clone(),
            }]);
        }

        let architect = self
            .agents
            .get("architect")
            .ok_or_else(|| OrchestratorError::MissingAgent("architect".to_string()))?;
        let model = self.resolve_agent_model(architect, None).await;
        decompose_request(request, architect, &model, &self.ollama).await
    }

    async fn execute_parallel(
        &self,
        subtasks: Vec<SubTask>,
        model_override: Option<&str>,
    ) -> Result<Vec<AgentResult>> {
        let semaphores: HashMap<String, Arc<Semaphore>> = self
            .agents
            .list()
            .into_iter()
            .map(|agent| {
                (
                    agent.name.clone(),
                    Arc::new(Semaphore::new(agent.max_concurrent.max(1))),
                )
            })
            .collect();

        let mut handles = Vec::new();
        for subtask in subtasks {
            let agent = self
                .agents
                .get(&subtask.agent_name)
                .or_else(|| self.agents.get("coder"))
                .ok_or_else(|| OrchestratorError::MissingAgent(subtask.agent_name.clone()))?
                .clone();
            let ollama = self.ollama.clone();
            let semaphore = semaphores
                .get(&agent.name)
                .cloned()
                .unwrap_or_else(|| Arc::new(Semaphore::new(1)));
            let model = self.resolve_agent_model(&agent, model_override).await;
            handles.push(tokio::spawn(async move {
                let _permit = semaphore
                    .acquire_owned()
                    .await
                    .map_err(|err| OrchestratorError::Join(err.to_string()))?;
                agent.execute_with_model(&subtask, &ollama, Some(&model)).await
            }));
        }

        let mut results = Vec::new();
        for handle in handles {
            let result = handle
                .await
                .map_err(|err| OrchestratorError::Join(err.to_string()))??;
            results.push(result);
        }
        Ok(results)
    }

    async fn run_single_agent_task(
        &self,
        agent_name: &str,
        description: &str,
        files: Vec<String>,
        model_override: Option<&str>,
    ) -> Result<String> {
        let agent = self
            .agents
            .get(agent_name)
            .ok_or_else(|| OrchestratorError::MissingAgent(agent_name.to_string()))?;
        let model = self.resolve_agent_model(agent, model_override).await;
        let task = SubTask {
            id: format!("{}-1", agent_name),
            description: description.to_string(),
            agent_name: agent_name.to_string(),
            input_files: files,
        };
        Ok(agent.execute_with_model(&task, &self.ollama, Some(&model)).await?.output)
    }

    async fn resolve_agent_model(&self, agent: &Agent, requested: Option<&str>) -> String {
        self.resolve_model_candidates(requested.unwrap_or(&agent.model), agent.config.fallback_model.as_deref()).await
    }

    async fn resolve_model_candidates(
        &self,
        preferred: &str,
        fallback: Option<&str>,
    ) -> String {
        let available = match self.list_models().await {
            Ok(models) => models,
            Err(err) => {
                warn!(error = %err, model = %preferred, "could not query Ollama models, using preferred model directly");
                return preferred.to_string();
            }
        };

        let mut candidates = vec![preferred.to_string()];
        if let Some(fallback) = fallback {
            candidates.push(fallback.to_string());
        }
        candidates.extend([
            "qwen3-coder-next:q8_0".to_string(),
            "qwen3.6:27b".to_string(),
            "qwen3.5:4b".to_string(),
        ]);

        for candidate in candidates {
            if available.iter().any(|name| name == &candidate) {
                return candidate;
            }
        }

        available.into_iter().next().unwrap_or_else(|| preferred.to_string())
    }
}

fn inferred_agent_for_task(task: &str) -> &'static str {
    let lowered = task.to_ascii_lowercase();
    if lowered.contains("security") || lowered.contains("vulnerab") || lowered.contains("owasp") {
        "security"
    } else if lowered.contains("perf") || lowered.contains("latency") || lowered.contains("throughput") || lowered.contains("optimiz") {
        "performance"
    } else if lowered.contains("plan") || lowered.contains("roadmap") || lowered.contains("strategy") {
        "planner"
    } else if lowered.contains("test") {
        "tester"
    } else if lowered.contains("review") {
        "reviewer"
    } else if lowered.contains("refactor") {
        "refactorer"
    } else if lowered.contains("debug") || lowered.contains("fix") {
        "debugger"
    } else if lowered.contains("document") || lowered.contains("readme") {
        "documenter"
    } else {
        "coder"
    }
}

#[async_trait]
pub trait OrchestratorLike {
    async fn code(&self, request: CodeRequest) -> Result<CodeResponse>;
    async fn review(&self, files: Vec<String>) -> Result<ReviewResponse>;
    async fn test(&self, files: Vec<String>, framework: Option<String>) -> Result<TestResponse>;
    async fn refactor(&self, files: Vec<String>, goal: &str) -> Result<RefactorResponse>;
    async fn chat(&self, prompt: &str, model: Option<&str>) -> Result<String>;
    async fn list_models(&self) -> Result<Vec<String>>;
    async fn prewarm_models(&self, models: Option<Vec<String>>) -> Result<Vec<String>>;
}

#[async_trait]
impl OrchestratorLike for Orchestrator {
    async fn code(&self, request: CodeRequest) -> Result<CodeResponse> {
        Orchestrator::code(self, request).await
    }

    async fn review(&self, files: Vec<String>) -> Result<ReviewResponse> {
        Orchestrator::review(self, files).await
    }

    async fn test(&self, files: Vec<String>, framework: Option<String>) -> Result<TestResponse> {
        Orchestrator::test(self, files, framework).await
    }

    async fn refactor(&self, files: Vec<String>, goal: &str) -> Result<RefactorResponse> {
        Orchestrator::refactor(self, files, goal).await
    }

    async fn chat(&self, prompt: &str, model: Option<&str>) -> Result<String> {
        Orchestrator::chat(self, prompt, model).await
    }

    async fn list_models(&self) -> Result<Vec<String>> {
        Orchestrator::list_models(self).await
    }

    async fn prewarm_models(&self, models: Option<Vec<String>>) -> Result<Vec<String>> {
        Orchestrator::prewarm_models(self, models).await
    }
}