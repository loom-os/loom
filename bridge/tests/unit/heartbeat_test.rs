use loom_bridge::{BridgeService, BridgeState};
use loom_core::{ActionBroker, EventBus};
use loom_proto::{bridge_server::Bridge, HeartbeatRequest};
use std::sync::Arc;
use tonic::Request;

#[tokio::test]
async fn test_heartbeat_ok() {
    let event_bus = Arc::new(EventBus::new().await.unwrap());
    let action_broker = Arc::new(ActionBroker::new());
    let svc = BridgeService::new(BridgeState::new(event_bus, action_broker));

    let ts = 12345;
    let resp = svc
        .heartbeat(Request::new(HeartbeatRequest { timestamp_ms: ts }))
        .await
        .unwrap()
        .into_inner();

    assert_eq!(resp.status, "ok");
    assert_eq!(resp.timestamp_ms, ts);
}
