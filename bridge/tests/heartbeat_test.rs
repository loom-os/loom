use loom_bridge::{ActionBroker, BridgeService, BridgeState};
use loom_core::{AgentDirectory, EventBus};
use loom_proto::{bridge_server::Bridge, HeartbeatRequest};
use std::sync::Arc;
use tonic::Request;

#[tokio::test]
async fn test_heartbeat_ok() {
    let event_bus = Arc::new(EventBus::new().await.unwrap());
    let agent_directory = Arc::new(AgentDirectory::new());
    let action_broker = Arc::new(ActionBroker::new());
    let svc = BridgeService::new(BridgeState::new(event_bus, action_broker, agent_directory));

    let ts = 12345;
    let resp = svc
        .heartbeat(Request::new(HeartbeatRequest { timestamp_ms: ts }))
        .await
        .unwrap()
        .into_inner();

    assert_eq!(resp.status, "ok");
    assert_eq!(resp.timestamp_ms, ts);
}
