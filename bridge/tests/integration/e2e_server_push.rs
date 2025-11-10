use super::*;
use loom_core::{ActionBroker, EventBus};
use tokio_stream::wrappers::ReceiverStream;

#[tokio::test]
async fn test_server_push_action_and_client_reply() {
    let event_bus = Arc::new(EventBus::new().await.unwrap());
    let action_broker = Arc::new(ActionBroker::new());
    event_bus.start().await.unwrap();

    let (addr, _handle, svc) = start_test_server(event_bus.clone(), action_broker.clone()).await;
    let mut client = new_client(addr).await;

    // Register agent
    let register_resp = client
        .register_agent(AgentRegisterRequest {
            agent_id: "agentA".into(),
            subscribed_topics: vec![],
            capabilities: vec![],
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

    // Spawn task to read server->client events and respond to ActionCall
    let tx_clone = tx_client.clone();
    let reader = tokio::spawn(async move {
        use tokio::time::{timeout, Duration};
        let mut got_call = false;
        // Wait up to 2s for the action call then reply
        if let Ok(_) = timeout(Duration::from_secs(2), async {
            while let Some(Ok(msg)) = inbound.message().await.transpose() {
                if let Some(server_event::Msg::ActionCall(call)) = msg.msg {
                    // Send back ActionResult
                    let _ = tx_clone
                        .send(ClientEvent {
                            msg: Some(client_event::Msg::ActionResult(ActionResult {
                                id: call.id.clone(),
                                status: ActionStatus::ActionOk as i32,
                                output: b"done".to_vec(),
                                error: None,
                            })),
                        })
                        .await;
                    got_call = true;
                    break;
                }
            }
            if got_call {
                Ok(())
            } else {
                Err(())
            }
        })
        .await
        {
            let _ = _; // ignore
        }
    });

    // Server pushes action to agent
    let pushed = svc
        .push_action_call(
            "agentA",
            ActionCall {
                id: "push1".into(),
                capability: "noop".into(),
                version: "".into(),
                payload: vec![],
                headers: Default::default(),
                timeout_ms: 1000,
                correlation_id: "c1".into(),
                qos: 0,
            },
        )
        .await
        .unwrap();
    assert!(pushed, "should deliver to connected agent");

    // Wait a bit for result to arrive and be stored
    use tokio::time::{sleep, Duration};
    sleep(Duration::from_millis(200)).await;

    let res = svc.get_action_result("push1");
    assert!(res.is_some());
    let res = res.unwrap();
    assert_eq!(res.status, ActionStatus::ActionOk as i32);
    assert_eq!(res.output, b"done".to_vec());

    let _ = reader.await; // clean up
}
