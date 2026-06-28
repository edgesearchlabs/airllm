//! Webhook tool — generic HTTP call.

use async_trait::async_trait;
use serde_json::Value;

use crate::{Tool, ToolError, ToolResult};

pub struct WebhookTool {
    http: reqwest::Client,
}

impl WebhookTool {
    pub fn new() -> Self {
        Self {
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap_or_default(),
        }
    }
}

impl Default for WebhookTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for WebhookTool {
    fn name(&self) -> &str {
        "webhook_call"
    }

    fn description(&self) -> &str {
        "Send an HTTP request to a webhook URL (POST/GET/PUT/DELETE)"
    }

    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "url": {"type": "string", "description": "Target URL"},
                "method": {"type": "string", "enum": ["GET", "POST", "PUT", "DELETE"], "default": "POST"},
                "headers": {"type": "object", "description": "HTTP headers"},
                "body": {"description": "Request body (JSON object or string)"}
            },
            "required": ["url"]
        })
    }

    async fn execute(&self, args: Value) -> ToolResult<Value> {
        let url = args
            .get("url")
            .and_then(Value::as_str)
            .ok_or_else(|| ToolError::InvalidArgs("url is required".into()))?;
        let method = args
            .get("method")
            .and_then(Value::as_str)
            .unwrap_or("POST");
        let headers = args.get("headers").cloned().unwrap_or(Value::Null);
        let body = args.get("body").cloned().unwrap_or(Value::Null);

        let mut req = match method.to_uppercase().as_str() {
            "GET" => self.http.get(url),
            "POST" => self.http.post(url),
            "PUT" => self.http.put(url),
            "DELETE" => self.http.delete(url),
            other => return Err(ToolError::InvalidArgs(format!("unsupported method: {other}"))),
        };

        // Add headers
        if let Some(h) = headers.as_object() {
            for (key, val) in h {
                if let Some(s) = val.as_str() {
                    req = req.header(key, s);
                }
            }
        }

        // Add body
        if !body.is_null() {
            req = req.json(&body);
        }

        let resp = req.send().await.map_err(ToolError::Http)?;
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();

        Ok(serde_json::json!({
            "status": status.as_u16(),
            "body": text,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema() {
        let tool = WebhookTool::new();
        let schema = tool.input_schema();
        assert!(schema["properties"]["url"].is_object());
    }
}