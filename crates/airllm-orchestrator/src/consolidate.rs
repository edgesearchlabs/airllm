use std::collections::{HashMap, HashSet};

use airllm_ollama::{ChatOptions, Message, OllamaClient};

use crate::agent::Agent;
use crate::error::Result;
use crate::types::{AgentResult, CodeResponse};

pub async fn consolidate_results(
    results: Vec<AgentResult>,
    reviewer: Option<&Agent>,
    reviewer_model: Option<&str>,
    ollama: &OllamaClient,
) -> Result<CodeResponse> {
    if results.len() == 1 {
        let result = results.into_iter().next().expect("single result");
        return Ok(CodeResponse {
            output: result.output,
            files_written: result.files,
            agent_used: "single-agent".to_string(),
            model_used: reviewer_model.unwrap_or("direct").to_string(),
        });
    }

    let files_written = merged_files(&results);
    let conflicts = detect_conflicts(&results);
    let output = if conflicts.is_empty() {
        merge_outputs(&results)
    } else if let Some(reviewer) = reviewer {
        let prompt = build_conflict_prompt(&results, &conflicts);
        let messages = vec![
            Message::system(reviewer.system_prompt.clone()),
            Message::user(prompt),
        ];
        match ollama
            .chat(
                reviewer_model.unwrap_or(&reviewer.model),
                &messages,
                ChatOptions {
                    temperature: reviewer.config.temperature,
                    top_p: reviewer.config.top_p,
                    ..ChatOptions::default()
                },
            )
            .await
        {
            Ok(value) => value,
            Err(_) => merge_outputs(&results),
        }
    } else {
        merge_outputs(&results)
    };

    let agent_used = if conflicts.is_empty() {
        "multi-agent"
    } else {
        "reviewer"
    };

    Ok(CodeResponse {
        output,
        files_written,
        agent_used: agent_used.to_string(),
        model_used: reviewer_model.unwrap_or("multi-agent").to_string(),
    })
}

fn merged_files(results: &[AgentResult]) -> Vec<String> {
    let mut all = HashSet::new();
    for result in results {
        for file in &result.files {
            all.insert(file.clone());
        }
    }
    let mut files: Vec<String> = all.into_iter().collect();
    files.sort();
    files
}

fn detect_conflicts(results: &[AgentResult]) -> Vec<String> {
    let mut counts: HashMap<String, usize> = HashMap::new();
    for result in results {
        for file in &result.files {
            *counts.entry(file.clone()).or_insert(0) += 1;
        }
    }
    let mut conflicts: Vec<String> = counts
        .into_iter()
        .filter_map(|(file, count)| if count > 1 { Some(file) } else { None })
        .collect();
    conflicts.sort();
    conflicts
}

fn merge_outputs(results: &[AgentResult]) -> String {
    results
        .iter()
        .map(|result| format!("## {}\n{}", result.task_id, result.output.trim()))
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn build_conflict_prompt(results: &[AgentResult], conflicts: &[String]) -> String {
    let mut prompt = String::new();
    prompt.push_str("Consolidate these multi-agent code results into one coherent answer.\n");
    prompt.push_str(&format!("Conflicting files: {}\n\n", conflicts.join(", ")));
    for result in results {
        prompt.push_str(&format!("Task {}:\n{}\n\n", result.task_id, result.output));
    }
    prompt
}