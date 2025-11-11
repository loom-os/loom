use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use dashmap::DashMap;

use crate::action_broker::ActionBroker;
use crate::proto::CapabilityDescriptor;

/// Agent capability descriptor with subscriptions
#[derive(Debug, Clone, Default)]
pub struct AgentInfo {
    pub agent_id: String,
    pub subscribed_topics: Vec<String>,
    pub capabilities: Vec<String>,
    pub metadata: HashMap<String, String>,
}

/// In-memory directory of agents
#[derive(Debug, Default)]
pub struct AgentDirectory {
    agents: DashMap<String, AgentInfo>,
    topic_index: DashMap<String, HashSet<String>>, // topic -> agent_ids
    capability_index: DashMap<String, HashSet<String>>, // capability -> agent_ids
}

impl AgentDirectory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_agent(&self, info: AgentInfo) {
        let id = info.agent_id.clone();
        // Remove old indexes if exists
        if let Some(old) = self.agents.get(&id) {
            for t in &old.subscribed_topics {
                if let Some(mut set) = self.topic_index.get_mut(t) {
                    set.remove(&id);
                }
            }
            for c in &old.capabilities {
                if let Some(mut set) = self.capability_index.get_mut(c) {
                    set.remove(&id);
                }
            }
        }
        // Insert
        for t in &info.subscribed_topics {
            self.topic_index
                .entry(t.clone())
                .or_default()
                .insert(id.clone());
        }
        for c in &info.capabilities {
            self.capability_index
                .entry(c.clone())
                .or_default()
                .insert(id.clone());
        }
        self.agents.insert(id, info);
    }

    pub fn unregister_agent(&self, agent_id: &str) {
        if let Some((_, old)) = self.agents.remove(agent_id) {
            for t in old.subscribed_topics {
                if let Some(mut set) = self.topic_index.get_mut(&t) {
                    set.remove(agent_id);
                }
            }
            for c in old.capabilities {
                if let Some(mut set) = self.capability_index.get_mut(&c) {
                    set.remove(agent_id);
                }
            }
        }
    }

    pub fn get(&self, agent_id: &str) -> Option<AgentInfo> {
        self.agents.get(agent_id).map(|e| e.clone())
    }

    pub fn by_topic(&self, topic: &str) -> Vec<String> {
        self.topic_index
            .get(topic)
            .map(|s| s.iter().cloned().collect())
            .unwrap_or_default()
    }

    pub fn by_capability(&self, capability: &str) -> Vec<String> {
        self.capability_index
            .get(capability)
            .map(|s| s.iter().cloned().collect())
            .unwrap_or_default()
    }

    pub fn all(&self) -> Vec<AgentInfo> {
        self.agents.iter().map(|e| e.clone()).collect()
    }
}

/// Snapshotting directory of capabilities using ActionBroker
#[derive(Debug, Default)]
pub struct CapabilityDirectory {
    snapshot: DashMap<String, CapabilityDescriptor>, // key: name:version
}

impl CapabilityDirectory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn refresh_from_broker(&self, broker: &ActionBroker) {
        self.snapshot.clear();
        for d in broker.list_capabilities() {
            let key = format!("{}:{}", d.name, d.version);
            self.snapshot.insert(key, d);
        }
    }

    pub fn list(&self) -> Vec<CapabilityDescriptor> {
        self.snapshot.iter().map(|e| e.clone()).collect()
    }

    pub fn find_by_name(&self, name: &str) -> Vec<CapabilityDescriptor> {
        self.snapshot
            .iter()
            .filter(|e| e.name == name)
            .map(|e| e.clone())
            .collect()
    }

    pub fn get(&self, name: &str, version: &str) -> Option<CapabilityDescriptor> {
        let key = format!("{}:{}", name, version);
        self.snapshot.get(&key).map(|e| e.clone())
    }
}
