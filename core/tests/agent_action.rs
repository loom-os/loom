use async_trait::async_trait;
use loom_core::action_broker::CapabilityProvider;
use loom_core::agent::AgentBehavior;
use loom_core::proto::{
    ActionCall, ActionResult, ActionStatus, AgentConfig, CapabilityDescriptor, Event as ProtoEvent,
    ProviderKind,
};
use loom_core::{Loom, Result};
use std::sync::Arc;
use tokio::sync::mpsc;

struct TestEchoProvider {
    tx: mpsc::Sender<String>,
}

#[async_trait]
impl CapabilityProvider for TestEchoProvider {
    fn descriptor(&self) -> CapabilityDescriptor {
        CapabilityDescriptor {
            name: "test.echo".into(),
            version: "0.1.0".into(),
            provider: ProviderKind::ProviderNative as i32,
            metadata: Default::default(),
        }
    }

    async fn invoke(&self, call: ActionCall) -> Result<ActionResult> {
        let _ = self
            .tx
            .send(String::from_utf8_lossy(&call.payload).to_string())
            .await;
        Ok(ActionResult {
            id: call.id,
            status: ActionStatus::ActionOk as i32,
            output: call.payload,
            error: None,
        })
    }
}

struct EmitActionBehavior;

#[async_trait]
impl AgentBehavior for EmitActionBehavior {
    async fn on_event(
        &mut self,
        event: ProtoEvent,
        _state: &mut loom_core::proto::AgentState,
    ) -> Result<Vec<loom_core::proto::Action>> {
        // Emit an action mapped to provider "test.echo"
        Ok(vec![loom_core::proto::Action {
            action_type: "test.echo".into(),
            parameters: Default::default(),
            payload: event.payload.clone(),
            priority: 80, // map to Realtime QoS
        }])
    }
    async fn on_init(&mut self, _config: &loom_core::proto::AgentConfig) -> Result<()> {
        Ok(())
    }
    async fn on_shutdown(&mut self) -> Result<()> {
        Ok(())
    }
}

#[tokio::test]
async fn agent_executes_action_via_broker() -> Result<()> {
    let mut loom = Loom::new().await?;
    loom.start().await?;

    let (tx, mut rx) = mpsc::channel::<String>(1);
    loom.action_broker
        .register_provider(Arc::new(TestEchoProvider { tx }));

    let cfg = AgentConfig {
        agent_id: "agent_exec".into(),
        agent_type: "test".into(),
        subscribed_topics: vec!["topic.exec".into()],
        capabilities: vec![],
        parameters: Default::default(),
    };

    let _id = loom
        .agent_runtime
        .create_agent(cfg, Box::new(EmitActionBehavior))
        .await?;

    // Send one event that triggers the action
    let evt = ProtoEvent {
        id: "evt1".into(),
        r#type: "unit".into(),
        timestamp_ms: 0,
        source: "test".into(),
        metadata: Default::default(),
        payload: b"hello".to_vec(),
        confidence: 1.0,
        tags: vec![],
        priority: 80,
    };
    let _ = loom.event_bus.publish("topic.exec", evt).await?;

    // Provider should receive the payload via the broker
    let got = tokio::time::timeout(std::time::Duration::from_secs(2), rx.recv()).await;
    assert!(got.is_ok(), "provider was not invoked by agent via broker");
    assert_eq!(got.unwrap().unwrap(), "hello");

    loom.shutdown().await?;
    Ok(())
}
