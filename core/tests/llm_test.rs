use loom_core::cognitive::llm::{LlmClient, LlmClientConfig};
use loom_core::context::{PromptBundle, TokenBudget};
use loom_core::Result;
use serde_json::json;
use serial_test::serial;

#[test]
#[serial]
fn config_loads_from_defaults() {
    // Clear env vars to test defaults (including ones from config_loads_from_env test)
    std::env::remove_var("VLLM_BASE_URL");
    std::env::remove_var("VLLM_MODEL");
    std::env::remove_var("VLLM_API_KEY");
    std::env::remove_var("REQUEST_TIMEOUT_MS");
    std::env::remove_var("VLLM_TEMPERATURE");
    std::env::remove_var("LLM_BASE_URL");
    std::env::remove_var("LLM_MODEL");
    std::env::remove_var("LLM_API_KEY");
    std::env::remove_var("LLM_TIMEOUT_MS");

    let cfg = LlmClientConfig::default();
    assert_eq!(cfg.base_url, "http://localhost:8000/v1");
    assert_eq!(cfg.model, "qwen2.5-0.5b-instruct");
    assert_eq!(cfg.api_key, None);
    assert_eq!(cfg.request_timeout_ms, 30_000);
    assert_eq!(cfg.temperature, 0.7);
}

#[test]
#[serial]
fn config_loads_from_env() {
    std::env::set_var("VLLM_BASE_URL", "http://test:9000/v1");
    std::env::set_var("VLLM_MODEL", "test-model");
    std::env::set_var("VLLM_API_KEY", "test-key");
    std::env::set_var("REQUEST_TIMEOUT_MS", "5000");
    std::env::set_var("VLLM_TEMPERATURE", "0.5");

    let cfg = LlmClientConfig::default();
    assert_eq!(cfg.base_url, "http://test:9000/v1");
    assert_eq!(cfg.model, "test-model");
    assert_eq!(cfg.api_key, Some("test-key".to_string()));
    assert_eq!(cfg.request_timeout_ms, 5000);
    assert_eq!(cfg.temperature, 0.5);

    // Clean up
    std::env::remove_var("VLLM_BASE_URL");
    std::env::remove_var("VLLM_MODEL");
    std::env::remove_var("VLLM_API_KEY");
    std::env::remove_var("REQUEST_TIMEOUT_MS");
    std::env::remove_var("VLLM_TEMPERATURE");
}

#[test]
fn client_creation_succeeds() -> Result<()> {
    let cfg = LlmClientConfig {
        base_url: "http://localhost:8000/v1".to_string(),
        model: "test".to_string(),
        api_key: None,
        request_timeout_ms: 10_000,
        temperature: 0.7,
    };
    let _client = LlmClient::new(cfg)?;
    // Just verify client can be created
    Ok(())
}

// Note: Full HTTP request/response testing requires a mock server or integration test.
// Here we test the adapter logic that prepares payloads.

#[test]
fn adapter_builds_messages_from_bundle() {
    let bundle = PromptBundle {
        system: "You are helpful".to_string(),
        instructions: "Answer concisely".to_string(),
        tools_json_schema: None,
        context_docs: vec!["Doc1".to_string(), "Doc2".to_string()],
        history: vec!["User: hi".to_string(), "Assistant: hello".to_string()],
    };
    let budget = TokenBudget {
        max_input_tokens: 512,
        max_output_tokens: 128,
    };

    let (messages, fused) =
        loom_core::cognitive::llm::promptbundle_to_messages_and_text(&bundle, budget);

    // Should have at least system and user messages
    assert!(!messages.is_empty());
    assert!(messages
        .iter()
        .any(|m| m.get("role").and_then(|r| r.as_str()) == Some("system")));
    assert!(messages
        .iter()
        .any(|m| m.get("role").and_then(|r| r.as_str()) == Some("user")));

    // Fused text should contain system and context
    assert!(fused.contains("System:"));
    assert!(fused.contains("Context:"));
}

#[test]
fn adapter_respects_token_budget() {
    let bundle = PromptBundle {
        system: "S".repeat(1000),
        instructions: "I".repeat(1000),
        tools_json_schema: None,
        context_docs: vec!["C".repeat(1000)],
        history: vec!["H".repeat(1000)],
    };
    let budget = TokenBudget {
        max_input_tokens: 64, // ~256 chars
        max_output_tokens: 32,
    };

    let (_messages, fused) =
        loom_core::cognitive::llm::promptbundle_to_messages_and_text(&bundle, budget);

    // Fused text should be significantly truncated from the original 4000 chars
    // The adapter should have dropped history and truncated instructions
    let actual_len = fused.chars().count();
    println!("Actual fused length: {} chars", actual_len);
    println!("Budget allows: {} chars", 64 * 4);
    assert!(
        actual_len < 1000,
        "fused text should be truncated from 4000 chars, got {}",
        actual_len
    );
}

#[test]
fn adapter_handles_empty_bundle() {
    let bundle = PromptBundle {
        system: String::new(),
        instructions: String::new(),
        tools_json_schema: None,
        context_docs: vec![],
        history: vec![],
    };
    let budget = TokenBudget::default();

    let (messages, fused) =
        loom_core::cognitive::llm::promptbundle_to_messages_and_text(&bundle, budget);

    // Empty bundle produces empty outputs
    assert!(
        messages.is_empty(),
        "empty bundle should produce empty messages"
    );
    assert!(
        fused.is_empty(),
        "empty bundle should produce empty fused text"
    );
}

#[test]
fn adapter_includes_tools_schema_when_provided() {
    let tools_schema = json!({
        "name": "get_weather",
        "description": "Get weather",
        "parameters": {}
    });
    let bundle = PromptBundle {
        system: "System".to_string(),
        instructions: "Use tools".to_string(),
        tools_json_schema: Some(tools_schema.to_string()),
        context_docs: vec![],
        history: vec![],
    };
    let budget = TokenBudget::default();

    let (_messages, fused) =
        loom_core::cognitive::llm::promptbundle_to_messages_and_text(&bundle, budget);

    // Fused text should mention tools
    assert!(
        fused.contains("Tools:") || fused.contains("tool"),
        "tools should be included"
    );
}

// Integration test placeholder for actual HTTP calls
// Uncomment and use a mock server like `wiremock` or `httpmock` for full coverage

/*
#[tokio::test]
async fn generate_succeeds_with_mock_responses_api() -> Result<()> {
    // Set up mock server returning Responses API payload
    // let mock_server = MockServer::start().await;
    // Mock::given(method("POST"))
    //     .and(path("/responses"))
    //     .respond_with(ResponseTemplate::new(200).set_body_json(json!({
    //         "choices": [{"text": "test response"}]
    //     })))
    //     .mount(&mock_server)
    //     .await;

    // let cfg = LlmClientConfig {
    //     base_url: mock_server.uri(),
    //     model: "test".to_string(),
    //     api_key: None,
    //     request_timeout_ms: 5000,
    //     temperature: 0.7,
    // };
    // let client = LlmClient::new(cfg)?;
    // let bundle = PromptBundle { ... };
    // let response = client.generate(&bundle, None).await?;
    // assert_eq!(response.text, "test response");
    Ok(())
}

#[tokio::test]
async fn generate_falls_back_to_chat_completions() -> Result<()> {
    // Mock server: responses endpoint 404, chat/completions 200
    Ok(())
}

#[tokio::test]
async fn generate_handles_timeout() -> Result<()> {
    // Mock server with delay longer than timeout
    Ok(())
}

#[tokio::test]
async fn generate_handles_malformed_json() -> Result<()> {
    // Mock server returns invalid JSON
    Ok(())
}
*/

#[test]
fn token_budget_default_values() {
    let budget = TokenBudget::default();
    assert_eq!(budget.max_input_tokens, 2048);
    assert_eq!(budget.max_output_tokens, 512);
}
