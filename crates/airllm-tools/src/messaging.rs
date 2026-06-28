//! Messaging tool — send messages via Telegram, Slack, Discord.

use async_trait::async_trait;
use serde_json::Value;

use crate::{Tool, ToolError, ToolResult};

pub struct MessagingTool {
    http: reqwest::Client,
}

impl MessagingTool {
    pub fn new() -> Self {
        Self {
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap_or_default(),
        }
    }
}

impl Default for MessagingTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for MessagingTool {
    fn name(&self) -> &str {
        "send_message"
    }

    fn description(&self) -> &str {
        "Send a message via Telegram, Slack, or Discord webhook"
    }

    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "platform": {"type": "string", "enum": ["telegram", "slack", "discord"]},
                "message": {"type": "string", "description": "Message text to send"},
                "channel": {"type": "string", "description": "Target channel/chat ID (optional for webhook-based)"},
                "webhook_url": {"type": "string", "description": "Webhook URL (Slack/Discord). If omitted, uses TELEGRAM_BOT_TOKEN env for Telegram."},
                "bot_token": {"type": "string", "description": "Bot token (from env: TELEGRAM_BOT_TOKEN)"}
            },
            "required": ["platform", "message"]
        })
    }

    async fn execute(&self, args: Value) -> ToolResult<Value> {
        let platform = args
            .get("platform")
            .and_then(Value::as_str)
            .ok_or_else(|| ToolError::InvalidArgs("platform is required".into()))?;
        let message = args
            .get("message")
            .and_then(Value::as_str)
            .ok_or_else(|| ToolError::InvalidArgs("message is required".into()))?;

        match platform {
            "telegram" => {
                let token = args
                    .get("bot_token")
                    .and_then(Value::as_str)
                    .map(String::from)
                    .or_else(|| std::env::var("TELEGRAM_BOT_TOKEN").ok())
                    .ok_or_else(|| {
                        ToolError::InvalidArgs(
                            "bot_token required (arg or TELEGRAM_BOT_TOKEN env)".into(),
                        )
                    })?;
                let chat_id = args
                    .get("channel")
                    .and_then(Value::as_str)
                    .ok_or_else(|| ToolError::InvalidArgs("channel (chat_id) required for Telegram".into()))?;

                let url = format!("https://api.telegram.org/bot{token}/sendMessage");
                let resp = self
                    .http
                    .post(&url)
                    .json(&serde_json::json!({
                        "chat_id": chat_id,
                        "text": message,
                    }))
                    .send()
                    .await
                    .map_err(ToolError::Http)?;

                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                Ok(serde_json::json!({
                    "platform": "telegram",
                    "status": status.as_u16(),
                    "response": body,
                }))
            }
            "slack" | "discord" => {
                let webhook_url = args
                    .get("webhook_url")
                    .and_then(Value::as_str)
                    .ok_or_else(|| {
                        ToolError::InvalidArgs("webhook_url required for Slack/Discord".into())
                    })?;

                let payload = if platform == "slack" {
                    serde_json::json!({"text": message})
                } else {
                    serde_json::json!({"content": message})
                };

                let resp = self
                    .http
                    .post(webhook_url)
                    .json(&payload)
                    .send()
                    .await
                    .map_err(ToolError::Http)?;

                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                Ok(serde_json::json!({
                    "platform": platform,
                    "status": status.as_u16(),
                    "response": body,
                }))
            }
            other => Err(ToolError::InvalidArgs(format!(
                "unsupported platform: {other}"
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_missing_platform() {
        let tool = MessagingTool::new();
        let result = tool
            .execute(serde_json::json!({"message": "hello"}))
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_unsupported_platform() {
        let tool = MessagingTool::new();
        let result = tool
            .execute(serde_json::json!({"platform": "whatsapp", "message": "hi"}))
            .await;
        assert!(result.is_err());
    }
}