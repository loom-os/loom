use async_trait::async_trait;
use loom_core::agent::AgentBehavior;
use loom_core::proto::{AgentConfig, Event};
use loom_core::{Loom, Result};

struct TestBehavior {
    tx: tokio::sync::mpsc::Sender<String>,
}

#[async_trait]
impl AgentBehavior for TestBehavior {
    async fn on_event(
        &mut self,
        event: Event,
        _state: &mut loom_core::proto::AgentState,
    ) -> Result<Vec<loom_core::proto::Action>> {
        let _ = self.tx.send(event.id.clone()).await;
        Ok(vec![])
    }
    async fn on_init(&mut self, _config: &loom_core::proto::AgentConfig) -> Result<()> {
        Ok(())
    }
    async fn on_shutdown(&mut self) -> Result<()> {
        Ok(())
    }
}

#[tokio::test]
async fn subscriptions_receive_events_from_shared_bus() -> Result<()> {
    // Create system and start
    let mut loom = Loom::new().await?;
    loom.start().await?;

    // Channel to capture events seen by agent behavior
    let (tx, mut rx) = tokio::sync::mpsc::channel::<String>(1);

    // Create a test agent that subscribes to a topic
    let cfg = AgentConfig {
        agent_id: "agent_test".into(),
        agent_type: "test".into(),
        subscribed_topics: vec!["test.topic".into()],
        capabilities: vec![],
        parameters: Default::default(),
    };

    let behavior = Box::new(TestBehavior { tx });
    let _id = loom.agent_runtime.create_agent(cfg, behavior).await?;

    // Publish an event on Loom's event bus
    let evt = Event {
        id: "evt_shared".into(),
        r#type: "unit".into(),
        timestamp_ms: 0,
        source: "test".into(),
        metadata: Default::default(),
        payload: vec![],
        confidence: 1.0,
        tags: vec![],
        priority: 0,
    };

    let _ = loom.event_bus.publish("test.topic", evt).await?;

    // Ensure agent behavior receives the event id
    let got = tokio::time::timeout(std::time::Duration::from_secs(2), rx.recv())
        .await
        .ok()
        .flatten();

    assert_eq!(got, Some("evt_shared".to_string()));

    loom.shutdown().await?;
    Ok(())
}
