//! Parse file paths, languages, and intents from user prompts.
//!
//! Extracts structured information from natural language input like:
//! - "cria hello.py em /tmp/" -> path=/tmp/hello.py, lang=python
//! - "create a rust file at src/main.rs" -> path=src/main.rs, lang=rust
//! - "write a calculator in python" -> lang=python, filename=calculator.py

use std::path::Path;

/// Parsed intent from a user prompt.
#[derive(Clone, Debug, Default)]
pub struct ParsedIntent {
    /// Explicit file path mentioned in the prompt, if any.
    pub file_path: Option<String>,
    /// Output directory mentioned, if any.
    pub output_dir: Option<String>,
    /// Programming language detected from extension or keywords.
    pub language: Option<String>,
    /// Suggested filename based on the task description.
    pub filename: Option<String>,
    /// Whether the user wants to create a new file.
    pub wants_create: bool,
    /// Whether the user wants to read/inspect an existing file.
    pub wants_read: bool,
}

/// Parse a user prompt to extract file path, language, and intent.
pub fn parse_prompt(prompt: &str) -> ParsedIntent {
    let mut intent = ParsedIntent::default();
    let lowered = prompt.to_lowercase();

    // Detect intent
    intent.wants_create = lowered.contains("create") || lowered.contains("cria")
        || lowered.contains("write") || lowered.contains("escreve")
        || lowered.contains("make") || lowered.contains("gera") || lowered.contains("generate");
    intent.wants_read = lowered.contains("read") || lowered.contains("leia")
        || lowered.contains("show") || lowered.contains("mostra") || lowered.contains("cat");

    // Extract explicit file paths (look for patterns like /path/to/file.ext or ./file.ext)
    intent.file_path = extract_file_path(prompt);

    // Extract output directory (look for "em /path", "in /path", "at /path", "to /path")
    intent.output_dir = extract_output_dir(&lowered);

    // Detect language from file extension or keywords
    if let Some(ref path) = intent.file_path {
        intent.language = language_from_extension(path);
    }
    if intent.language.is_none() {
        intent.language = detect_language_keywords(&lowered);
    }

    // Suggest a filename if not explicitly provided
    if intent.file_path.is_none() {
        intent.filename = suggest_filename(&lowered, intent.language.as_deref());
    }

    intent
}

/// Extract a file path from the prompt.
/// Looks for paths starting with /, ./, ~/, or containing a file extension.
fn extract_file_path(prompt: &str) -> Option<String> {
    // Pattern 1: explicit path like /tmp/hello.py or ./src/main.rs or ~/file.txt
    let words: Vec<&str> = prompt.split_whitespace().collect();
    for word in &words {
        let clean = word.trim_matches(|c: char| !c.is_alphanumeric() && c != '/' && c != '.' && c != '-' && c != '_' && c != '~');
        if (clean.starts_with('/') || clean.starts_with("./") || clean.starts_with("~/"))
            && has_file_extension(clean)
        {
            return Some(clean.to_string());
        }
        // Pattern 2: filename.ext as a standalone word
        if has_file_extension(clean) && clean.contains('.') && !clean.starts_with('.') {
            // Make sure it looks like a filename, not a sentence ending
            if clean.len() > 3 && clean.matches('.').count() <= 2 {
                return Some(clean.to_string());
            }
        }
    }
    None
}

/// Extract output directory from phrases like "em /tmp/", "in /home/user", "at ./src".
fn extract_output_dir(lowered: &str) -> Option<String> {
    let markers = ["em ", "in ", "at ", "to ", "no diretório ", "directory "];
    for marker in &markers {
        if let Some(pos) = lowered.find(marker) {
            let after = &lowered[pos + marker.len()..];
            // Extract the path-like token after the marker
            let path_end = after
                .find(|c: char| c.is_whitespace() || c == ',' || c == '.')
                .unwrap_or(after.len());
            let candidate = after[..path_end].trim();
            if candidate.starts_with('/') || candidate.starts_with("./") || candidate.starts_with("~/") {
                return Some(candidate.trim_end_matches('/').to_string());
            }
        }
    }
    None
}

/// Check if a string has a file extension (contains a dot followed by 1-5 alpha chars).
fn has_file_extension(s: &str) -> bool {
    if let Some(dot_pos) = s.rfind('.') {
        let ext = &s[dot_pos + 1..];
        // Extension must be 1-5 alphanumeric chars and the string before the dot
        // must not be empty (to avoid matching ".bashrc" as having an extension)
        !ext.is_empty()
            && ext.len() <= 5
            && ext.chars().all(|c| c.is_alphanumeric())
            && dot_pos > 0
    } else {
        false
    }
}

/// Detect language from file extension.
pub fn language_from_extension(path: &str) -> Option<String> {
    let ext = Path::new(path)
        .extension()?
        .to_string_lossy()
        .to_lowercase();
    let lang = match ext.as_str() {
        "rs" => "rust",
        "py" => "python",
        "js" | "mjs" | "cjs" => "javascript",
        "ts" | "mts" | "cts" => "typescript",
        "java" => "java",
        "go" => "go",
        "c" | "h" => "c",
        "cpp" | "cc" | "cxx" | "hpp" => "cpp",
        "rb" => "ruby",
        "php" => "php",
        "swift" => "swift",
        "kt" => "kotlin",
        "sh" | "bash" => "shell",
        "html" | "htm" => "html",
        "css" => "css",
        "json" => "json",
        "toml" => "toml",
        "yaml" | "yml" => "yaml",
        "sql" => "sql",
        "md" => "markdown",
        _ => return None,
    };
    Some(lang.to_string())
}

/// Detect language from keywords in the prompt.
fn detect_language_keywords(lowered: &str) -> Option<String> {
    let keywords: &[(&str, &str)] = &[
        ("rust", "rust"), ("cargo", "rust"),
        ("python", "python"), ("django", "python"), ("flask", "python"), ("pandas", "python"),
        ("javascript", "javascript"), ("node", "javascript"), ("nodejs", "javascript"), ("npm", "javascript"),
        ("typescript", "typescript"), ("deno", "typescript"),
        ("java", "java"), ("spring", "java"), ("maven", "java"),
        ("go ", "go"), ("golang", "go"),
        ("c++", "cpp"), ("cpp", "cpp"),
        ("ruby", "ruby"), ("rails", "ruby"),
        ("php", "php"),
        ("shell", "shell"), ("bash", "shell"),
        ("html", "html"),
        ("css", "css"),
        ("sql", "sql"),
    ];
    for (keyword, lang) in keywords {
        if lowered.contains(keyword) {
            return Some(lang.to_string());
        }
    }
    None
}

/// Suggest a filename based on the task description and language.
fn suggest_filename(lowered: &str, language: Option<&str>) -> Option<String> {
    // Look for "called X" or "chamado X" or "named X" patterns
    let markers = ["called ", "chamado ", "named ", "nomeado "];
    for marker in &markers {
        if let Some(pos) = lowered.find(marker) {
            let after = &lowered[pos + marker.len()..];
            // Extract the name token — allow alphanumeric, dots, dashes, underscores
            let name_end = after
                .find(|c: char| c.is_whitespace() || c == ',')
                .unwrap_or(after.len());
            let name = after[..name_end].trim();
            if !name.is_empty() && name.len() > 1 {
                // If the name already has an extension, return it as-is
                if name.contains('.') {
                    return Some(name.to_string());
                }
                let ext = extension_for_language(language.unwrap_or("txt"));
                return Some(format!("{name}.{ext}"));
            }
        }
    }

    // Try to derive from key nouns in the prompt
    let candidates = ["calculator", "calculadora", "server", "servidor", "client", "cliente",
                      "api", "app", "application", "tool", "ferramenta", "script",
                      "hello", "main", "index", "test", "teste"];
    for candidate in &candidates {
        if lowered.contains(candidate) {
            let ext = extension_for_language(language.unwrap_or("txt"));
            return Some(format!("{candidate}.{ext}"));
        }
    }

    None
}

/// Get the file extension for a language name.
pub fn extension_for_language(lang: &str) -> &str {
    match lang.to_lowercase().as_str() {
        "rust" => "rs",
        "python" => "py",
        "javascript" => "js",
        "typescript" => "ts",
        "java" => "java",
        "go" => "go",
        "c" => "c",
        "cpp" | "c++" => "cpp",
        "ruby" => "rb",
        "php" => "php",
        "swift" => "swift",
        "kotlin" => "kt",
        "shell" | "bash" => "sh",
        "html" => "html",
        "css" => "css",
        "json" => "json",
        "toml" => "toml",
        "yaml" | "yml" => "yaml",
        "sql" => "sql",
        "markdown" | "md" => "md",
        _ => "txt",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_explicit_path() {
        let intent = parse_prompt("create hello.py at /tmp/");
        assert_eq!(intent.file_path, Some("hello.py".to_string()));
        assert!(intent.wants_create);
    }

    #[test]
    fn test_parse_absolute_path() {
        let intent = parse_prompt("write code to /home/user/main.rs");
        assert_eq!(intent.file_path, Some("/home/user/main.rs".to_string()));
        assert_eq!(intent.language, Some("rust".to_string()));
    }

    #[test]
    fn test_parse_output_dir() {
        let intent = parse_prompt("cria um arquivo python em /tmp/test");
        assert_eq!(intent.output_dir, Some("/tmp/test".to_string()));
        assert_eq!(intent.language, Some("python".to_string()));
    }

    #[test]
    fn test_parse_language_from_keyword() {
        let intent = parse_prompt("write a rust web server");
        assert_eq!(intent.language, Some("rust".to_string()));
    }

    #[test]
    fn test_parse_filename_suggestion() {
        let intent = parse_prompt("create a python calculator");
        assert_eq!(intent.language, Some("python".to_string()));
        assert_eq!(intent.filename, Some("calculator.py".to_string()));
    }

    #[test]
    fn test_parse_called_pattern() {
        let intent = parse_prompt("create a file called hello.py");
        // "hello.py" is detected as a file path (has extension)
        assert!(intent.file_path.is_some() || intent.filename.is_some());
        let result = intent.file_path.or(intent.filename).unwrap();
        assert_eq!(result, "hello.py");
    }

    #[test]
    fn test_language_from_extension() {
        assert_eq!(language_from_extension("main.rs"), Some("rust".to_string()));
        assert_eq!(language_from_extension("app.py"), Some("python".to_string()));
        assert_eq!(language_from_extension("index.ts"), Some("typescript".to_string()));
        assert_eq!(language_from_extension("no_ext"), None);
    }

    #[test]
    fn test_extension_for_language() {
        assert_eq!(extension_for_language("rust"), "rs");
        assert_eq!(extension_for_language("python"), "py");
        assert_eq!(extension_for_language("javascript"), "js");
        assert_eq!(extension_for_language("unknown"), "txt");
    }

    #[test]
    fn test_wants_create() {
        assert!(parse_prompt("create a file").wants_create);
        assert!(parse_prompt("cria hello.py").wants_create);
        assert!(!parse_prompt("what is 2+2").wants_create);
    }

    #[test]
    fn test_wants_read() {
        assert!(parse_prompt("read main.rs").wants_read);
        assert!(parse_prompt("mostra o arquivo").wants_read);
        assert!(!parse_prompt("create a file").wants_read);
    }

    #[test]
    fn test_has_file_extension() {
        assert!(has_file_extension("hello.py"));
        assert!(has_file_extension("main.rs"));
        assert!(!has_file_extension("hello"));
        assert!(!has_file_extension(".bashrc"));
        assert!(!has_file_extension("no_ext"));
    }
}