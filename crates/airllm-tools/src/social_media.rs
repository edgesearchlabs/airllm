//! Social media tool — post to Twitter/X, LinkedIn, etc.

use async_trait::async_trait;
use serde_json::Value;

use crate::{Tool, ToolError, ToolResult};

pub struct SocialMediaTool {
    #[allow(dead_code)]
    http: reqwest::Client,
}

impl SocialMediaTool {
    pub fn new() -> Self {
        Self {
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap_or_default(),
        }
    }
}

impl Default for SocialMediaTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for SocialMediaTool {
    fn name(&self) -> &str {
        "post_social"
    }

    fn description(&self) -> &str {
        "Post content to social media platforms (Twitter/X, LinkedIn)"
    }

    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "platform": {"type": "string", "enum": ["twitter", "linkedin"], "description": "Target platform"},
                "content": {"type": "string", "description": "Post content text"},
                "api_key": {"type": "string", "description": "Platform API key (from env: SOCIAL_API_KEY)"},
                "api_secret": {"type": "string", "description": "Platform API secret (from env: SOCIAL_API_SECRET)"}
            },
            "required": ["platform", "content"]
        })
    }

    async fn execute(&self, args: Value) -> ToolResult<Value> {
        let platform = args
            .get("platform")
            .and_then(Value::as_str)
            .ok_or_else(|| ToolError::InvalidArgs("platform is required".into()))?;
        let content = args
            .get("content")
            .and_then(Value::as_str)
            .ok_or_else(|| ToolError::InvalidArgs("content is required".into()))?;

        // Resolve credentials from args or env
        let api_key = args
            .get("api_key")
            .and_then(Value::as_str)
            .map(String::from)
            .or_else(|| std::env::var("SOCIAL_API_KEY").ok())
            .ok_or_else(|| {
                ToolError::InvalidArgs("api_key required (arg or SOCIAL_API_KEY env)".into())
            })?;

        let api_secret = args
            .get("api_secret")
            .and_then(Value::as_str)
            .map(String::from)
            .or_else(|| std::env::var("SOCIAL_API_SECRET").ok())
            .unwrap_or_default();

        // In production, this would call the actual platform API.
        // For now, we build the request and return a simulated result.
        let endpoint = match platform {
            "twitter" => "https://api.twitter.com/2/tweets",
            "linkedin" => "https://api.linkedin.com/v2/ugcPosts",
            other => return Err(ToolError::InvalidArgs(format!("unsupported platform: {other}"))),
        };

        // Simulate the API call (real implementation would use OAuth)
        tracing::info!(
            platform,
            endpoint,
            content_len = content.len(),
            "social media post prepared"
        );

        Ok(serde_json::json!({
            "platform": platform,
            "endpoint": endpoint,
            "status": "prepared",
            "content_length": content.len(),
            "api_key_present": !api_key.is_empty(),
            "api_secret_present": !api_secret.is_empty(),
            "note": "In production, this would make the actual API call. Use mock mode for testing."
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_missing_platform() {
        let tool = SocialMediaTool::new();
        let result = tool
            .execute(serde_json::json!({"content": "hello"}))
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_missing_api_key() {
        let tool = SocialMediaTool::new();
        let result = tool
            .execute(serde_json::json!({"platform": "twitter", "content": "hello"}))
            .await;
        assert!(result.is_err()); // no api_key
    }
}