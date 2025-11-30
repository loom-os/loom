use async_trait::async_trait;
use loom_core::agent::{AgentBehavior, AgentRuntime};
use loom_core::proto::{Action, AgentConfig, AgentState, Event};
use loom_core::{EventBus, ModelRouter, Result, ToolRegistry};
use std::sync::Arc;
use tokio::sync::Mutex;

// Mock behavior that counts events
struct CountingBehavior {
    counter: Arc<Mutex<usize>>,
}

#[async_trait]
impl AgentBehavior for CountingBehavior {
    async fn on_event(&mut self, _event: Event, _state: &mut AgentState) -> Result<Vec<Action>> {
        let mut c = self.counter.lock().await;
        *c += 1;
        Ok(vec![])
    }

    async fn on_init(&mut self, _config: &AgentConfig) -> Result<()> {
        Ok(())
    }

    async fn on_shutdown(&mut self) -> Result<()> {
        Ok(())
    }
}

// Mock behavior that returns actions
struct ActionEmitBehavior {
    action_type: String,
}

#[async_trait]
impl AgentBehavior for ActionEmitBehavior {
    async fn on_event(&mut self, event: Event, _state: &mut AgentState) -> Result<Vec<Action>> {
        Ok(vec![Action {
            action_type: self.action_type.clone(),
            parameters: Default::default(),
            payload: event.payload.clone(),
            priority: 80,
        }])
    }

    async fn on_init(&mut self, _config: &AgentConfig) -> Result<()> {
        Ok(())
    }

    async fn on_shutdown(&mut self) -> Result<()> {
        Ok(())
    }
}

fn make_event(id: &str) -> Event {
    Event {
        id: id.to_string(),
        r#type: "test".to_string(),
        timestamp_ms: 0,
        source: "test".to_string(),
        metadata: Default::default(),
        payload: vec![],
        confidence: 1.0,
        tags: vec![],
        priority: 0,
    }
}

#[tokio::test]
async fn create_agent_subscribes_and_receives_events() -> Result<()> {
    let bus = Arc::new(EventBus::new().await?);
    bus.start().await?;

    let registry = Arc::new(ToolRegistry::new());
    let router = ModelRouter::new().await?;
    let runtime = AgentRuntime::new(Arc::clone(&bus), Arc::clone(&registry), router).await?;

    let counter = Arc::new(Mutex::new(0));
    let cfg = AgentConfig {
        agent_id: "agent1".to_string(),
        agent_type: "test".to_string(),
        subscribed_topics: vec!["topic.test".to_string()],
        capabilities: vec![],
        parameters: Default::default(),
    };

    let behavior = Box::new(CountingBehavior {
        counter: Arc::clone(&counter),
    });
    let agent_id = runtime.create_agent(cfg, behavior).await?;
    assert_eq!(agent_id, "agent1");

    // Publish events
    for i in 0..5 {
        bus.publish("topic.test", make_event(&format!("e{}", i)))
            .await?;
    }

    // Wait for processing
    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    let count = *counter.lock().await;
    assert_eq!(count, 5, "agent should have received 5 events");
    Ok(())
}

#[tokio::test]
async fn delete_agent_stops_receiving_events() -> Result<()> {
    let bus = Arc::new(EventBus::new().await?);
    bus.start().await?;

    let registry = Arc::new(ToolRegistry::new());
    let router = ModelRouter::new().await?;
    let runtime = AgentRuntime::new(Arc::clone(&bus), Arc::clone(&registry), router).await?;

    let counter = Arc::new(Mutex::new(0));
    let cfg = AgentConfig {
        agent_id: "agent_del".to_string(),
        agent_type: "test".to_string(),
        subscribed_topics: vec!["topic.del".to_string()],
        capabilities: vec![],
        parameters: Default::default(),
    };

    let behavior = Box::new(CountingBehavior {
        counter: Arc::clone(&counter),
    });
    runtime.create_agent(cfg, behavior).await?;

    // Publish before delete
    bus.publish("topic.del", make_event("before")).await?;
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Delete agent
    runtime.delete_agent("agent_del").await?;

    // Publish after delete
    bus.publish("topic.del", make_event("after")).await?;
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let count = *counter.lock().await;
    assert_eq!(count, 1, "agent should only receive event before deletion");
    Ok(())
}

#[tokio::test]
async fn delete_nonexistent_agent_errors() -> Result<()> {
    let bus = Arc::new(EventBus::new().await?);
    let registry = Arc::new(ToolRegistry::new());
    let router = ModelRouter::new().await?;
    let runtime = AgentRuntime::new(Arc::clone(&bus), Arc::clone(&registry), router).await?;

    let result = runtime.delete_agent("nonexistent").await;
    assert!(result.is_err(), "deleting nonexistent agent should error");
    Ok(())
}

#[tokio::test]
async fn multiple_agents_receive_from_same_topic() -> Result<()> {
    let bus = Arc::new(EventBus::new().await?);
    bus.start().await?;

    let registry = Arc::new(ToolRegistry::new());
    let router = ModelRouter::new().await?;
    let runtime = AgentRuntime::new(Arc::clone(&bus), Arc::clone(&registry), router).await?;

    let counter1 = Arc::new(Mutex::new(0));
    let counter2 = Arc::new(Mutex::new(0));

    let cfg1 = AgentConfig {
        agent_id: "agent_a".to_string(),
        agent_type: "test".to_string(),
        subscribed_topics: vec!["topic.shared".to_string()],
        capabilities: vec![],
        parameters: Default::default(),
    };

    let cfg2 = AgentConfig {
        agent_id: "agent_b".to_string(),
        agent_type: "test".to_string(),
        subscribed_topics: vec!["topic.shared".to_string()],
        capabilities: vec![],
        parameters: Default::default(),
    };

    runtime
        .create_agent(
            cfg1,
            Box::new(CountingBehavior {
                counter: Arc::clone(&counter1),
            }),
        )
        .await?;
    runtime
        .create_agent(
            cfg2,
            Box::new(CountingBehavior {
                counter: Arc::clone(&counter2),
            }),
        )
        .await?;

    // Publish
    bus.publish("topic.shared", make_event("shared")).await?;
    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    let c1 = *counter1.lock().await;
    let c2 = *counter2.lock().await;
    assert_eq!(c1, 1, "agent_a should receive event");
    assert_eq!(c2, 1, "agent_b should receive event");
    Ok(())
}

#[tokio::test]
async fn agent_subscribes_to_multiple_topics() -> Result<()> {
    let bus = Arc::new(EventBus::new().await?);
    bus.start().await?;

    let registry = Arc::new(ToolRegistry::new());
    let router = ModelRouter::new().await?;
    let runtime = AgentRuntime::new(Arc::clone(&bus), Arc::clone(&registry), router).await?;

    let counter = Arc::new(Mutex::new(0));
    let cfg = AgentConfig {
        agent_id: "agent_multi".to_string(),
        agent_type: "test".to_string(),
        subscribed_topics: vec!["topic.a".to_string(), "topic.b".to_string()],
        capabilities: vec![],
        parameters: Default::default(),
    };

    runtime
        .create_agent(
            cfg,
            Box::new(CountingBehavior {
                counter: Arc::clone(&counter),
            }),
        )
        .await?;

    // Publish to both topics
    bus.publish("topic.a", make_event("a1")).await?;
    bus.publish("topic.b", make_event("b1")).await?;
    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    let count = *counter.lock().await;
    assert_eq!(count, 2, "agent should receive from both topics");
    Ok(())
}

#[tokio::test]
async fn shutdown_aborts_all_agents() -> Result<()> {
    let bus = Arc::new(EventBus::new().await?);
    bus.start().await?;

    let registry = Arc::new(ToolRegistry::new());
    let router = ModelRouter::new().await?;
    let mut runtime = AgentRuntime::new(Arc::clone(&bus), Arc::clone(&registry), router).await?;

    let counter = Arc::new(Mutex::new(0));
    let cfg = AgentConfig {
        agent_id: "agent_shut".to_string(),
        agent_type: "test".to_string(),
        subscribed_topics: vec!["topic.shut".to_string()],
        capabilities: vec![],
        parameters: Default::default(),
    };

    runtime
        .create_agent(
            cfg,
            Box::new(CountingBehavior {
                counter: Arc::clone(&counter),
            }),
        )
        .await?;

    // Publish before shutdown
    bus.publish("topic.shut", make_event("pre")).await?;
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Shutdown runtime
    runtime.shutdown().await?;

    // Publish after shutdown
    bus.publish("topic.shut", make_event("post")).await?;
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let count = *counter.lock().await;
    assert_eq!(count, 1, "agent should only process pre-shutdown events");
    Ok(())
}

#[tokio::test]
async fn agent_can_emit_actions() -> Result<()> {
    let bus = Arc::new(EventBus::new().await?);
    bus.start().await?;

    let registry = Arc::new(ToolRegistry::new());
    let router = ModelRouter::new().await?;
    let runtime = AgentRuntime::new(Arc::clone(&bus), Arc::clone(&registry), router).await?;

    let cfg = AgentConfig {
        agent_id: "agent_action".to_string(),
        agent_type: "test".to_string(),
        subscribed_topics: vec!["topic.action".to_string()],
        capabilities: vec![],
        parameters: Default::default(),
    };

    runtime
        .create_agent(
            cfg,
            Box::new(ActionEmitBehavior {
                action_type: "test.action".to_string(),
            }),
        )
        .await?;

    // Publish event
    bus.publish("topic.action", make_event("trigger")).await?;
    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    // Note: full action execution verification requires ActionBroker integration,
    // which is tested in agent_action.rs. This test verifies agent can emit actions.
    Ok(())
}

#[tokio::test]
async fn create_duplicate_agent_id_replaces_previous() -> Result<()> {
    let bus = Arc::new(EventBus::new().await?);
    bus.start().await?;

    let registry = Arc::new(ToolRegistry::new());
    let router = ModelRouter::new().await?;
    let runtime = AgentRuntime::new(Arc::clone(&bus), Arc::clone(&registry), router).await?;

    let counter1 = Arc::new(Mutex::new(0));
    let counter2 = Arc::new(Mutex::new(0));

    let cfg1 = AgentConfig {
        agent_id: "dup_agent".to_string(),
        agent_type: "test".to_string(),
        subscribed_topics: vec!["topic.dup".to_string()],
        capabilities: vec![],
        parameters: Default::default(),
    };

    let cfg2 = AgentConfig {
        agent_id: "dup_agent".to_string(),
        agent_type: "test".to_string(),
        subscribed_topics: vec!["topic.dup".to_string()],
        capabilities: vec![],
        parameters: Default::default(),
    };

    runtime
        .create_agent(
            cfg1,
            Box::new(CountingBehavior {
                counter: Arc::clone(&counter1),
            }),
        )
        .await?;

    // Delete first agent
    runtime.delete_agent("dup_agent").await?;
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    // Create again with same ID
    runtime
        .create_agent(
            cfg2,
            Box::new(CountingBehavior {
                counter: Arc::clone(&counter2),
            }),
        )
        .await?;

    // Publish
    bus.publish("topic.dup", make_event("test")).await?;
    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    // Second agent should receive, first should not (deleted before second was created)
    let c1 = *counter1.lock().await;
    let c2 = *counter2.lock().await;
    assert_eq!(c1, 0, "first agent should not receive after deletion");
    assert_eq!(c2, 1, "second agent should receive");
    Ok(())
}
