use loom_bridge::{BridgeService, BridgeState};
use loom_core::{AgentDirectory, EventBus, ToolRegistry};
use loom_proto::{bridge_server::Bridge, AgentRegisterRequest};
use std::sync::Arc;
use tonic::Request;

#[tokio::test]
async fn test_register_agent_success() {
    let event_bus = Arc::new(EventBus::new().await.unwrap());
    let agent_directory = Arc::new(AgentDirectory::new());
    let tool_registry = Arc::new(ToolRegistry::new());
    let svc = BridgeService::new(BridgeState::new(event_bus, tool_registry, agent_directory));

    let resp = svc
        .register_agent(Request::new(AgentRegisterRequest {
            agent_id: "agent1".into(),
            subscribed_topics: vec!["topic.a".into(), "topic.b".into()],
            tools: vec![],
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
    let agent_directory = Arc::new(AgentDirectory::new());
    let tool_registry = Arc::new(ToolRegistry::new());
    let svc = BridgeService::new(BridgeState::new(event_bus, tool_registry, agent_directory));

    let resp = svc
        .register_agent(Request::new(AgentRegisterRequest {
            agent_id: "".into(),
            subscribed_topics: vec![],
            tools: vec![],
            metadata: Default::default(),
        }))
        .await
        .unwrap()
        .into_inner();

    assert!(!resp.success);
    assert!(resp.error_message.contains("agent_id"));
}
