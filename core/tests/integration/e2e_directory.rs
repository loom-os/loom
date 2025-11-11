use async_trait::async_trait;
use loom_core::action_broker::CapabilityProvider;
use loom_core::proto::{
    ActionCall, ActionResult, ActionStatus, CapabilityDescriptor, ProviderKind,
};
use loom_core::{ActionBroker, AgentDirectory, CapabilityDirectory, Result};
use std::sync::Arc;

struct EchoProvider {
    name: String,
    version: String,
}

#[async_trait]
impl CapabilityProvider for EchoProvider {
    fn descriptor(&self) -> CapabilityDescriptor {
        CapabilityDescriptor {
            name: self.name.clone(),
            version: self.version.clone(),
            provider: ProviderKind::ProviderNative as i32,
            metadata: Default::default(),
        }
    }
    async fn invoke(&self, call: ActionCall) -> Result<ActionResult> {
        Ok(ActionResult {
            id: call.id,
            status: ActionStatus::ActionOk as i32,
            output: call.payload,
            error: None,
        })
    }
}

#[tokio::test]
async fn agent_directory_indexes_and_updates() -> Result<()> {
    let dir = AgentDirectory::new();
    dir.register_agent(loom_core::AgentInfo {
        agent_id: "a1".into(),
        subscribed_topics: vec!["t.x".into(), "t.y".into()],
        capabilities: vec!["cap.search".into()],
        metadata: Default::default(),
    });
    dir.register_agent(loom_core::AgentInfo {
        agent_id: "a2".into(),
        subscribed_topics: vec!["t.y".into()],
        capabilities: vec!["cap.tts".into()],
        metadata: Default::default(),
    });

    let ty = dir.by_topic("t.y");
    assert_eq!(ty.len(), 2);
    let cap = dir.by_capability("cap.search");
    assert_eq!(cap, vec!["a1".to_string()]);

    // Update a1 to drop t.x and add cap.tts
    dir.register_agent(loom_core::AgentInfo {
        agent_id: "a1".into(),
        subscribed_topics: vec!["t.y".into()],
        capabilities: vec!["cap.tts".into()],
        metadata: Default::default(),
    });
    assert_eq!(dir.by_topic("t.x").len(), 0);
    assert!(dir.by_capability("cap.search").is_empty());
    assert_eq!(dir.by_capability("cap.tts").len(), 2);

    dir.unregister_agent("a2");
    assert_eq!(dir.by_capability("cap.tts").len(), 1);
    Ok(())
}

#[tokio::test]
async fn capability_directory_snapshots_broker() -> Result<()> {
    let broker = ActionBroker::new();
    broker.register_provider(Arc::new(EchoProvider {
        name: "cap.echo".into(),
        version: "1.0.0".into(),
    }));
    broker.register_provider(Arc::new(EchoProvider {
        name: "cap.echo".into(),
        version: "2.0.0".into(),
    }));
    broker.register_provider(Arc::new(EchoProvider {
        name: "cap.weather".into(),
        version: "1.1.0".into(),
    }));

    let caps = CapabilityDirectory::new();
    caps.refresh_from_broker(&broker);

    let list = caps.list();
    assert_eq!(list.len(), 3);
    let echo = caps.find_by_name("cap.echo");
    assert_eq!(echo.len(), 2);
    let w = caps.get("cap.weather", "1.1.0");
    assert!(w.is_some());
    Ok(())
}
