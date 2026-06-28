use regex::Regex;

use crate::types::Complexity;

/// A single routing rule: pattern → complexity → model.
#[allow(dead_code)]
struct RoutingRule {
    pattern: Regex,
    complexity: Complexity,
    model: &'static str,
}

/// Routes tasks to the appropriate model based on keyword heuristics.
pub struct ModelRouter {
    rules: Vec<RoutingRule>,
}

impl ModelRouter {
    /// Create a router with the default routing rules.
    pub fn new() -> Self {
        let rules = vec![
            RoutingRule {
                pattern: Regex::new(
                    r"(?i)\b(rename|format|complete|lint|indent|sort)\b",
                )
                .expect("valid regex"),
                complexity: Complexity::Low,
                model: "qwen3.5:4b",
            },
            RoutingRule {
                pattern: Regex::new(
                    r"(?i)\b(architect|refactor|design|debug|migrat|overhaul)\b",
                )
                .expect("valid regex"),
                complexity: Complexity::High,
                model: "qwen3-coder-next:q8_0",
            },
            RoutingRule {
                pattern: Regex::new(
                    r"(?i)\b(orchestrat|plan|strategy|roadmap|analyz|research)",
                )
                .expect("valid regex"),
                complexity: Complexity::Cloud,
                model: "qwen3.5:397b-cloud",
            },
            // Medium is the catch-all — checked last
            RoutingRule {
                pattern: Regex::new(
                    r"(?i)\b(implement|create|fix|test|review|write|build|add|update|patch)\b",
                )
                .expect("valid regex"),
                complexity: Complexity::Medium,
                model: "qwen3.6:27b",
            },
        ];

        Self { rules }
    }

    /// Classify the complexity of a task based on its description.
    pub fn classify(&self, request: &str) -> Complexity {
        for rule in &self.rules {
            if rule.pattern.is_match(request) {
                return rule.complexity;
            }
        }

        // Default to Medium if no rule matches
        Complexity::Medium
    }

    /// Select the model name for a given complexity level.
    pub fn select_model(&self, complexity: &Complexity) -> &'static str {
        match complexity {
            Complexity::Low => "qwen3.5:4b",
            Complexity::Medium => "qwen3.6:27b",
            Complexity::High => "qwen3-coder-next:q8_0",
            Complexity::Cloud => "qwen3.5:397b-cloud",
        }
    }

    /// Convenience: classify and select in one call.
    pub fn route(&self, request: &str) -> &'static str {
        let complexity = self.classify(request);
        self.select_model(&complexity)
    }
}

impl Default for ModelRouter {
    fn default() -> Self {
        Self::new()
    }
}