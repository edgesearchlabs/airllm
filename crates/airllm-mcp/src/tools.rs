use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

pub fn available_tools() -> Vec<ToolDefinition> {
    vec![code_tool(), review_tool(), test_tool(), list_models_tool()]
}

fn code_tool() -> ToolDefinition {
    ToolDefinition {
        name: "code".into(),
        description: "Generate code for a given task".into(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "task": {"type": "string"},
                "language": {"type": "string"},
                "files": {"type": "array", "items": {"type": "string"}},
                "model_override": {"type": "string"}
            },
            "required": ["task"],
        }),
    }
}

fn review_tool() -> ToolDefinition {
    ToolDefinition {
        name: "review".into(),
        description: "Review provided files".into(),
        input_schema: json!({
            "type": "object",
            "properties": {"files": {"type": "array", "items": {"type": "string"}}},
            "required": ["files"],
        }),
    }
}

fn test_tool() -> ToolDefinition {
    ToolDefinition {
        name: "test".into(),
        description: "Generate or run tests for provided files".into(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "files": {"type": "array", "items": {"type": "string"}},
                "framework": {"type": "string"}
            },
            "required": ["files"],
        }),
    }
}

fn list_models_tool() -> ToolDefinition {
    ToolDefinition {
        name: "list_models".into(),
        description: "List available models".into(),
        input_schema: json!({"type": "object", "properties": {}, "additionalProperties": false}),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tools_present() {
        let names: Vec<_> = available_tools().into_iter().map(|t| t.name).collect();
        assert!(names.contains(&"code".to_string()));
        assert!(names.contains(&"review".to_string()));
        assert!(names.contains(&"test".to_string()));
        assert!(names.contains(&"list_models".to_string()));
    }
}
