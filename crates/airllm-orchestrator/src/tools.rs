//! Tool calling system for the orchestrator.
//!
//! The LLM is instructed (via system prompt) to emit `[TOOL_CALL]...[/TOOL_CALL]`
//! blocks containing JSON with a `name` and `arguments` field. The orchestrator
//! parses these, executes the matching tool, and feeds results back into the
//! conversation as `[TOOL_RESULT]...[/TOOL_RESULT]` blocks.
//!
//! This approach works with any Ollama model (no native function-calling
//! required) and keeps the fast-path performance intact.

use std::fs;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use serde::de::Error as _;

const TOOL_CALL_OPEN: &str = "[TOOL_CALL]";
const TOOL_CALL_CLOSE: &str = "[/TOOL_CALL]";
const TOOL_RESULT_OPEN: &str = "[TOOL_RESULT]";
const TOOL_RESULT_CLOSE: &str = "[/TOOL_RESULT]";

/// A tool call extracted from LLM output.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolCall {
    pub name: String,
    pub arguments: serde_json::Value,
}

/// The result of executing a tool.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolResult {
    pub tool_name: String,
    pub success: bool,
    pub output: String,
    /// Files that were created or modified by this tool call.
    pub files_affected: Vec<String>,
}

impl ToolResult {
    pub fn ok(name: &str, output: impl Into<String>, files: Vec<String>) -> Self {
        Self {
            tool_name: name.to_string(),
            success: true,
            output: output.into(),
            files_affected: files,
        }
    }

    pub fn err(name: &str, msg: impl Into<String>) -> Self {
        Self {
            tool_name: name.to_string(),
            success: false,
            output: msg.into(),
            files_affected: vec![],
        }
    }

    /// Render as a block to feed back into the conversation.
    pub fn to_block(&self) -> String {
        let status = if self.success { "success" } else { "error" };
        let files = if self.files_affected.is_empty() {
            String::new()
        } else {
            format!("\n  [FILES]{}[/FILES]", self.files_affected.join(", "))
        };
        format!(
            "{TOOL_RESULT_OPEN} status=\"{status}\"\n  [TOOL]{tool}[/TOOL]\n  [OUTPUT]{output}{files}\n{TOOL_RESULT_CLOSE}",
            tool = self.tool_name,
            output = escape_text(&self.output),
        )
    }
}

/// Parse all tool call blocks from LLM output.
pub fn parse_tool_calls(output: &str) -> Vec<ToolCall> {
    let mut calls = Vec::new();
    let mut remaining = output;

    loop {
        let Some(start) = remaining.find(TOOL_CALL_OPEN) else {
            break;
        };
        let after_start = &remaining[start + TOOL_CALL_OPEN.len()..];
        let Some(end_rel) = after_start.find(TOOL_CALL_CLOSE) else {
            break;
        };
        let json_str = after_start[..end_rel].trim();

        if let Ok(call) = parse_single_tool_call(json_str) {
            calls.push(call);
        }

        remaining = &after_start[end_rel + TOOL_CALL_CLOSE.len()..];
    }

    calls
}

fn parse_single_tool_call(json_str: &str) -> std::result::Result<ToolCall, serde_json::Error> {
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(json_str) {
        return Ok(ToolCall {
            name: v.get("name")
                .and_then(|n| n.as_str())
                .unwrap_or("unknown")
                .to_string(),
            arguments: v.get("arguments")
                .cloned()
                .unwrap_or(serde_json::Value::Null),
        });
    }

    if let Ok(arr) = serde_json::from_str::<Vec<serde_json::Value>>(json_str) {
        if let Some(first) = arr.first() {
            return Ok(ToolCall {
                name: first.get("name")
                    .and_then(|n| n.as_str())
                    .unwrap_or("unknown")
                    .to_string(),
                arguments: first.get("arguments")
                    .cloned()
                    .unwrap_or(serde_json::Value::Null),
            });
        }
    }

    Err(serde_json::Error::custom("empty or invalid tool call JSON"))
}

/// Execute a single tool call and return its result.
pub fn execute_tool(call: &ToolCall) -> ToolResult {
    match call.name.as_str() {
        "file_write" | "write_file" | "FileWriteTool" => execute_file_write(&call.arguments),
        "file_read" | "read_file" | "FileReadTool" => execute_file_read(&call.arguments),
        "bash" | "run_command" | "BashTool" => execute_bash(&call.arguments),
        "list_files" | "ListFilesTool" => execute_list_files(&call.arguments),
        _ => ToolResult::err(&call.name, format!("Unknown tool: {}", call.name)),
    }
}

// -- Tool implementations --

fn execute_file_write(args: &serde_json::Value) -> ToolResult {
    let path = match args.get("file_path").or_else(|| args.get("path")) {
        Some(p) => match p.as_str() {
            Some(s) => s,
            None => return ToolResult::err("file_write", "file_path must be a string"),
        },
        None => return ToolResult::err("file_write", "missing required argument: file_path"),
    };

    let content = match args.get("content") {
        Some(c) => match c.as_str() {
            Some(s) => s,
            None => return ToolResult::err("file_write", "content must be a string"),
        },
        None => return ToolResult::err("file_write", "missing required argument: content"),
    };

    let expanded = expand_path(path);
    let filepath = PathBuf::from(&expanded);

    if let Some(parent) = filepath.parent() {
        if !parent.exists() {
            if let Err(e) = fs::create_dir_all(parent) {
                return ToolResult::err("file_write", format!("Failed to create directory {}: {}", parent.display(), e));
            }
        }
    }

    let file_exists = filepath.exists();

    match fs::write(&filepath, content) {
        Ok(_) => {
            let action = if file_exists { "Updated" } else { "Created" };
            ToolResult::ok(
                "file_write",
                format!("{} file: {}", action, filepath.display()),
                vec![filepath.display().to_string()],
            )
        }
        Err(e) => ToolResult::err("file_write", format!("Failed to write {}: {}", filepath.display(), e)),
    }
}

fn execute_file_read(args: &serde_json::Value) -> ToolResult {
    let path = match args.get("file_path").or_else(|| args.get("path")) {
        Some(p) => match p.as_str() {
            Some(s) => s,
            None => return ToolResult::err("file_read", "file_path must be a string"),
        },
        None => return ToolResult::err("file_read", "missing required argument: file_path"),
    };

    let expanded = expand_path(path);
    let filepath = PathBuf::from(&expanded);

    match fs::read_to_string(&filepath) {
        Ok(content) => {
            let truncated = if content.len() > 50_000 {
                format!("{}\n... [truncated, {} bytes total]", &content[..50_000], content.len())
            } else {
                content
            };
            ToolResult::ok("file_read", truncated, vec![])
        }
        Err(e) => ToolResult::err("file_read", format!("Failed to read {}: {}", filepath.display(), e)),
    }
}

fn execute_bash(args: &serde_json::Value) -> ToolResult {
    let command = match args.get("command") {
        Some(c) => match c.as_str() {
            Some(s) => s,
            None => return ToolResult::err("bash", "command must be a string"),
        },
        None => return ToolResult::err("bash", "missing required argument: command"),
    };

    let dangerous = ["rm -rf /", "mkfs", "dd if=", ":(){ :|:& };:"];
    for pattern in &dangerous {
        if command.contains(pattern) {
            return ToolResult::err("bash", format!("Blocked dangerous command pattern: {}", pattern));
        }
    }

    let output = std::process::Command::new("bash")
        .arg("-c")
        .arg(command)
        .output();

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout).to_string();
            let stderr = String::from_utf8_lossy(&out.stderr).to_string();
            let combined = if stderr.is_empty() {
                stdout
            } else {
                format!("{}\n--- stderr ---\n{}", stdout, stderr)
            };

            if out.status.success() {
                ToolResult::ok("bash", combined, vec![])
            } else {
                ToolResult::err("bash", format!("Exit code: {}\n{}", out.status, combined))
            }
        }
        Err(e) => ToolResult::err("bash", format!("Failed to execute command: {}", e)),
    }
}

fn execute_list_files(args: &serde_json::Value) -> ToolResult {
    let path = match args.get("path") {
        Some(p) => match p.as_str() {
            Some(s) => s,
            None => return ToolResult::err("list_files", "path must be a string"),
        },
        None => ".",
    };

    let expanded = expand_path(path);
    let dirpath = PathBuf::from(&expanded);

    match fs::read_dir(&dirpath) {
        Ok(entries) => {
            let mut files = Vec::new();
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                let ft = entry.file_type();
                let prefix = if ft.map(|t| t.is_dir()).unwrap_or(false) { "[DIR] " } else { "      " };
                files.push(format!("{}{}", prefix, name));
            }
            files.sort();
            ToolResult::ok("list_files", files.join("\n"), vec![])
        }
        Err(e) => ToolResult::err("list_files", format!("Failed to list {}: {}", dirpath.display(), e)),
    }
}

// -- Utilities --

/// Expand tilde and relative paths to absolute.
pub fn expand_path(path: &str) -> String {
    if path == "~" {
        return std::env::var("HOME").unwrap_or_else(|_| "~".to_string());
    }
    if let Some(rest) = path.strip_prefix("~/") {
        let home = std::env::var("HOME").unwrap_or_else(|_| "~".to_string());
        return format!("{}/{}", home, rest);
    }
    if Path::new(path).is_absolute() {
        return path.to_string();
    }
    std::env::current_dir()
        .map(|cwd| cwd.join(path).to_string_lossy().to_string())
        .unwrap_or_else(|_| path.to_string())
}

fn escape_text(s: &str) -> String {
    s.replace('[', "\\[").replace(']', "\\]")
}

/// Extract any text that appears outside of tool call blocks.
/// This is the visible-to-user portion of the LLM response.
pub fn extract_visible_text(output: &str) -> String {
    let mut result = String::new();
    let mut remaining = output;

    loop {
        let Some(start) = remaining.find(TOOL_CALL_OPEN) else {
            result.push_str(remaining);
            break;
        };
        result.push_str(&remaining[..start]);
        let after_start = &remaining[start + TOOL_CALL_OPEN.len()..];
        if let Some(end_rel) = after_start.find(TOOL_CALL_CLOSE) {
            remaining = &after_start[end_rel + TOOL_CALL_CLOSE.len()..];
        } else {
            break;
        }
    }

    result.trim().to_string()
}

/// Check if the LLM output contains any tool calls.
pub fn has_tool_calls(output: &str) -> bool {
    output.contains(TOOL_CALL_OPEN)
}

/// Get the tool calling instructions for the system prompt.
/// This tells the LLM how to format tool calls.
pub fn tool_instructions() -> String {
    format!(
r#"You have access to the following tools for file operations and code execution:

1. file_write - Write content to a file
   {open}
   {{"name": "file_write", "arguments": {{"file_path": "/absolute/path/to/file.py", "content": "file content here"}}}}
   {close}

2. file_read - Read a file's contents
   {open}
   {{"name": "file_read", "arguments": {{"file_path": "/absolute/path/to/file.py"}}}}
   {close}

3. bash - Execute a shell command
   {open}
   {{"name": "bash", "arguments": {{"command": "ls -la"}}}}
   {close}

4. list_files - List files in a directory
   {open}
   {{"name": "list_files", "arguments": {{"path": "/some/directory"}}}}
   {close}

RULES:
- Use tools IMMEDIATELY when you need to create, read, or inspect files. Do not explain first, just call the tool.
- Always use ABSOLUTE paths for file_path.
- After each tool call, you will receive a [TOOL_RESULT] block with the outcome. Use it to continue.
- You can make multiple tool calls in a single response if they are independent.
- For file creation, use file_write with the full content. Do NOT use code blocks for file output - use the tool instead.
- Be concise in your text output. Lead with the action, not the explanation."#,
        open = TOOL_CALL_OPEN,
        close = TOOL_CALL_CLOSE,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_single_tool_call() {
        let json = r#"{"name": "file_write", "arguments": {"file_path": "/tmp/test.py", "content": "print('hi')"}}"#;
        let call = parse_single_tool_call(json).unwrap();
        assert_eq!(call.name, "file_write");
        assert_eq!(call.arguments["file_path"], "/tmp/test.py");
    }

    #[test]
    fn test_parse_tool_calls_from_output() {
        let output = format!("I'll create the file now.\n{}\n{{\"name\": \"file_write\", \"arguments\": {{\"file_path\": \"/tmp/hello.py\", \"content\": \"print('Hello World')\"}}}}\n{}\nDone!", TOOL_CALL_OPEN, TOOL_CALL_CLOSE);
        let calls = parse_tool_calls(&output);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "file_write");
    }

    #[test]
    fn test_parse_multiple_tool_calls() {
        let output = format!(
            "{o}\n{{\"name\": \"file_write\", \"arguments\": {{\"file_path\": \"/tmp/a.py\", \"content\": \"a\"}}}}\n{c}\n{o}\n{{\"name\": \"file_read\", \"arguments\": {{\"file_path\": \"/tmp/a.py\"}}}}\n{c}",
            o = TOOL_CALL_OPEN, c = TOOL_CALL_CLOSE
        );
        let calls = parse_tool_calls(&output);
        assert_eq!(calls.len(), 2);
    }

    #[test]
    fn test_execute_file_write() {
        let args = serde_json::json!({
            "file_path": "/tmp/airllm_tool_test.txt",
            "content": "test content"
        });
        let result = execute_tool(&ToolCall {
            name: "file_write".to_string(),
            arguments: args,
        });
        assert!(result.success);
        assert_eq!(result.files_affected, vec!["/tmp/airllm_tool_test.txt"]);
        assert_eq!(fs::read_to_string("/tmp/airllm_tool_test.txt").unwrap(), "test content");
        fs::remove_file("/tmp/airllm_tool_test.txt").ok();
    }

    #[test]
    fn test_execute_file_read() {
        fs::write("/tmp/airllm_read_test.txt", "hello").ok();
        let args = serde_json::json!({"file_path": "/tmp/airllm_read_test.txt"});
        let result = execute_tool(&ToolCall {
            name: "file_read".to_string(),
            arguments: args,
        });
        assert!(result.success);
        assert_eq!(result.output, "hello");
        fs::remove_file("/tmp/airllm_read_test.txt").ok();
    }

    #[test]
    fn test_execute_bash() {
        let args = serde_json::json!({"command": "echo hello_world"});
        let result = execute_tool(&ToolCall {
            name: "bash".to_string(),
            arguments: args,
        });
        assert!(result.success);
        assert!(result.output.contains("hello_world"));
    }

    #[test]
    fn test_execute_bash_dangerous_blocked() {
        let args = serde_json::json!({"command": "rm -rf /"});
        let result = execute_tool(&ToolCall {
            name: "bash".to_string(),
            arguments: args,
        });
        assert!(!result.success);
    }

    #[test]
    fn test_expand_path() {
        assert_eq!(expand_path("/abs/path"), "/abs/path");
        assert!(expand_path("~/test").starts_with('/'));
    }

    #[test]
    fn test_extract_visible_text() {
        let output = format!("Hello!\n{o}\n{{\"name\": \"x\"}}\n{c}\nDone!", o = TOOL_CALL_OPEN, c = TOOL_CALL_CLOSE);
        let visible = extract_visible_text(&output);
        assert_eq!(visible, "Hello!\n\nDone!");
    }

    #[test]
    fn test_has_tool_calls() {
        assert!(has_tool_calls(TOOL_CALL_OPEN));
        assert!(!has_tool_calls("just text"));
    }

    #[test]
    fn test_tool_result_block() {
        let result = ToolResult::ok("file_write", "Created file", vec!["/tmp/test.py".to_string()]);
        let block = result.to_block();
        assert!(block.contains("success"));
        assert!(block.contains("file_write"));
        assert!(block.contains("/tmp/test.py"));
    }

    #[test]
    fn test_unknown_tool() {
        let result = execute_tool(&ToolCall {
            name: "nonexistent".to_string(),
            arguments: serde_json::Value::Null,
        });
        assert!(!result.success);
        assert!(result.output.contains("Unknown tool"));
    }
}