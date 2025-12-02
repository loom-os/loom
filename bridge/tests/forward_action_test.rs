use loom_bridge::{BridgeService, BridgeState};
use loom_core::tools::ToolResult as CoreToolResult;
use loom_core::{AgentDirectory, EventBus, Tool, ToolRegistry};
use loom_proto::{bridge_server::Bridge, ToolCall, ToolStatus};
use std::sync::Arc;
use tonic::Request;

/// A simple echo tool for testing
struct EchoTool;

#[async_trait::async_trait]
impl Tool for EchoTool {
    fn name(&self) -> String {
        "test.echo".to_string()
    }

    fn description(&self) -> String {
        "Echoes back the input".to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "message": { "type": "string" }
            }
        })
    }

    async fn call(&self, arguments: serde_json::Value) -> CoreToolResult<serde_json::Value> {
        Ok(arguments)
    }
}

#[tokio::test]
async fn test_forward_tool_call_success() {
    let event_bus = Arc::new(EventBus::new().await.unwrap());
    let agent_directory = Arc::new(AgentDirectory::new());
    let tool_registry = Arc::new(ToolRegistry::new());

    // Register echo tool
    tool_registry.register(Arc::new(EchoTool)).await;

    let svc = BridgeService::new(BridgeState::new(event_bus, tool_registry, agent_directory));

    let req = ToolCall {
        id: "t1".into(),
        name: "test.echo".into(),
        arguments: r#"{"message":"hello"}"#.into(),
        headers: Default::default(),
        timeout_ms: 1000,
        correlation_id: "c1".into(),
        qos: 0,
    };

    let res = svc
        .forward_tool_call(Request::new(req))
        .await
        .unwrap()
        .into_inner();

    assert_eq!(res.status, ToolStatus::ToolOk as i32);
    assert!(res.output.contains("hello"));
}

#[tokio::test]
async fn test_forward_tool_call_not_found() {
    let event_bus = Arc::new(EventBus::new().await.unwrap());
    let agent_directory = Arc::new(AgentDirectory::new());
    let tool_registry = Arc::new(ToolRegistry::new());
    let svc = BridgeService::new(BridgeState::new(event_bus, tool_registry, agent_directory));

    let req = ToolCall {
        id: "t2".into(),
        name: "unknown.tool".into(),
        arguments: "{}".into(),
        headers: Default::default(),
        timeout_ms: 10,
        correlation_id: "c2".into(),
        qos: 0,
    };

    let res = svc
        .forward_tool_call(Request::new(req))
        .await
        .unwrap()
        .into_inner();

    assert_eq!(res.status, ToolStatus::ToolNotFound as i32);
    assert!(res.error.is_some());
    let err = res.error.unwrap();
    assert_eq!(err.code, "NOT_FOUND");
}
