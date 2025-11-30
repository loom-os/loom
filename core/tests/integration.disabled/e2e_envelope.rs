use async_trait::async_trait;
use loom_core::action_broker::CapabilityProvider;
use loom_core::agent::AgentBehavior;
use loom_core::proto::{
    Action, ActionCall, ActionResult, ActionStatus, AgentConfig, AgentState, CapabilityDescriptor,
    Event, ProviderKind,
};
use loom_core::{ActionBroker, AgentRuntime, Envelope, EventBus, ModelRouter, Result};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

fn make_event(id: &str) -> Event {
    Event {
        id: id.to_string(),
        r#type: "test".to_string(),
        timestamp_ms: 0,
        source: "tester".to_string(),
        metadata: HashMap::new(),
        payload: vec![],
        confidence: 1.0,
        tags: vec![],
        priority: 0,
    }
}

struct EnvCaptureBehavior {
    pub last_env: Arc<Mutex<Option<loom_core::Envelope>>>,
}

#[async_trait]
impl AgentBehavior for EnvCaptureBehavior {
    async fn on_event(&mut self, event: Event, _state: &mut AgentState) -> Result<Vec<Action>> {
        let env = loom_core::Envelope::from_event(&event);
        let mut slot = self.last_env.lock().await;
        *slot = Some(env);
        Ok(vec![])
    }

    async fn on_init(&mut self, _config: &AgentConfig) -> Result<()> {
        Ok(())
    }
    async fn on_shutdown(&mut self) -> Result<()> {
        Ok(())
    }
}

#[tokio::test]
async fn ttl_1_drops_before_behavior() -> Result<()> {
    let bus = Arc::new(EventBus::new().await?);
    bus.start().await?;
    let broker = Arc::new(ActionBroker::new());
    let router = ModelRouter::new().await?;
    let runtime = AgentRuntime::new(Arc::clone(&bus), Arc::clone(&broker), router).await?;

    let capture = Arc::new(Mutex::new(None));
    let cfg = AgentConfig {
        agent_id: "agent_ttl".into(),
        agent_type: "test".into(),
        subscribed_topics: vec!["topic.ttl".into()],
        capabilities: vec![],
        parameters: Default::default(),
    };

    runtime
        .create_agent(
            cfg,
            Box::new(EnvCaptureBehavior {
                last_env: Arc::clone(&capture),
            }),
        )
        .await?;

    // Build event with TTL=1 so agent should drop before behavior
    let mut evt = make_event("evt1");
    let mut env = Envelope::new("threadX", "tester");
    env.ttl = 1;
    env.attach_to_event(&mut evt);

    bus.publish("topic.ttl", evt).await?;
    tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;

    let seen = capture.lock().await;
    assert!(
        seen.is_none(),
        "behavior should not run because TTL=1 is exhausted by next_hop()"
    );
    Ok(())
}

#[tokio::test]
async fn ttl_2_reaches_behavior_with_hop_1() -> Result<()> {
    let bus = Arc::new(EventBus::new().await?);
    bus.start().await?;
    let broker = Arc::new(ActionBroker::new());
    let router = ModelRouter::new().await?;
    let runtime = AgentRuntime::new(Arc::clone(&bus), Arc::clone(&broker), router).await?;

    let capture = Arc::new(Mutex::new(None));
    let cfg = AgentConfig {
        agent_id: "agent_ttl2".into(),
        agent_type: "test".into(),
        subscribed_topics: vec!["topic.ttl2".into()],
        capabilities: vec![],
        parameters: Default::default(),
    };

    runtime
        .create_agent(
            cfg,
            Box::new(EnvCaptureBehavior {
                last_env: Arc::clone(&capture),
            }),
        )
        .await?;

    let mut evt = make_event("evt2");
    let mut env = Envelope::new("threadY", "tester");
    env.ttl = 2;
    env.attach_to_event(&mut evt);

    bus.publish("topic.ttl2", evt).await?;
    tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;

    let seen = capture.lock().await;
    let e = seen
        .as_ref()
        .expect("behavior should have captured envelope");
    assert_eq!(e.hop, 1, "hop should increment to 1 inside agent loop");
    assert_eq!(e.ttl, 1, "ttl should decrement to 1 when starting from 2");
    Ok(())
}

// Provider that captures headers of received ActionCall
struct HeaderCaptureProvider {
    pub last: Arc<Mutex<Option<(String, HashMap<String, String>)>>>,
    pub name: String,
}

#[async_trait]
impl CapabilityProvider for HeaderCaptureProvider {
    fn descriptor(&self) -> CapabilityDescriptor {
        CapabilityDescriptor {
            name: self.name.clone(),
            version: "1.0.0".into(),
            provider: ProviderKind::ProviderNative as i32,
            metadata: Default::default(),
        }
    }

    async fn invoke(&self, call: ActionCall) -> Result<ActionResult> {
        let mut m = self.last.lock().await;
        *m = Some((call.id.clone(), call.headers.clone()));
        Ok(ActionResult {
            id: call.id,
            status: ActionStatus::ActionOk as i32,
            output: vec![],
            error: None,
        })
    }
}

struct EmitActionBehavior {
    cap: String,
}

#[async_trait]
impl AgentBehavior for EmitActionBehavior {
    async fn on_event(&mut self, event: Event, _state: &mut AgentState) -> Result<Vec<Action>> {
        Ok(vec![Action {
            action_type: self.cap.clone(),
            parameters: Default::default(),
            payload: event.payload.clone(),
            priority: 50,
        }])
    }
    async fn on_init(&mut self, _config: &AgentConfig) -> Result<()> {
        Ok(())
    }
    async fn on_shutdown(&mut self) -> Result<()> {
        Ok(())
    }
}

#[tokio::test]
async fn action_broker_receives_envelope_headers() -> Result<()> {
    let bus = Arc::new(EventBus::new().await?);
    bus.start().await?;
    let broker = Arc::new(ActionBroker::new());
    let router = ModelRouter::new().await?;
    let runtime = AgentRuntime::new(Arc::clone(&bus), Arc::clone(&broker), router).await?;

    let cap_name = "test.capture".to_string();
    let captured = Arc::new(Mutex::new(None));
    broker.register_provider(Arc::new(HeaderCaptureProvider {
        last: Arc::clone(&captured),
        name: cap_name.clone(),
    }));

    let cfg = AgentConfig {
        agent_id: "agent_headers".into(),
        agent_type: "test".into(),
        subscribed_topics: vec!["topic.emit".into()],
        capabilities: vec![],
        parameters: Default::default(),
    };
    runtime
        .create_agent(
            cfg,
            Box::new(EmitActionBehavior {
                cap: cap_name.clone(),
            }),
        )
        .await?;

    bus.publish("topic.emit", make_event("trigger")).await?;
    tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

    let guard = captured.lock().await;
    let (id, hdrs) = guard
        .as_ref()
        .expect("provider should have been invoked")
        .clone();
    // correlation_id should equal the ActionCall id according to execute_action
    let cid = hdrs.get("correlation_id").cloned().unwrap_or_default();
    assert_eq!(cid, id, "correlation_id in headers should equal call.id");
    // sender must be the agent.* identity
    let sender = hdrs.get("sender").cloned().unwrap_or_default();
    assert!(sender.starts_with("agent."), "sender should be agent.*");
    Ok(())
}
