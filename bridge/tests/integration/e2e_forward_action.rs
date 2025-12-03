use super::*;
use loom_core::tools::ToolResult as CoreToolResult;
use loom_core::{EventBus, Tool, ToolRegistry};

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
async fn test_forward_tool_call_echo() {
    let event_bus = Arc::new(EventBus::new().await.unwrap());
    let tool_registry = Arc::new(ToolRegistry::new());
    event_bus.start().await.unwrap();

    // Register test tool
    tool_registry.register(Arc::new(EchoTool)).await;

    let (addr, _handle, _svc) = start_test_server(event_bus.clone(), tool_registry.clone()).await;
    let mut client = new_client(addr).await;

    // Forward tool call
    let res = client
        .forward_tool_call(ToolCall {
            id: "t1".into(),
            name: "test.echo".into(),
            arguments: r#"{"message":"ping"}"#.into(),
            headers: Default::default(),
            timeout_ms: 1000,
            correlation_id: "c1".into(),
            qos: 0,
        })
        .await
        .unwrap()
        .into_inner();

    assert_eq!(res.status, ToolStatus::ToolOk as i32);
    assert!(res.output.contains("ping"));
}
