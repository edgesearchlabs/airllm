//! Email tool — send emails via SMTP.

use async_trait::async_trait;
use serde_json::Value;

use crate::{Tool, ToolError, ToolResult};

pub struct EmailTool;

impl EmailTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for EmailTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for EmailTool {
    fn name(&self) -> &str {
        "send_email"
    }

    fn description(&self) -> &str {
        "Send an email via SMTP (requires SMTP_HOST, SMTP_USER, SMTP_PASS env vars)"
    }

    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "to": {"type": "string", "description": "Recipient email address"},
                "subject": {"type": "string", "description": "Email subject"},
                "body": {"type": "string", "description": "Email body (plain text)"},
                "from": {"type": "string", "description": "Sender email (optional, defaults to SMTP_USER)"}
            },
            "required": ["to", "subject", "body"]
        })
    }

    async fn execute(&self, args: Value) -> ToolResult<Value> {
        let to = args
            .get("to")
            .and_then(Value::as_str)
            .ok_or_else(|| ToolError::InvalidArgs("to is required".into()))?;
        let subject = args
            .get("subject")
            .and_then(Value::as_str)
            .ok_or_else(|| ToolError::InvalidArgs("subject is required".into()))?;
        let body = args
            .get("body")
            .and_then(Value::as_str)
            .ok_or_else(|| ToolError::InvalidArgs("body is required".into()))?;

        let smtp_host = std::env::var("SMTP_HOST").unwrap_or_default();
        let smtp_user = std::env::var("SMTP_USER").unwrap_or_default();
        let _smtp_pass = std::env::var("SMTP_PASS").unwrap_or_default();
        let from = args
            .get("from")
            .and_then(Value::as_str)
            .unwrap_or(&smtp_user);

        if smtp_host.is_empty() || smtp_user.is_empty() {
            return Err(ToolError::InvalidArgs(
                "SMTP_HOST and SMTP_USER env vars required".into(),
            ));
        }

        // In production, this would use the `lettre` crate to send via SMTP.
        // For now, we prepare the email and return a simulated result.
        tracing::info!(
            to,
            from,
            subject_len = subject.len(),
            body_len = body.len(),
            "email prepared for sending"
        );

        Ok(serde_json::json!({
            "status": "prepared",
            "to": to,
            "from": from,
            "subject": subject,
            "body_length": body.len(),
            "smtp_host": smtp_host,
            "note": "In production, this would send via SMTP using the lettre crate."
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_missing_to() {
        let tool = EmailTool::new();
        let result = tool
            .execute(serde_json::json!({"subject": "test", "body": "hello"}))
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_no_smtp_config() {
        let tool = EmailTool::new();
        // Ensure env vars are not set
        std::env::remove_var("SMTP_HOST");
        std::env::remove_var("SMTP_USER");
        let result = tool
            .execute(serde_json::json!({
                "to": "test@example.com",
                "subject": "test",
                "body": "hello"
            }))
            .await;
        assert!(result.is_err());
    }
}