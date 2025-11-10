use async_trait::async_trait;
use loom_core::action_broker::{ActionBroker, CapabilityProvider};
use loom_core::context::PromptBundle;
use loom_core::llm::{
    build_action_call, make_refine_bundle, parse_tool_calls_from_chat,
    parse_tool_calls_from_responses, NormalizedToolCall,
};
use loom_core::proto::{ActionCall, ActionResult, ActionStatus, CapabilityDescriptor};
use loom_core::Result;
use serde_json::json;
use std::sync::Arc;

struct EchoProvider;

#[async_trait]
impl CapabilityProvider for EchoProvider {
    fn descriptor(&self) -> CapabilityDescriptor {
        CapabilityDescriptor {
            name: "unit.echo".into(),
            version: "0.1.0".into(),
            provider: loom_core::proto::ProviderKind::ProviderNative as i32,
            metadata: Default::default(),
        }
    }

    async fn invoke(&self, call: ActionCall) -> Result<ActionResult> {
        Ok(ActionResult {
            id: call.id,
            status: ActionStatus::ActionOk as i32,
            output: call.payload,
            error: None,
        })
    }
}

#[test]
fn test_parse_responses_tool_use() {
    let resp = json!({
        "output": [
            {"content": [
                {"type":"tool_use","name":"unit.echo","id":"call_1","input":{"msg":"hello"}}
            ]}
        ],
        "output_text": ""
    });
    let calls = parse_tool_calls_from_responses(&resp);
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].name, "unit.echo");
    assert_eq!(calls[0].arguments["msg"], "hello");
}

#[test]
fn test_parse_responses_multiple_tool_calls() {
    let resp = json!({
        "output": [
            {"content": [
                {"type":"tool_use","name":"web.search","id":"call_1","input":{"query":"rust"}},
                {"type":"tool_use","name":"weather.get","id":"call_2","input":{"location":"Beijing"}}
            ]}
        ],
        "output_text": ""
    });
    let calls = parse_tool_calls_from_responses(&resp);
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0].name, "web.search");
    assert_eq!(calls[1].name, "weather.get");
    assert_eq!(calls[0].arguments["query"], "rust");
    assert_eq!(calls[1].arguments["location"], "Beijing");
}

#[test]
fn test_parse_responses_no_tool_calls() {
    let resp = json!({
        "output": [
            {"content": [
                {"type":"text","text":"Just regular text"}
            ]}
        ],
        "output_text": "Just regular text"
    });
    let calls = parse_tool_calls_from_responses(&resp);
    assert_eq!(calls.len(), 0);
}

#[test]
fn test_parse_chat_tool_calls() {
    let chat = json!({
        "choices": [
            {"message": {"tool_calls": [
                {"id":"tool_1","function": {"name":"unit.echo","arguments":"{\"msg\":\"hi\"}"}}
            ]}}
        ]
    });
    let calls = parse_tool_calls_from_chat(&chat);
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].name, "unit.echo");
    assert_eq!(calls[0].arguments["msg"], "hi");
}

#[test]
fn test_parse_chat_multiple_tool_calls() {
    let chat = json!({
        "choices": [
            {"message": {"tool_calls": [
                {"id":"tool_1","function": {"name":"web.search","arguments":"{\"query\":\"weather\"}"}},
                {"id":"tool_2","function": {"name":"weather.get","arguments":"{\"location\":\"SF\"}"}}
            ]}}
        ]
    });
    let calls = parse_tool_calls_from_chat(&chat);
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0].name, "web.search");
    assert_eq!(calls[1].name, "weather.get");
}

#[test]
fn test_parse_chat_malformed_json_arguments() {
    let chat = json!({
        "choices": [
            {"message": {"tool_calls": [
                {"id":"tool_1","function": {"name":"unit.echo","arguments":"not valid json"}}
            ]}}
        ]
    });
    let calls = parse_tool_calls_from_chat(&chat);
    // Should handle malformed JSON gracefully by using empty object
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].name, "unit.echo");
    assert_eq!(calls[0].arguments, json!({}));
}

#[test]
fn test_parse_chat_no_tool_calls() {
    let chat = json!({
        "choices": [
            {"message": {"content": "Just a regular response"}}
        ]
    });
    let calls = parse_tool_calls_from_chat(&chat);
    assert_eq!(calls.len(), 0);
}

#[tokio::test]
async fn test_build_and_invoke_action_call() -> Result<()> {
    let broker = Arc::new(ActionBroker::new());
    broker.register_provider(Arc::new(EchoProvider));

    let call = NormalizedToolCall {
        id: None,
        name: "unit.echo".into(),
        arguments: json!({"k": 1}),
    };
    let action = build_action_call(&call, 1000, Some("cid-1".into()));

    assert_eq!(action.capability, "unit.echo");
    assert_eq!(action.timeout_ms, 1000);
    assert_eq!(
        action.headers.get("correlation_id").map(|s| s.as_str()),
        Some("cid-1")
    );

    let res = broker.invoke(action).await?;
    assert_eq!(res.status, ActionStatus::ActionOk as i32);
    Ok(())
}

#[tokio::test]
async fn test_action_call_without_correlation_id() -> Result<()> {
    let broker = Arc::new(ActionBroker::new());
    broker.register_provider(Arc::new(EchoProvider));

    let call = NormalizedToolCall {
        id: Some("test_id".into()),
        name: "unit.echo".into(),
        arguments: json!({"data": "test"}),
    };
    let action = build_action_call(&call, 5000, None);

    assert_eq!(action.capability, "unit.echo");
    assert_eq!(action.timeout_ms, 5000);
    assert!(action.headers.get("correlation_id").is_none());

    let res = broker.invoke(action).await?;
    assert_eq!(res.status, ActionStatus::ActionOk as i32);
    Ok(())
}

#[test]
fn test_refine_bundle_contains_results() {
    let base = PromptBundle::default();
    let calls = vec![NormalizedToolCall {
        id: None,
        name: "unit.echo".into(),
        arguments: json!({}),
    }];
    let ok = ActionResult {
        id: "a".into(),
        status: ActionStatus::ActionOk as i32,
        output: b"result".to_vec(),
        error: None,
    };
    let bundle = make_refine_bundle(&base, &calls, &[ok]);

    assert!(bundle.system.contains("Tool Results:"));
    assert!(bundle.system.contains("unit.echo"));
}

#[test]
fn test_refine_bundle_with_existing_system() {
    let base = PromptBundle {
        system: "You are a helpful assistant".into(),
        instructions: "Answer the question".into(),
        tools_json_schema: None,
        context_docs: vec![],
        history: vec![],
    };
    let calls = vec![NormalizedToolCall {
        id: None,
        name: "web.search".into(),
        arguments: json!({"query":"test"}),
    }];
    let ok = ActionResult {
        id: "a".into(),
        status: ActionStatus::ActionOk as i32,
        output: b"search results".to_vec(),
        error: None,
    };
    let bundle = make_refine_bundle(&base, &calls, &[ok]);

    // Should preserve original system and append tool results
    assert!(bundle.system.contains("You are a helpful assistant"));
    assert!(bundle.system.contains("Tool Results:"));
    assert!(bundle.system.contains("web.search"));
}

#[test]
fn test_refine_bundle_with_error_result() {
    let base = PromptBundle::default();
    let calls = vec![NormalizedToolCall {
        id: None,
        name: "weather.get".into(),
        arguments: json!({"location":"test"}),
    }];
    let error = ActionResult {
        id: "a".into(),
        status: ActionStatus::ActionError as i32,
        output: vec![],
        error: Some(loom_core::proto::ActionError {
            code: "TIMEOUT".into(),
            message: "Request timed out".into(),
            details: Default::default(),
        }),
    };
    let bundle = make_refine_bundle(&base, &calls, &[error]);

    assert!(bundle.system.contains("Tool Results:"));
    assert!(bundle.system.contains("weather.get"));
    assert!(bundle.system.contains("TIMEOUT"));
}

#[test]
fn test_refine_bundle_truncates_many_results() {
    let base = PromptBundle::default();
    let mut calls = vec![];
    let mut results = vec![];

    // Create 20 tool calls (more than the 8 limit)
    for i in 0..20 {
        calls.push(NormalizedToolCall {
            id: None,
            name: format!("tool_{}", i),
            arguments: json!({}),
        });
        results.push(ActionResult {
            id: format!("id_{}", i),
            status: ActionStatus::ActionOk as i32,
            output: b"ok".to_vec(),
            error: None,
        });
    }

    let bundle = make_refine_bundle(&base, &calls, &results);

    // Should contain first few tools
    assert!(bundle.system.contains("tool_0"));
    assert!(bundle.system.contains("tool_5"));
    assert!(bundle.system.contains("tool_8"));
    // But not all 20
    assert!(!bundle.system.contains("tool_15"));
}
