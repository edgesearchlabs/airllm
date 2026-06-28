use airllm_ollama::{chat_stream, Message};
use futures::StreamExt;

#[tokio::test]
async fn test_stream_parses_tokens() {
    let mut server = mockito::Server::new_async().await;

    // NDJSON: each line is a JSON object
    let stream_body = concat!(
        r#"{"message":{"role":"assistant","content":"Hello"},"done":false}"#, "\n",
        r#"{"message":{"role":"assistant","content":" world"},"done":false}"#, "\n",
        r#"{"message":{"role":"assistant","content":"!"},"done":false}"#, "\n",
        r#"{"done":true}"#, "\n"
    );

    server
        .mock("POST", "/api/chat")
        .with_status(200)
        .with_header("content-type", "application/x-ndjson")
        .with_body(stream_body)
        .create_async()
        .await;

    // We need to make the actual HTTP request and pass the response to chat_stream
    let http = reqwest::Client::new();
    let response = http
        .post(format!("{}/api/chat", server.url()))
        .json(&serde_json::json!({
            "model": "qwen3.6:27b",
            "messages": [{"role": "user", "content": "hi"}],
            "stream": true
        }))
        .send()
        .await
        .unwrap();

    let stream = chat_stream(response);
    let tokens: Vec<String> = stream
        .map(|r| r.expect("stream item should be ok"))
        .collect()
        .await;

    assert_eq!(tokens, vec!["Hello", " world", "!"]);
}

#[tokio::test]
async fn test_stream_empty_content_skipped() {
    let mut server = mockito::Server::new_async().await;

    let stream_body = concat!(
        r#"{"message":{"role":"assistant","content":""},"done":false}"#, "\n",
        r#"{"message":{"role":"assistant","content":"real"},"done":false}"#, "\n",
        r#"{"done":true}"#, "\n"
    );

    server
        .mock("POST", "/api/chat")
        .with_status(200)
        .with_body(stream_body)
        .create_async()
        .await;

    let http = reqwest::Client::new();
    let response = http
        .post(format!("{}/api/chat", server.url()))
        .json(&serde_json::json!({
            "model": "qwen3.6:27b",
            "messages": [{"role": "user", "content": "hi"}],
            "stream": true
        }))
        .send()
        .await
        .unwrap();

    let stream = chat_stream(response);
    let tokens: Vec<String> = stream
        .map(|r| r.expect("stream item should be ok"))
        .collect()
        .await;

    // Empty content should be skipped
    assert_eq!(tokens, vec!["real"]);
}

#[tokio::test]
async fn test_stream_handles_partial_chunks() {
    let mut server = mockito::Server::new_async().await;

    // Simulate a response where JSON lines might be split across chunks
    let stream_body = concat!(
        r#"{"message":{"content":"A"},"done":false}"#, "\n",
        r#"{"message":{"content":"B"},"done":false}"#, "\n",
        r#"{"done":true}"#, "\n"
    );

    server
        .mock("POST", "/api/chat")
        .with_status(200)
        .with_body(stream_body)
        .create_async()
        .await;

    let http = reqwest::Client::new();
    let response = http
        .post(format!("{}/api/chat", server.url()))
        .json(&serde_json::json!({
            "model": "qwen3.6:27b",
            "messages": [{"role": "user", "content": "hi"}],
            "stream": true
        }))
        .send()
        .await
        .unwrap();

    let stream = chat_stream(response);
    let tokens: Vec<String> = stream
        .map(|r| r.expect("stream item should be ok"))
        .collect()
        .await;

    assert_eq!(tokens, vec!["A", "B"]);
}

#[test]
fn test_message_for_stream() {
    let msg = Message::user("test prompt");
    assert_eq!(msg.content, "test prompt");
}