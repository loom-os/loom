// Event flow tracker for Dashboard
//
// Tracks event flow between agents and components for visualization

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;

// Type aliases to reduce type complexity and satisfy clippy
type FlowKey = (String, String, String); // (source, target, topic)
type FlowMap = HashMap<FlowKey, EventFlow>;
type NodeMap = HashMap<String, FlowNode>;
type Shared<T> = Arc<RwLock<T>>;

const FLOW_RETENTION_MS: u64 = 60_000;
const NODE_RETENTION_MS: u64 = 120_000;
const MAX_TOPICS_PER_NODE: usize = 20;

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
    pub topics: VecDeque<String>,
    pub last_active_ms: u64,
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
        let now = Self::now_ms();

        // Add EventBus as central node
        nodes.insert(
            "EventBus".to_string(),
            FlowNode {
                id: "EventBus".to_string(),
                node_type: NodeType::EventBus,
                event_count: 0,
                topics: VecDeque::new(),
                last_active_ms: now,
            },
        );

        Self {
            flows: Arc::new(RwLock::new(HashMap::new())),
            nodes: Arc::new(RwLock::new(nodes)),
        }
    }

    /// Record an event flow from source to target
    pub async fn record_flow(&self, source: &str, target: &str, topic: &str) {
        let now = Self::now_ms();
        let source_id = source.to_string();
        let target_id = target.to_string();
        let topic_id = topic.to_string();

        // Update flow
        {
            let mut flows = self.flows.write().await;
            let key: FlowKey = (source_id.clone(), target_id.clone(), topic_id.clone());

            flows
                .entry(key)
                .and_modify(|f| {
                    f.count += 1;
                    f.last_event_ms = now;
                })
                .or_insert(EventFlow {
                    source: source_id.clone(),
                    target: target_id.clone(),
                    topic: topic_id.clone(),
                    count: 1,
                    last_event_ms: now,
                });
        }

        // Update nodes
        let mut nodes = self.nodes.write().await;
        Self::update_node(&mut nodes, &source_id, now, topic_id.as_str());
        Self::update_node(&mut nodes, &target_id, now, topic_id.as_str());
    }

    /// Get current flow graph snapshot
    pub async fn get_graph(&self) -> FlowGraph {
        let flows = self.flows.read().await;
        let nodes = self.nodes.read().await;

        // Clean up old flows (> 30 seconds)
        let now = Self::now_ms();

        let active_flows: Vec<EventFlow> = flows
            .values()
            .filter(|f| now.saturating_sub(f.last_event_ms) < FLOW_RETENTION_MS / 2)
            .cloned()
            .collect();

        let active_nodes: Vec<FlowNode> = nodes
            .values()
            .filter(|n| {
                n.id == "EventBus" || now.saturating_sub(n.last_active_ms) < NODE_RETENTION_MS
            })
            .cloned()
            .collect();

        FlowGraph {
            nodes: active_nodes,
            flows: active_flows,
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }

    /// Clear old flows (> 60 seconds)
    pub async fn cleanup(&self) {
        let now = Self::now_ms();

        {
            let mut flows = self.flows.write().await;
            flows.retain(|_, f| now.saturating_sub(f.last_event_ms) < FLOW_RETENTION_MS);
        }

        {
            let mut nodes = self.nodes.write().await;
            nodes.retain(|id, node| {
                if id == "EventBus" {
                    return true;
                }
                now.saturating_sub(node.last_active_ms) < NODE_RETENTION_MS
            });
        }
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

    fn update_node(nodes: &mut NodeMap, node_id: &str, now: u64, topic: &str) {
        nodes
            .entry(node_id.to_string())
            .and_modify(|n| {
                n.event_count = n.event_count.saturating_add(1);
                n.last_active_ms = now;
                if !n.topics.iter().any(|existing| existing == topic) {
                    if n.topics.len() >= MAX_TOPICS_PER_NODE {
                        n.topics.pop_front();
                    }
                    n.topics.push_back(topic.to_string());
                }
            })
            .or_insert_with(|| {
                let mut topics = VecDeque::new();
                topics.push_back(topic.to_string());
                FlowNode {
                    id: node_id.to_string(),
                    node_type: Self::infer_node_type(node_id),
                    event_count: 1,
                    topics,
                    last_active_ms: now,
                }
            });
    }

    fn now_ms() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_else(|_| std::time::Duration::from_millis(0))
            .as_millis() as u64
    }
}

impl Default for FlowTracker {
    fn default() -> Self {
        Self::new()
    }
}
