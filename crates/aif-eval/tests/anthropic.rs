use aif_eval::anthropic::{AnthropicClient, Message, Role, ApiError};

#[test]
fn client_requires_api_key() {
    let result = AnthropicClient::new("", "claude-sonnet-4-6", None);
    assert!(result.is_err());
    match result.unwrap_err() {
        ApiError::MissingApiKey => {}
        other => panic!("Expected MissingApiKey, got {:?}", other),
    }
}

#[test]
fn client_creates_with_valid_key() {
    let client = AnthropicClient::new("sk-test-123", "claude-sonnet-4-6", None).unwrap();
    assert_eq!(client.model(), "claude-sonnet-4-6");
}

#[test]
fn message_serialization() {
    let msg = Message {
        role: Role::User,
        content: "Hello".into(),
    };
    let json = serde_json::to_string(&msg).unwrap();
    assert!(json.contains("\"role\":\"user\""));
    assert!(json.contains("Hello"));
}

#[test]
fn request_body_format() {
    let client = AnthropicClient::new("sk-test", "claude-sonnet-4-6", None).unwrap();
    let body = client.build_request_body(
        Some("You are a helpful assistant."),
        &[Message {
            role: Role::User,
            content: "Test".into(),
        }],
        1024,
    );
    let parsed: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(parsed["model"], "claude-sonnet-4-6");
    assert_eq!(parsed["max_tokens"], 1024);
    assert!(parsed["system"].is_string());
    assert!(parsed["messages"].is_array());
}
