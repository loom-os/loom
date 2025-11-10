use loom_bridge::{BridgeService, BridgeState};
use loom_core::proto::{CapabilityDescriptor, ProviderKind};
use loom_core::{ActionBroker, CapabilityProvider, EventBus, Result as LoomResult};
use loom_proto::{bridge_server::Bridge, ActionCall, ActionResult, ActionStatus};
use std::sync::Arc;
use tonic::Request;

struct TestEchoProvider;

#[async_trait::async_trait]
impl CapabilityProvider for TestEchoProvider {
    fn descriptor(&self) -> CapabilityDescriptor {
        CapabilityDescriptor {
            name: "test.echo".into(),
            version: "1.0".into(),
            provider: ProviderKind::ProviderNative as i32,
            metadata: Default::default(),
        }
    }

    async fn invoke(&self, call: ActionCall) -> LoomResult<ActionResult> {
        Ok(ActionResult {
            id: call.id,
            status: ActionStatus::ActionOk as i32,
            output: call.payload,
            error: None,
        })
    }
}

#[tokio::test]
async fn test_forward_action_success() {
    let event_bus = Arc::new(EventBus::new().await.unwrap());
    let action_broker = Arc::new(ActionBroker::new());
    action_broker.register_provider(Arc::new(TestEchoProvider));
    let svc = BridgeService::new(BridgeState::new(event_bus, action_broker));

    let req = ActionCall {
        id: "a1".into(),
        capability: "test.echo".into(),
        version: "1.0".into(),
        payload: b"hello".to_vec(),
        headers: Default::default(),
        timeout_ms: 1000,
        correlation_id: "c1".into(),
        qos: 0,
    };

    let res = svc
        .forward_action(Request::new(req))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(res.status, ActionStatus::ActionOk as i32);
    assert_eq!(res.output, b"hello".to_vec());
}

#[tokio::test]
async fn test_forward_action_missing_capability() {
    let event_bus = Arc::new(EventBus::new().await.unwrap());
    let action_broker = Arc::new(ActionBroker::new());
    let svc = BridgeService::new(BridgeState::new(event_bus, action_broker));

    let req = ActionCall {
        id: "a2".into(),
        capability: "unknown.cap".into(),
        version: "1.0".into(),
        payload: vec![],
        headers: Default::default(),
        timeout_ms: 10,
        correlation_id: "c2".into(),
        qos: 0,
    };

    let res = svc
        .forward_action(Request::new(req))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(res.status, ActionStatus::ActionError as i32);
    assert!(res.error.is_some());
    let err = res.error.unwrap();
    assert_eq!(err.code, "BROKER_ERROR");
}
