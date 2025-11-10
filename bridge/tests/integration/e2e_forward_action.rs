use super::*;
use loom_core::proto::{CapabilityDescriptor, ProviderKind};
use loom_core::{ActionBroker, CapabilityProvider, EventBus, Result as LoomResult};
use std::sync::Arc;

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

    async fn invoke(&self, call: super::ActionCall) -> LoomResult<super::ActionResult> {
        Ok(super::ActionResult {
            id: call.id,
            status: super::ActionStatus::ActionOk as i32,
            output: call.payload,
            error: None,
        })
    }
}

#[tokio::test]
async fn test_forward_action_echo() {
    let event_bus = Arc::new(EventBus::new().await.unwrap());
    let action_broker = Arc::new(ActionBroker::new());
    event_bus.start().await.unwrap();

    // Register test provider
    action_broker.register_provider(Arc::new(TestEchoProvider));

    let (addr, _handle) = start_test_server(event_bus.clone(), action_broker.clone()).await;
    let mut client = new_client(addr).await;

    // ForwardAction
    let payload = b"ping".to_vec();
    let res = client
        .forward_action(ActionCall {
            id: "act1".into(),
            capability: "test.echo".into(),
            version: "1.0".into(),
            payload: payload.clone(),
            headers: Default::default(),
            timeout_ms: 1000,
            correlation_id: "c1".into(),
            qos: 0,
        })
        .await
        .unwrap()
        .into_inner();

    assert_eq!(res.status, ActionStatus::ActionOk as i32);
    assert_eq!(res.output, payload);
}
