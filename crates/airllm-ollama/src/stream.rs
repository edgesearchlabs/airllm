use futures::Stream;
use serde::Deserialize;
use tokio_stream::StreamExt;

use crate::error::{OllamaError, Result};

/// A single chunk in the Ollama streaming response.
#[derive(Deserialize)]
struct StreamChunk {
    message: Option<StreamMessage>,
    done: bool,
}

#[derive(Deserialize)]
struct StreamMessage {
    content: Option<String>,
}

/// Wraps the raw `reqwest::Response` bytes stream into a stream of
/// token strings, parsing each NDJSON line.
pub fn chat_stream(
    response: reqwest::Response,
) -> impl Stream<Item = Result<String>> {
    async_stream::try_stream! {
        let mut bytes = response.bytes_stream();
        let mut buffer = String::new();

        while let Some(chunk_result) = bytes.next().await {
            let chunk = chunk_result.map_err(OllamaError::Request)?;
            buffer.push_str(&String::from_utf8_lossy(&chunk));

            // Process complete lines (NDJSON — each line is a JSON object)
            while let Some(newline_pos) = buffer.find('\n') {
                let line = buffer[..newline_pos].trim().to_string();
                buffer = buffer[newline_pos + 1..].to_string();

                if line.is_empty() {
                    continue;
                }

                let parsed: StreamChunk = serde_json::from_str(&line)
                    .map_err(|e| OllamaError::StreamParse(e.to_string()))?;

                if parsed.done {
                    return;
                }

                if let Some(msg) = parsed.message {
                    if let Some(content) = msg.content {
                        if !content.is_empty() {
                            yield content;
                        }
                    }
                }
            }
        }
    }
}