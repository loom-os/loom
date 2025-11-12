// Topology builder for Dashboard
//
// Builds agent topology graph from AgentDirectory and EventBus metrics

use crate::directory::AgentDirectory;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TopologySnapshot {
    pub agents: Vec<AgentNode>,
    pub edges: Vec<TopologyEdge>,
    pub timestamp: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AgentNode {
    pub id: String,
    pub topics: Vec<String>,
    pub capabilities: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TopologyEdge {
    pub from_topic: String,
    pub to_agent: String,
    pub event_count: u64,
}

pub struct TopologyBuilder {
    agent_directory: Arc<AgentDirectory>,
}

impl TopologyBuilder {
    pub fn new(agent_directory: Arc<AgentDirectory>) -> Self {
        Self { agent_directory }
    }

    /// Build current topology snapshot
    pub async fn build_snapshot(&self) -> TopologySnapshot {
        let agents = self.agent_directory.all();

        let mut agent_nodes = Vec::new();
        let mut edges = Vec::new();
        let mut topic_to_agents: HashMap<String, Vec<String>> = HashMap::new();

        // Build nodes
        for info in agents {
            agent_nodes.push(AgentNode {
                id: info.agent_id.clone(),
                topics: info.subscribed_topics.clone(),
                capabilities: info.capabilities.clone(),
            });

            // Build topic -> agent mapping
            for topic in &info.subscribed_topics {
                topic_to_agents
                    .entry(topic.clone())
                    .or_default()
                    .push(info.agent_id.clone());
            }
        }

        // Build edges from topic -> subscriber relationships
        // Note: event_count is 0 in this simple version (would need EventBus integration)
        for (topic, subscribers) in topic_to_agents {
            for subscriber in subscribers {
                edges.push(TopologyEdge {
                    from_topic: topic.clone(),
                    to_agent: subscriber,
                    event_count: 0,
                });
            }
        }

        TopologySnapshot {
            agents: agent_nodes,
            edges,
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }
}
