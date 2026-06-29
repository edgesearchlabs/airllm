//! Permission system for tool execution.
//!
//! Three modes:
//! - `Default`: prompt user for confirmation before each write/bash action
//! - `AcceptEdits`: auto-approve file writes, prompt for bash
//! - `Bypass`: no prompts, execute everything

use std::io::{self, Write};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PermissionMode {
    #[default]
    Default,
    AcceptEdits,
    Bypass,
}

impl PermissionMode {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::AcceptEdits => "accept-edits",
            Self::Bypass => "bypass",
        }
    }

    pub fn parse_mode(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "accept" | "accept-edits" | "acceptedits" | "auto" => Self::AcceptEdits,
            "bypass" | "full" | "full-access" | "yes" => Self::Bypass,
            _ => Self::Default,
        }
    }
}

/// The type of action being requested for permission check.
#[derive(Clone, Debug)]
pub enum PermissionAction {
    FileWrite { path: String, content: String },
    FileRead { path: String },
    Bash { command: String },
    ListFiles { path: String },
}

impl PermissionAction {
    pub fn tool_name(&self) -> &str {
        match self {
            Self::FileWrite { .. } => "file_write",
            Self::FileRead { .. } => "file_read",
            Self::Bash { .. } => "bash",
            Self::ListFiles { .. } => "list_files",
        }
    }

    pub fn is_destructive(&self) -> bool {
        matches!(self, Self::FileWrite { .. } | Self::Bash { .. })
    }

    /// Short description for the prompt.
    pub fn description(&self) -> String {
        match self {
            Self::FileWrite { path, content } => {
                let lines = content.lines().count();
                let bytes = content.len();
                let exists = std::path::Path::new(path).exists();
                let action = if exists { "overwrite" } else { "create" };
                format!("{action} {path} ({lines} lines, {bytes} bytes)")
            }
            Self::FileRead { path } => format!("read {path}"),
            Self::Bash { command } => {
                let preview: String = command.chars().take(80).collect();
                format!("run: {preview}")
            }
            Self::ListFiles { path } => format!("list files in {path}"),
        }
    }
}

/// Result of a permission check.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PermissionDecision {
    /// User approved this single action.
    Allow,
    /// User approved all actions of this type for the session.
    AllowSession,
    /// User denied the action.
    Deny { reason: String },
}

/// Check if an action is allowed given the current permission mode.
/// If the mode is Default or AcceptEdits (for bash), this will prompt the user.
pub fn check_permission(action: &PermissionAction, mode: PermissionMode) -> PermissionDecision {
    match mode {
        PermissionMode::Bypass => PermissionDecision::Allow,
        PermissionMode::AcceptEdits => {
            match action {
                PermissionAction::FileWrite { .. } | PermissionAction::FileRead { .. } | PermissionAction::ListFiles { .. } => {
                    PermissionDecision::Allow
                }
                PermissionAction::Bash { .. } => prompt_user(action),
            }
        }
        PermissionMode::Default => {
            match action {
                PermissionAction::FileRead { .. } | PermissionAction::ListFiles { .. } => PermissionDecision::Allow,
                PermissionAction::FileWrite { .. } | PermissionAction::Bash { .. } => prompt_user(action),
            }
        }
    }
}

/// Prompt the user for confirmation via stdin.
fn prompt_user(action: &PermissionAction) -> PermissionDecision {
    let desc = action.description();
    let tool = action.tool_name();

    // Print the prompt
    eprintln!();
    eprintln!("┌─ Permission required ─────────────────────────────");
    eprintln!("│ Tool: {tool}");
    eprintln!("│ Action: {desc}");

    // Show a preview for file writes
    if let PermissionAction::FileWrite { content, path } = action {
        let preview: String = content.lines().take(10).collect::<Vec<_>>().join("\n");
        eprintln!("│ Preview ({path}):");
        for line in preview.lines() {
            eprintln!("│   {line}");
        }
        let total_lines = content.lines().count();
        if total_lines > 10 {
            eprintln!("│   ... ({} more lines)", total_lines - 10);
        }
    }

    eprintln!("└────────────────────────────────────────────────────");
    eprint!("Allow? [y]es / [n]o / [a]lways (session): ");
    io::stderr().flush().ok();

    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_err() {
        return PermissionDecision::Deny { reason: "Failed to read input".to_string() };
    }

    match input.trim().to_lowercase().as_str() {
        "y" | "yes" => PermissionDecision::Allow,
        "a" | "always" => PermissionDecision::AllowSession,
        _ => PermissionDecision::Deny { reason: "User denied".to_string() },
    }
}

/// Check if a path is in a dangerous/protected location.
pub fn is_dangerous_path(path: &str) -> bool {
    let dangerous_files = [
        ".gitconfig", ".gitmodules", ".bashrc", ".bash_profile",
        ".zshrc", ".zprofile", ".profile", ".ssh/config",
        ".env", ".env.local",
    ];
    let dangerous_dirs = [".git/", ".ssh/", ".vscode/", ".idea/"];

    let normalized = path.replace('\\', "/");
    let basename = normalized.rsplit('/').next().unwrap_or("");

    for df in &dangerous_files {
        if basename == *df {
            return true;
        }
    }
    for dd in &dangerous_dirs {
        if normalized.contains(dd) {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_permission_mode_from_str() {
        assert_eq!(PermissionMode::parse_mode("default"), PermissionMode::Default);
        assert_eq!(PermissionMode::parse_mode("accept"), PermissionMode::AcceptEdits);
        assert_eq!(PermissionMode::parse_mode("bypass"), PermissionMode::Bypass);
        assert_eq!(PermissionMode::parse_mode("auto"), PermissionMode::AcceptEdits);
        assert_eq!(PermissionMode::parse_mode("full-access"), PermissionMode::Bypass);
    }

    #[test]
    fn test_bypass_allows_everything() {
        let action = PermissionAction::FileWrite {
            path: "/tmp/test.txt".to_string(),
            content: "hello".to_string(),
        };
        assert_eq!(check_permission(&action, PermissionMode::Bypass), PermissionDecision::Allow);
    }

    #[test]
    fn test_accept_edits_allows_file_write() {
        let action = PermissionAction::FileWrite {
            path: "/tmp/test.txt".to_string(),
            content: "hello".to_string(),
        };
        assert_eq!(check_permission(&action, PermissionMode::AcceptEdits), PermissionDecision::Allow);
    }

    #[test]
    fn test_default_allows_read() {
        let action = PermissionAction::FileRead {
            path: "/tmp/test.txt".to_string(),
        };
        assert_eq!(check_permission(&action, PermissionMode::Default), PermissionDecision::Allow);
    }

    #[test]
    fn test_dangerous_paths() {
        assert!(is_dangerous_path("/home/user/.bashrc"));
        assert!(is_dangerous_path("/home/user/.git/config"));
        assert!(is_dangerous_path("/home/user/.ssh/id_rsa"));
        assert!(is_dangerous_path("/home/user/.env"));
        assert!(!is_dangerous_path("/tmp/hello.py"));
        assert!(!is_dangerous_path("/home/user/project/main.rs"));
    }

    #[test]
    fn test_action_description() {
        let action = PermissionAction::Bash {
            command: "ls -la".to_string(),
        };
        assert!(action.description().contains("ls -la"));
    }
}