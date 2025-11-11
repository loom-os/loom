use std::sync::Arc;

use async_trait::async_trait;
use loom_core::proto::{ActionCall, ActionResult, ActionStatus, CapabilityDescriptor};
use loom_core::Result;
use loom_core::{ActionBroker, AgentDirectory, AgentInfo, CapabilityDirectory, CapabilityProvider};

struct EchoProvider;

#[async_trait]
impl CapabilityProvider for EchoProvider {
    fn descriptor(&self) -> CapabilityDescriptor {
        CapabilityDescriptor {
            name: "echo".into(),
            version: "v1".into(),
            provider: loom_core::proto::ProviderKind::ProviderNative as i32,
            metadata: std::collections::HashMap::new(),
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
async fn agent_directory_indexing() {
    let dir = AgentDirectory::new();
    dir.register_agent(AgentInfo {
        agent_id: "agent.a".into(),
        subscribed_topics: vec!["topic.a".into(), "topic.shared".into()],
        capabilities: vec!["echo".into()],
        metadata: Default::default(),
    });
    dir.register_agent(AgentInfo {
        agent_id: "agent.b".into(),
        subscribed_topics: vec!["topic.b".into(), "topic.shared".into()],
        capabilities: vec!["search".into()],
        metadata: Default::default(),
    });

    assert_eq!(dir.by_topic("topic.shared").len(), 2);
    assert_eq!(dir.by_capability("echo"), vec!["agent.a".to_string()]);

    dir.unregister_agent("agent.a");
    assert_eq!(dir.by_capability("echo").len(), 0);
}

#[tokio::test]
async fn capability_directory_snapshot() {
    let broker = Arc::new(ActionBroker::new());
    broker.register_provider(Arc::new(EchoProvider));
    let cap_dir = CapabilityDirectory::new();
    cap_dir.refresh_from_broker(&broker);
    let list = cap_dir.list();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].name, "echo");
}
