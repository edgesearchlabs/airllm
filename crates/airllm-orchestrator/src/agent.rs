use std::fs;
use std::path::Path;

use airllm_ollama::{ChatOptions, Message, OllamaClient};

use crate::error::Result;
use crate::types::{AgentConfig, AgentResult, SubTask};

#[derive(Clone, Debug)]
pub struct Agent {
    pub name: String,
    pub model: String,
    pub system_prompt: String,
    pub parallelizable: bool,
    pub max_concurrent: usize,
    pub config: AgentConfig,
}

impl Agent {
    pub fn from_config(config: AgentConfig, system_prompt: String) -> Self {
        Self {
            name: config.name.clone(),
            model: config.default_model.clone(),
            system_prompt,
            parallelizable: config.parallelizable,
            max_concurrent: config.max_concurrent,
            config,
        }
    }

    pub fn load_prompt(path: &Path) -> Result<String> {
        Ok(fs::read_to_string(path)?)
    }

    pub async fn execute(&self, task: &SubTask, ollama: &OllamaClient) -> Result<AgentResult> {
        self.execute_with_model(task, ollama, None).await
    }

    pub async fn execute_with_model(
        &self,
        task: &SubTask,
        ollama: &OllamaClient,
        model_override: Option<&str>,
    ) -> Result<AgentResult> {
        let prompt = self.build_user_prompt(task);
        let messages = vec![Message::system(self.system_prompt.clone()), Message::user(prompt)];
        let output = ollama
            .chat(
                model_override.unwrap_or(&self.model),
                &messages,
                ChatOptions {
                    temperature: self.config.temperature,
                    top_p: self.config.top_p,
                    ..ChatOptions::default()
                },
            )
            .await?;

        let mut files = infer_files_from_output(&output);
        if files.is_empty() {
            files = task.input_files.clone();
        }

        Ok(AgentResult {
            task_id: task.id.clone(),
            output,
            files,
            success: true,
        })
    }

    fn build_user_prompt(&self, task: &SubTask) -> String {
        let mut prompt = String::new();
        prompt.push_str("Sub-task description:\n");
        prompt.push_str(&task.description);
        prompt.push_str("\n\n");

        if !task.input_files.is_empty() {
            prompt.push_str("Context files:\n");
            for path in &task.input_files {
                prompt.push_str(&format!("- {}\n", path));
                if let Ok(metadata) = fs::metadata(path) {
                    if metadata.is_file() {
                        match fs::read_to_string(path) {
                            Ok(content) => {
                                let trimmed = truncate_chars(&content, 8_000);
                                prompt.push_str(&format!(
                                    "```text\n# File: {}\n{}\n```\n",
                                    path, trimmed
                                ));
                            }
                            Err(err) => {
                                prompt.push_str(&format!(
                                    "Could not read file '{}': {}\n",
                                    path, err
                                ));
                            }
                        }
                    }
                }
            }
            prompt.push('\n');
        }

        prompt.push_str(
            "When you create or modify code, reference concrete paths and return complete code blocks. \
             Prefer concise, executable output over explanation.",
        );
        prompt
    }
}

fn truncate_chars(value: &str, max_chars: usize) -> String {
    let truncated: String = value.chars().take(max_chars).collect();
    if value.chars().count() > max_chars {
        format!("{}\n... [truncated]", truncated)
    } else {
        truncated
    }
}

fn infer_files_from_output(output: &str) -> Vec<String> {
    let mut files = Vec::new();
    for line in output.lines() {
        let trimmed = line.trim();
        if let Some(path) = trimmed.strip_prefix("File:") {
            files.push(path.trim().to_string());
        }
        if let Some(path) = trimmed.strip_prefix("Path:") {
            files.push(path.trim().to_string());
        }
        if trimmed.starts_with("```") && trimmed.len() > 3 {
            let info = trimmed.trim_start_matches('`').trim();
            if info.contains('/') || info.ends_with(".rs") || info.ends_with(".md") {
                files.push(info.to_string());
            }
        }
    }
    files.sort();
    files.dedup();
    files
}