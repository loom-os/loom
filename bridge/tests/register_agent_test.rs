use loom_bridge::{BridgeService, BridgeState};
use loom_core::{ActionBroker, EventBus};
use loom_proto::{bridge_server::Bridge, AgentRegisterRequest};
use std::sync::Arc;
use tonic::Request;

#[tokio::test]
async fn test_register_agent_success() {
    let event_bus = Arc::new(EventBus::new().await.unwrap());
    let action_broker = Arc::new(ActionBroker::new());
    let svc = BridgeService::new(BridgeState::new(event_bus, action_broker));

    let resp = svc
        .register_agent(Request::new(AgentRegisterRequest {
            agent_id: "agent1".into(),
            subscribed_topics: vec!["topic.a".into(), "topic.b".into()],
            capabilities: vec![],
            metadata: Default::default(),
        }))
        .await
        .unwrap()
        .into_inner();

    assert!(resp.success);
}

#[tokio::test]
async fn test_register_agent_empty_id() {
    let event_bus = Arc::new(EventBus::new().await.unwrap());
    let action_broker = Arc::new(ActionBroker::new());
    let svc = BridgeService::new(BridgeState::new(event_bus, action_broker));

    let resp = svc
        .register_agent(Request::new(AgentRegisterRequest {
            agent_id: "".into(),
            subscribed_topics: vec![],
            capabilities: vec![],
            metadata: Default::default(),
        }))
        .await
        .unwrap()
        .into_inner();

    assert!(!resp.success);
    assert!(resp.error_message.contains("agent_id"));
}
