// Event flow tracker for Dashboard
//
// Tracks event flow between agents and components for visualization

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

// Type aliases to reduce type complexity and satisfy clippy
type FlowKey = (String, String, String); // (source, target, topic)
type FlowMap = HashMap<FlowKey, EventFlow>;
type NodeMap = HashMap<String, FlowNode>;
type Shared<T> = Arc<RwLock<T>>;

/// Represents an event flow between two nodes (agents/components)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EventFlow {
    pub source: String,
    pub target: String,
    pub topic: String,
    pub count: u64,
    pub last_event_ms: u64,
}

/// Represents a node in the flow graph
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FlowNode {
    pub id: String,
    pub node_type: NodeType,
    pub event_count: u64,
    pub topics: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NodeType {
    Agent,
    EventBus,
    Router,
    LLM,
    Tool,
    Storage,
}

/// Flow graph snapshot for visualization
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FlowGraph {
    pub nodes: Vec<FlowNode>,
    pub flows: Vec<EventFlow>,
    pub timestamp: String,
}

/// Tracks event flows between nodes
pub struct FlowTracker {
    flows: Shared<FlowMap>,
    nodes: Shared<NodeMap>,
}

impl FlowTracker {
    pub fn new() -> Self {
        let mut nodes = HashMap::new();

        // Add EventBus as central node
        nodes.insert(
            "EventBus".to_string(),
            FlowNode {
                id: "EventBus".to_string(),
                node_type: NodeType::EventBus,
                event_count: 0,
                topics: vec![],
            },
        );

        Self {
            flows: Arc::new(RwLock::new(HashMap::new())),
            nodes: Arc::new(RwLock::new(nodes)),
        }
    }

    /// Record an event flow from source to target
    pub async fn record_flow(&self, source: &str, target: &str, topic: &str) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        // Update flow
        let mut flows = self.flows.write().await;
        let key: FlowKey = (source.to_string(), target.to_string(), topic.to_string());

        flows
            .entry(key)
            .and_modify(|f| {
                f.count += 1;
                f.last_event_ms = now;
            })
            .or_insert(EventFlow {
                source: source.to_string(),
                target: target.to_string(),
                topic: topic.to_string(),
                count: 1,
                last_event_ms: now,
            });

        // Update nodes
        let mut nodes = self.nodes.write().await;

        // Update or create source node
        nodes
            .entry(source.to_string())
            .and_modify(|n| {
                n.event_count += 1;
                if !n.topics.contains(&topic.to_string()) {
                    n.topics.push(topic.to_string());
                }
            })
            .or_insert(FlowNode {
                id: source.to_string(),
                node_type: Self::infer_node_type(source),
                event_count: 1,
                topics: vec![topic.to_string()],
            });

        // Update or create target node
        nodes
            .entry(target.to_string())
            .and_modify(|n| {
                n.event_count += 1;
                if !n.topics.contains(&topic.to_string()) {
                    n.topics.push(topic.to_string());
                }
            })
            .or_insert(FlowNode {
                id: target.to_string(),
                node_type: Self::infer_node_type(target),
                event_count: 1,
                topics: vec![topic.to_string()],
            });
    }

    /// Get current flow graph snapshot
    pub async fn get_graph(&self) -> FlowGraph {
        let flows = self.flows.read().await;
        let nodes = self.nodes.read().await;

        // Clean up old flows (> 30 seconds)
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        let active_flows: Vec<EventFlow> = flows
            .values()
            .filter(|f| now - f.last_event_ms < 30_000)
            .cloned()
            .collect();

        FlowGraph {
            nodes: nodes.values().cloned().collect(),
            flows: active_flows,
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }

    /// Clear old flows (> 60 seconds)
    pub async fn cleanup(&self) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        let mut flows = self.flows.write().await;
        flows.retain(|_, f| now - f.last_event_ms < 60_000);
    }

    /// Infer node type from node ID
    fn infer_node_type(node_id: &str) -> NodeType {
        match node_id {
            "EventBus" => NodeType::EventBus,
            "Router" => NodeType::Router,
            id if id.contains("llm") || id.contains("LLM") => NodeType::LLM,
            id if id.contains("tool") => NodeType::Tool,
            id if id.contains("storage") => NodeType::Storage,
            _ => NodeType::Agent,
        }
    }
}

impl Default for FlowTracker {
    fn default() -> Self {
        Self::new()
    }
}
