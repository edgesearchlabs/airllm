use airllm_ollama::{ChatOptions, Message, OllamaClient};

use crate::agent::Agent;
use crate::error::Result;
use crate::types::{CodeRequest, SubTask};

pub async fn decompose_request(
    request: &CodeRequest,
    architect: &Agent,
    model: &str,
    ollama: &OllamaClient,
) -> Result<Vec<SubTask>> {
    let prompt = format!(
    r#"Task: {task}
Language: {language}
Files: {files}

Return ONLY a JSON array of sub-tasks in the shape
[{{"id":"t1","description":"...","agent_name":"coder","input_files":["..."]}}].
Use only these agent names: coder, reviewer, tester, architect, debugger, refactorer, documenter, planner, security, performance.
Prefer 1 to 4 sub-tasks. If the task is simple, return a single sub-task."#,
        task = request.task,
        language = request.language.clone().unwrap_or_else(|| "unspecified".to_string()),
        files = if request.files.is_empty() {
            "(none)".to_string()
        } else {
            request.files.join(", ")
        }
    );

    let messages = vec![
        Message::system(architect.system_prompt.clone()),
        Message::user(prompt),
    ];

    let response = ollama
        .chat(
            model,
            &messages,
            ChatOptions {
                temperature: 0.1,
                top_p: 0.9,
                ..ChatOptions::default()
            },
        )
        .await;

    let response = match response {
        Ok(value) => value,
        Err(_) => return Ok(vec![fallback_subtask(request)]),
    };

    match extract_json_array(&response)
        .and_then(|json| serde_json::from_str::<Vec<SubTask>>(&json).ok())
    {
        Some(subtasks) if !subtasks.is_empty() => Ok(subtasks),
        _ => Ok(vec![fallback_subtask(request)]),
    }
}

fn fallback_subtask(request: &CodeRequest) -> SubTask {
    SubTask {
        id: "task-1".to_string(),
        description: request.task.clone(),
        agent_name: infer_agent_name(&request.task).to_string(),
        input_files: request.files.clone(),
    }
}

fn infer_agent_name(task: &str) -> &'static str {
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
    } else if lowered.contains("architect") || lowered.contains("design") {
        "architect"
    } else {
        "coder"
    }
}

fn extract_json_array(response: &str) -> Option<String> {
    let trimmed = response.trim();
    if trimmed.starts_with('[') && trimmed.ends_with(']') {
        return Some(trimmed.to_string());
    }

    if let Some(start) = trimmed.find("```") {
        let rest = &trimmed[start + 3..];
        let rest = rest.strip_prefix("json").unwrap_or(rest).trim_start();
        if let Some(end) = rest.find("```") {
            let candidate = rest[..end].trim();
            if candidate.starts_with('[') && candidate.ends_with(']') {
                return Some(candidate.to_string());
            }
        }
    }

    let start = trimmed.find('[')?;
    let end = trimmed.rfind(']')?;
    Some(trimmed[start..=end].to_string())
}