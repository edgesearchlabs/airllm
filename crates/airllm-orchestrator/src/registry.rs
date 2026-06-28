use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use tracing::warn;

use crate::agent::Agent;
use crate::error::Result;
use crate::types::AgentConfig;

#[derive(Clone, Default)]
pub struct AgentRegistry {
    agents: HashMap<String, Agent>,
}

impl AgentRegistry {
    pub fn new() -> Self {
        Self {
            agents: HashMap::new(),
        }
    }

    pub fn register(&mut self, agent: Agent) {
        self.agents.insert(agent.name.clone(), agent);
    }

    pub fn get(&self, name: &str) -> Option<&Agent> {
        self.agents.get(name)
    }

    pub fn list(&self) -> Vec<&Agent> {
        let mut names: Vec<&String> = self.agents.keys().collect();
        names.sort();
        names
            .into_iter()
            .filter_map(|name| self.agents.get(name))
            .collect()
    }

    pub fn load_from_dir(dir: &Path) -> Result<Self> {
        let mut registry = Self::new();
        let mut entries: Vec<PathBuf> = fs::read_dir(dir)?
            .filter_map(|entry| entry.ok().map(|e| e.path()))
            .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("toml"))
            .collect();
        entries.sort();

        for path in entries {
            let raw = fs::read_to_string(&path)?;
            let config: AgentConfig = toml::from_str(&raw)?;
            let prompt_path = if Path::new(&config.system_prompt).is_absolute() {
                PathBuf::from(&config.system_prompt)
            } else {
                path.parent()
                    .unwrap_or(dir)
                    .join(&config.system_prompt)
            };
            let prompt = Agent::load_prompt(&prompt_path)?;
            registry.register(Agent::from_config(config, prompt));
        }

        Ok(registry)
    }

    pub fn builtin() -> Self {
        let mut registry = Self::new();
        for (name, model, fallback, prompt) in [
            (
                "architect",
                "qwen3-coder-next:q8_0",
                Some("granite4.1:30b"),
                "You are the AirLLM architect agent. Break complex work into clean, independent sub-tasks and return structured outputs.",
            ),
            (
                "coder",
                "qwen3.6:27b",
                Some("qwen3.5:4b"),
                "You are the AirLLM coder agent. Produce complete, executable code changes with minimal prose.",
            ),
            (
                "reviewer",
                "granite4.1:30b",
                Some("qwen3.6:27b"),
                "You are the AirLLM reviewer agent. Identify bugs, risks, regressions and concrete fixes.",
            ),
            (
                "tester",
                "qwen3.5:4b",
                Some("jaahas/crow:9b"),
                "You are the AirLLM tester agent. Write focused tests and validation guidance.",
            ),
            (
                "debugger",
                "nemotron-3-nano:30b",
                Some("qwen3.6:27b"),
                "You are the AirLLM debugger agent. Analyze failures and produce root-cause-driven fixes.",
            ),
            (
                "refactorer",
                "qwen3.6:27b",
                Some("granite4.1:30b"),
                "You are the AirLLM refactorer agent. Improve structure and maintainability without changing behavior.",
            ),
            (
                "documenter",
                "jaahas/crow:9b",
                Some("qwen3.5:4b"),
                "You are the AirLLM documenter agent. Produce concise, accurate documentation and usage notes.",
            ),
            (
                "planner",
                "granite4.1:30b",
                Some("qwen3.6:27b"),
                "You are the AirLLM planner agent. Turn broad requests into a small, prioritized execution plan with concrete sequencing.",
            ),
            (
                "security",
                "nemotron-3-nano:30b",
                Some("granite4.1:30b"),
                "You are the AirLLM security agent. Review code and architecture for vulnerabilities, attack surface, and unsafe defaults.",
            ),
            (
                "performance",
                "qwen3-coder-next:q8_0",
                Some("granite4.1:30b"),
                "You are the AirLLM performance agent. Optimize latency, throughput, memory usage, and concurrency without changing behavior.",
            ),
        ] {
            let config = AgentConfig {
                name: name.to_string(),
                default_model: model.to_string(),
                fallback_model: fallback.map(str::to_string),
                system_prompt: String::new(),
                parallelizable: !matches!(name, "reviewer" | "architect" | "debugger" | "planner" | "security"),
                max_concurrent: if name == "coder" { 3 } else if matches!(name, "tester" | "documenter") { 2 } else { 1 },
                temperature: if matches!(name, "architect" | "planner" | "security") { 0.1 } else if name == "documenter" { 0.3 } else { 0.2 },
                top_p: 0.9,
                routing_patterns: Vec::new(),
            };
            registry.register(Agent::from_config(config, prompt.to_string()));
        }
        registry
    }

    pub fn load_or_builtin(dir: &Path) -> Self {
        match Self::load_from_dir(dir) {
            Ok(registry) if !registry.agents.is_empty() => registry,
            Ok(_) => {
                warn!(path = %dir.display(), "agent directory was empty, using builtin registry");
                Self::builtin()
            }
            Err(err) => {
                warn!(path = %dir.display(), error = %err, "failed to load agent registry, using builtin registry");
                Self::builtin()
            }
        }
    }
}