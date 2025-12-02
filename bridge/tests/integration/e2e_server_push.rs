use super::*;
use loom_core::{EventBus, ToolRegistry};
use tokio_stream::wrappers::ReceiverStream;

#[tokio::test]
async fn test_server_push_tool_call_and_client_reply() {
    let event_bus = Arc::new(EventBus::new().await.unwrap());
    let tool_registry = Arc::new(ToolRegistry::new());
    event_bus.start().await.unwrap();

    let (addr, _handle, svc) = start_test_server(event_bus.clone(), tool_registry.clone()).await;
    let mut client = new_client(addr).await;

    // Register agent
    let register_resp = client
        .register_agent(AgentRegisterRequest {
            agent_id: "agentA".into(),
            subscribed_topics: vec![],
            tools: vec![],
            metadata: Default::default(),
        })
        .await
        .unwrap()
        .into_inner();
    assert!(register_resp.success);

    // Open stream: queue Ack first to avoid deadlock
    let (tx_client, rx_stream) = tokio::sync::mpsc::channel(16);
    tx_client
        .send(ClientEvent {
            msg: Some(client_event::Msg::Ack(super::Ack {
                message_id: "agentA".into(),
            })),
        })
        .await
        .unwrap();
    let outbound = ReceiverStream::new(rx_stream);
    let mut inbound = client.event_stream(outbound).await.unwrap().into_inner();

    // Spawn task to read server->client events and respond to ToolCall
    let tx_clone = tx_client.clone();
    let reader = tokio::spawn(async move {
        use tokio::time::{timeout, Duration};
        // Wait up to 2s for the tool call then reply
        let _ = timeout(Duration::from_secs(2), async {
            while let Some(Ok(msg)) = inbound.message().await.transpose() {
                if let Some(server_event::Msg::ToolCall(call)) = msg.msg {
                    // Send back ToolResult
                    let _ = tx_clone
                        .send(ClientEvent {
                            msg: Some(client_event::Msg::ToolResult(ToolResult {
                                id: call.id.clone(),
                                status: ToolStatus::ToolOk as i32,
                                output: r#"{"result":"done"}"#.into(),
                                error: None,
                            })),
                        })
                        .await;
                    break;
                }
            }
        })
        .await;
    });

    // Push a tool call to the agent from the server side
    let call = ToolCall {
        id: "pushed1".into(),
        name: "agent.action".into(),
        arguments: "{}".into(),
        headers: Default::default(),
        timeout_ms: 1000,
        correlation_id: "pc1".into(),
        qos: 0,
    };
    let pushed = svc.push_tool_call("agentA", call).await.unwrap();
    assert!(pushed, "should have pushed to connected agent");

    // Wait for reader to complete
    let _ = reader.await;

    // Check the result was stored
    use tokio::time::{sleep, Duration};
    sleep(Duration::from_millis(100)).await;
    let result = svc.get_tool_result("pushed1");
    assert!(result.is_some(), "Expected stored tool result");
    let r = result.unwrap();
    assert_eq!(r.status, ToolStatus::ToolOk as i32);
}
