use airllm_ollama::{ChatOptions, Message, MessageRole, OllamaClient};
use mockito::Matcher;

#[tokio::test]
async fn test_chat_success() {
    let mut server = mockito::Server::new_async().await;

    let mock_response = serde_json::json!({
        "message": {
            "role": "assistant",
            "content": "Hello from the model!"
        },
        "done": true
    });

    server
        .mock("POST", "/api/chat")
        .match_body(Matcher::Regex(r#""keep_alive":"30m""#.to_string()))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(mock_response.to_string())
        .create_async()
        .await;

    let client = OllamaClient::new(&server.url());
    let messages = vec![
        Message::system("You are helpful."),
        Message::user("Say hello."),
    ];

    let result = client
        .chat("qwen3.6:27b", &messages, ChatOptions::default())
        .await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "Hello from the model!");
}

#[tokio::test]
async fn test_list_models_uses_cache() {
    let mut server = mockito::Server::new_async().await;

    let mock_response = serde_json::json!({
        "models": [
            {
                "name": "qwen3.5:4b",
                "size": 3650720256u64,
                "details": {
                    "quantization_level": "Q4_K_M"
                }
            }
        ]
    });

    let mock = server
        .mock("GET", "/api/tags")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(mock_response.to_string())
        .expect(1)
        .create_async()
        .await;

    let client = OllamaClient::new(&server.url());
    let first = client.list_models().await.expect("first models call");
    let second = client.list_models().await.expect("second models call");

    assert_eq!(first.len(), 1);
    assert_eq!(second.len(), 1);
    mock.assert_async().await;
}

#[tokio::test]
async fn test_chat_model_not_found() {
    let mut server = mockito::Server::new_async().await;

    server
        .mock("POST", "/api/chat")
        .with_status(404)
        .with_body("model not found")
        .create_async()
        .await;

    let client = OllamaClient::new(&server.url());
    let messages = vec![Message::user("test")];

    let result = client
        .chat("nonexistent-model", &messages, ChatOptions::default())
        .await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(
        err,
        airllm_ollama::OllamaError::ModelNotFound(_)
    ));
}

#[tokio::test]
async fn test_chat_http_error() {
    let mut server = mockito::Server::new_async().await;

    server
        .mock("POST", "/api/chat")
        .with_status(500)
        .with_body("internal server error")
        .create_async()
        .await;

    let client = OllamaClient::new(&server.url());
    let messages = vec![Message::user("test")];

    let result = client
        .chat("qwen3.6:27b", &messages, ChatOptions::default())
        .await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(
        err,
        airllm_ollama::OllamaError::Http { .. }
    ));
}

#[tokio::test]
async fn test_list_models() {
    let mut server = mockito::Server::new_async().await;

    let mock_response = serde_json::json!({
        "models": [
            {
                "name": "qwen3.5:4b",
                "size": 3650720256u64,
                "details": {
                    "quantization_level": "Q4_K_M"
                }
            },
            {
                "name": "qwen3.6:27b",
                "size": 18253611008u64,
                "details": {
                    "quantization_level": "Q4_K_M"
                }
            }
        ]
    });

    server
        .mock("GET", "/api/tags")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(mock_response.to_string())
        .create_async()
        .await;

    let client = OllamaClient::new(&server.url());
    let models = client.list_models().await;

    assert!(models.is_ok());
    let models = models.unwrap();
    assert_eq!(models.len(), 2);
    assert_eq!(models[0].name, "qwen3.5:4b");
    assert_eq!(models[0].quantization, "Q4_K_M");
    assert_eq!(models[1].name, "qwen3.6:27b");
}

#[tokio::test]
async fn test_model_available() {
    let mut server = mockito::Server::new_async().await;

    let mock_response = serde_json::json!({
        "models": [
            {
                "name": "qwen3.6:27b",
                "size": 18253611008u64,
                "details": {
                    "quantization_level": "Q4_K_M"
                }
            }
        ]
    });

    server
        .mock("GET", "/api/tags")
        .with_status(200)
        .with_body(mock_response.to_string())
        .create_async()
        .await;

    let client = OllamaClient::new(&server.url());

    assert!(client.model_available("qwen3.6:27b").await.unwrap());
    assert!(!client.model_available("nonexistent").await.unwrap());
}

#[tokio::test]
async fn test_prewarm_model() {
    let mut server = mockito::Server::new_async().await;

    server
        .mock("POST", "/api/generate")
        .match_body(Matcher::Regex(r#""keep_alive":"30m""#.to_string()))
        .match_body(Matcher::Regex(r#""num_predict":0"#.to_string()))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"response":"","done":true}"#)
        .create_async()
        .await;

    let client = OllamaClient::new(&server.url());
    client.prewarm_model("qwen3.5:4b").await.expect("prewarm should succeed");
}

#[test]
fn test_message_constructors() {
    let sys = Message::system("You are helpful.");
    assert_eq!(sys.role, MessageRole::System);
    assert_eq!(sys.content, "You are helpful.");

    let user = Message::user("Hello.");
    assert_eq!(user.role, MessageRole::User);

    let asst = Message::assistant("Hi there!");
    assert_eq!(asst.role, MessageRole::Assistant);
}

#[test]
fn test_chat_options_default() {
    let opts = ChatOptions::default();
    assert_eq!(opts.temperature, 0.7);
    assert_eq!(opts.top_p, 0.9);
    assert_eq!(opts.top_k, 40);
    assert_eq!(opts.num_ctx, 4096);
}