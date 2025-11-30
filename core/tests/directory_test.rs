use std::sync::Arc;

use async_trait::async_trait;
use loom_core::directory::AgentStatus as DirectoryAgentStatus;
use loom_core::tools::{Tool, ToolResult};
use loom_core::{AgentDirectory, AgentInfo, CapabilityDirectory, ToolRegistry};
use serde_json::{json, Value};

struct EchoTool;

#[async_trait]
impl Tool for EchoTool {
    fn name(&self) -> String {
        "echo".to_string()
    }

    fn description(&self) -> String {
        "Echo back the input".to_string()
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "message": {"type": "string"}
            }
        })
    }

    async fn call(&self, arguments: Value) -> ToolResult<Value> {
        Ok(arguments)
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
        last_heartbeat: None,
        status: DirectoryAgentStatus::Active,
    });
    dir.register_agent(AgentInfo {
        agent_id: "agent.b".into(),
        subscribed_topics: vec!["topic.b".into(), "topic.shared".into()],
        capabilities: vec!["search".into()],
        metadata: Default::default(),
        last_heartbeat: None,
        status: DirectoryAgentStatus::Active,
    });

    assert_eq!(dir.by_topic("topic.shared").len(), 2);
    assert_eq!(dir.by_capability("echo"), vec!["agent.a".to_string()]);

    dir.unregister_agent("agent.a");
    assert_eq!(dir.by_capability("echo").len(), 0);
}

#[tokio::test]
async fn capability_directory_snapshot() {
    let registry = Arc::new(ToolRegistry::new());
    registry.register(Arc::new(EchoTool)).await;
    let cap_dir = CapabilityDirectory::new();
    cap_dir.refresh_from_registry(&registry);
    let list = cap_dir.list();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].name, "echo");
}
