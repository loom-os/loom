use super::*;
use loom_core::{ActionBroker, EventBus};
use tokio_stream::wrappers::ReceiverStream;

#[tokio::test]
async fn test_register_and_event_roundtrip() {
    let event_bus = Arc::new(EventBus::new().await.unwrap());
    let action_broker = Arc::new(ActionBroker::new());
    event_bus.start().await.unwrap();

    let (addr, _handle, _svc) = start_test_server(event_bus.clone(), action_broker.clone()).await;
    let mut client = new_client(addr).await;

    // Register agent
    let register_resp = client
        .register_agent(AgentRegisterRequest {
            agent_id: "agentA".into(),
            subscribed_topics: vec!["topic.test".into()],
            capabilities: vec![],
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

    // Receive delivery (with some timeout)
    use tokio::time::{timeout, Duration};
    let delivery = timeout(Duration::from_secs(2), async {
        loop {
            if let Some(Ok(msg)) = rx.message().await.transpose() {
                if let Some(server_event::Msg::Delivery(d)) = msg.msg {
                    if let Some(ev) = d.event {
                        return Some(ev);
                    }
                }
            } else {
                break;
            }
        }
        None
    })
    .await
    .expect("timely event");

    assert!(delivery.is_some(), "expected echoed event delivered");
    let ev = delivery.unwrap();
    assert_eq!(ev.r#type, "test_input");
    assert_eq!(ev.payload, b"hello".to_vec());
}
