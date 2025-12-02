use super::*;
use loom_core::{EventBus, ToolRegistry};
use tokio_stream::wrappers::ReceiverStream;

#[tokio::test]
async fn test_register_and_event_roundtrip() {
    let event_bus = Arc::new(EventBus::new().await.unwrap());
    let tool_registry = Arc::new(ToolRegistry::new());
    event_bus.start().await.unwrap();

    let (addr, _handle, _svc) = start_test_server(event_bus.clone(), tool_registry.clone()).await;
    let mut client = new_client(addr).await;

    // Register agent
    let register_resp = client
        .register_agent(AgentRegisterRequest {
            agent_id: "agentA".into(),
            subscribed_topics: vec!["topic.test".into()],
            tools: vec![],
            metadata: Default::default(),
        })
        .await
        .unwrap()
        .into_inner();
    assert!(register_resp.success);

    // Open stream: prepare outbound and send first Ack BEFORE awaiting server response to avoid deadlock
    let (tx_client, rx_stream) = tokio::sync::mpsc::channel(16);
    // First message Ack with agent_id handshake (queued before call)
    tx_client
        .send(ClientEvent {
            msg: Some(client_event::Msg::Ack(super::Ack {
                message_id: "agentA".into(),
            })),
        })
        .await
        .unwrap();
    let outbound = ReceiverStream::new(rx_stream);
    let mut rx = client.event_stream(outbound).await.unwrap().into_inner();

    // Publish an event
    tx_client
        .send(ClientEvent {
            msg: Some(client_event::Msg::Publish(Publish {
                topic: "topic.test".into(),
                event: Some(Event {
                    id: "ev1".into(),
                    r#type: "test_input".into(),
                    timestamp_ms: 0,
                    source: "tester".into(),
                    metadata: Default::default(),
                    payload: b"hello".to_vec(),
                    confidence: 1.0,
                    tags: vec![],
                    priority: 50,
                }),
            })),
        })
        .await
        .unwrap();

    // Receive the event back via subscription
    use tokio::time::{timeout, Duration};
    let recv_msg = timeout(Duration::from_secs(2), rx.message())
        .await
        .expect("recv timed out")
        .unwrap()
        .unwrap();

    if let Some(server_event::Msg::Delivery(del)) = recv_msg.msg {
        assert_eq!(del.topic, "topic.test");
        assert_eq!(del.event.as_ref().unwrap().id, "ev1");
    } else {
        panic!("Expected Delivery");
    }
}
