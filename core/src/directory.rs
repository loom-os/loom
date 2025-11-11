use std::collections::{HashMap, HashSet};

use dashmap::DashMap;

use crate::action_broker::ActionBroker;
use crate::proto::CapabilityDescriptor;

/// Information about a registered agent including subscriptions and capabilities.
///
/// `AgentInfo` describes an agent's identity, the topics it subscribes to,
/// the capabilities it provides, and arbitrary metadata for filtering or routing.
#[derive(Debug, Clone, Default)]
pub struct AgentInfo {
    /// Unique identifier for this agent
    pub agent_id: String,
    /// List of EventBus topics this agent subscribes to
    pub subscribed_topics: Vec<String>,
    /// List of capability names this agent provides
    pub capabilities: Vec<String>,
    /// Arbitrary key-value metadata (e.g., role, version, region)
    pub metadata: HashMap<String, String>,
}

/// Thread-safe, in-memory directory for agent discovery and indexing.
///
/// `AgentDirectory` maintains indices of agents by topic and capability,
/// enabling fast lookup for routing and collaboration scenarios. All operations
/// are lock-free using `DashMap`.
///
/// # Thread Safety
///
/// All methods are safe to call concurrently from multiple threads. Internal
/// indices are automatically updated on register/unregister operations.
///
/// # Examples
///
/// ```
/// use loom_core::{AgentDirectory, AgentInfo};
/// use std::collections::HashMap;
///
/// let dir = AgentDirectory::new();
///
/// // Register an agent
/// let info = AgentInfo {
///     agent_id: "agent-1".to_string(),
///     subscribed_topics: vec!["tasks".to_string()],
///     capabilities: vec!["translate".to_string()],
///     metadata: HashMap::new(),
/// };
/// dir.register_agent(info);
///
/// // Find agents by topic
/// let agents = dir.by_topic("tasks");
/// assert_eq!(agents.len(), 1);
/// ```
#[derive(Debug, Default)]
pub struct AgentDirectory {
    agents: DashMap<String, AgentInfo>,
    topic_index: DashMap<String, HashSet<String>>, // topic -> agent_ids
    capability_index: DashMap<String, HashSet<String>>, // capability -> agent_ids
}

impl AgentDirectory {
    /// Creates a new empty `AgentDirectory`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers or updates an agent in the directory.
    ///
    /// If an agent with the same `agent_id` already exists, it is replaced
    /// and all indices are updated atomically. Old topic and capability
    /// subscriptions are removed before adding new ones.
    ///
    /// # Arguments
    ///
    /// * `info` - Agent information including ID, topics, capabilities, and metadata
    ///
    /// # Examples
    ///
    /// ```
    /// use loom_core::{AgentDirectory, AgentInfo};
    /// use std::collections::HashMap;
    ///
    /// let dir = AgentDirectory::new();
    ///
    /// let info = AgentInfo {
    ///     agent_id: "worker-1".to_string(),
    ///     subscribed_topics: vec!["jobs".to_string()],
    ///     capabilities: vec!["process".to_string()],
    ///     metadata: {
    ///         let mut m = HashMap::new();
    ///         m.insert("region".to_string(), "us-west".to_string());
    ///         m
    ///     },
    /// };
    /// dir.register_agent(info);
    /// ```
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

    /// Unregisters an agent and removes it from all indices.
    ///
    /// If the agent exists, removes it from the main registry and cleans up
    /// all topic and capability index entries. If the agent doesn't exist,
    /// this is a no-op.
    ///
    /// # Arguments
    ///
    /// * `agent_id` - The unique identifier of the agent to remove
    ///
    /// # Examples
    ///
    /// ```
    /// use loom_core::{AgentDirectory, AgentInfo};
    /// use std::collections::HashMap;
    ///
    /// let dir = AgentDirectory::new();
    ///
    /// let info = AgentInfo {
    ///     agent_id: "agent-1".to_string(),
    ///     subscribed_topics: vec!["tasks".to_string()],
    ///     capabilities: vec![],
    ///     metadata: HashMap::new(),
    /// };
    /// dir.register_agent(info);
    /// assert_eq!(dir.by_topic("tasks").len(), 1);
    ///
    /// dir.unregister_agent("agent-1");
    /// assert_eq!(dir.by_topic("tasks").len(), 0);
    /// ```
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

    /// Retrieves information about a specific agent by ID.
    ///
    /// # Arguments
    ///
    /// * `agent_id` - The unique identifier of the agent
    ///
    /// # Returns
    ///
    /// * `Some(AgentInfo)` - If the agent exists
    /// * `None` - If the agent is not registered
    ///
    /// # Examples
    ///
    /// ```
    /// use loom_core::{AgentDirectory, AgentInfo};
    /// use std::collections::HashMap;
    ///
    /// let dir = AgentDirectory::new();
    ///
    /// let info = AgentInfo {
    ///     agent_id: "agent-1".to_string(),
    ///     subscribed_topics: vec![],
    ///     capabilities: vec!["translate".to_string()],
    ///     metadata: HashMap::new(),
    /// };
    /// dir.register_agent(info);
    ///
    /// if let Some(agent) = dir.get("agent-1") {
    ///     println!("Found agent: {}", agent.agent_id);
    /// }
    /// ```
    pub fn get(&self, agent_id: &str) -> Option<AgentInfo> {
        self.agents.get(agent_id).map(|e| e.clone())
    }

    /// Finds all agent IDs subscribed to a specific topic.
    ///
    /// Returns a snapshot of agent IDs that have the specified topic in their
    /// `subscribed_topics` list. The order is undefined.
    ///
    /// # Arguments
    ///
    /// * `topic` - The topic name to search for
    ///
    /// # Returns
    ///
    /// Vector of agent IDs subscribed to the topic. Empty if no agents are subscribed.
    ///
    /// # Examples
    ///
    /// ```
    /// use loom_core::{AgentDirectory, AgentInfo};
    /// use std::collections::HashMap;
    ///
    /// let dir = AgentDirectory::new();
    ///
    /// for i in 1..=3 {
    ///     dir.register_agent(AgentInfo {
    ///         agent_id: format!("agent-{}", i),
    ///         subscribed_topics: vec!["notifications".to_string()],
    ///         capabilities: vec![],
    ///         metadata: HashMap::new(),
    ///     });
    /// }
    ///
    /// let subscribers = dir.by_topic("notifications");
    /// assert_eq!(subscribers.len(), 3);
    /// ```
    pub fn by_topic(&self, topic: &str) -> Vec<String> {
        self.topic_index
            .get(topic)
            .map(|s| s.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Finds all agent IDs that provide a specific capability.
    ///
    /// Returns a snapshot of agent IDs that have the specified capability in their
    /// `capabilities` list. The order is undefined.
    ///
    /// # Arguments
    ///
    /// * `capability` - The capability name to search for
    ///
    /// # Returns
    ///
    /// Vector of agent IDs providing the capability. Empty if no agents provide it.
    ///
    /// # Examples
    ///
    /// ```
    /// use loom_core::{AgentDirectory, AgentInfo};
    /// use std::collections::HashMap;
    ///
    /// let dir = AgentDirectory::new();
    ///
    /// dir.register_agent(AgentInfo {
    ///     agent_id: "translator-1".to_string(),
    ///     subscribed_topics: vec![],
    ///     capabilities: vec!["translate".to_string(), "summarize".to_string()],
    ///     metadata: HashMap::new(),
    /// });
    ///
    /// let translators = dir.by_capability("translate");
    /// assert_eq!(translators.len(), 1);
    /// ```
    pub fn by_capability(&self, capability: &str) -> Vec<String> {
        self.capability_index
            .get(capability)
            .map(|s| s.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Returns all registered agents.
    ///
    /// Returns a snapshot of all `AgentInfo` entries in the directory.
    /// The order is undefined.
    ///
    /// # Returns
    ///
    /// Vector of all registered agent information. Empty if no agents are registered.
    ///
    /// # Examples
    ///
    /// ```
    /// use loom_core::{AgentDirectory, AgentInfo};
    /// use std::collections::HashMap;
    ///
    /// let dir = AgentDirectory::new();
    ///
    /// dir.register_agent(AgentInfo {
    ///     agent_id: "agent-1".to_string(),
    ///     subscribed_topics: vec![],
    ///     capabilities: vec![],
    ///     metadata: HashMap::new(),
    /// });
    ///
    /// dir.register_agent(AgentInfo {
    ///     agent_id: "agent-2".to_string(),
    ///     subscribed_topics: vec![],
    ///     capabilities: vec![],
    ///     metadata: HashMap::new(),
    /// });
    ///
    /// let all_agents = dir.all();
    /// assert_eq!(all_agents.len(), 2);
    /// ```
    pub fn all(&self) -> Vec<AgentInfo> {
        self.agents.iter().map(|e| e.clone()).collect()
    }
}

/// Thread-safe snapshot-based directory for capability discovery.
///
/// `CapabilityDirectory` maintains a snapshot of capabilities registered in an
/// `ActionBroker`. Unlike `AgentDirectory` which tracks agents, this tracks
/// the actual capability implementations (providers) available for invocation.
///
/// # Snapshot Model
///
/// The directory uses a snapshot model: call `refresh_from_broker()` to update
/// the internal cache from the ActionBroker's current state. Queries operate on
/// this cached snapshot, not live broker state.
///
/// # Thread Safety
///
/// All methods are safe to call concurrently. Uses `DashMap` for lock-free reads.
///
/// # Examples
///
/// ```
/// use loom_core::{CapabilityDirectory, ActionBroker};
///
/// let broker = ActionBroker::new();
/// let dir = CapabilityDirectory::new();
///
/// // Refresh snapshot from broker
/// dir.refresh_from_broker(&broker);
///
/// // Query capabilities
/// let all = dir.list();
/// println!("Found {} capabilities", all.len());
/// ```
#[derive(Debug, Default)]
pub struct CapabilityDirectory {
    snapshot: DashMap<String, CapabilityDescriptor>, // key: name:version
}

impl CapabilityDirectory {
    /// Creates a new empty `CapabilityDirectory`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Refreshes the internal snapshot from an `ActionBroker`.
    ///
    /// Clears the existing snapshot and rebuilds it from the broker's current
    /// list of registered capabilities. This operation is atomic from the
    /// perspective of concurrent readers.
    ///
    /// # Arguments
    ///
    /// * `broker` - The ActionBroker to snapshot capabilities from
    ///
    /// # Examples
    ///
    /// ```
    /// use loom_core::{CapabilityDirectory, ActionBroker};
    /// use std::sync::Arc;
    ///
    /// let broker = Arc::new(ActionBroker::new());
    /// let dir = CapabilityDirectory::new();
    ///
    /// // Initial snapshot
    /// dir.refresh_from_broker(&broker);
    ///
    /// // Later, after registering more providers...
    /// // broker.register_provider(...);
    /// dir.refresh_from_broker(&broker);  // Update snapshot
    /// ```
    pub fn refresh_from_broker(&self, broker: &ActionBroker) {
        self.snapshot.clear();
        for d in broker.list_capabilities() {
            let key = format!("{}:{}", d.name, d.version);
            self.snapshot.insert(key, d);
        }
    }

    /// Returns all capabilities in the current snapshot.
    ///
    /// Returns a cloned vector of all capability descriptors. The order is undefined.
    /// This reflects the state at the last `refresh_from_broker()` call.
    ///
    /// # Returns
    ///
    /// Vector of all capability descriptors in the snapshot. Empty if no capabilities
    /// are registered or if `refresh_from_broker()` was never called.
    ///
    /// # Examples
    ///
    /// ```
    /// use loom_core::{CapabilityDirectory, ActionBroker};
    ///
    /// let broker = ActionBroker::new();
    /// let dir = CapabilityDirectory::new();
    ///
    /// dir.refresh_from_broker(&broker);
    /// let all_capabilities = dir.list();
    /// println!("Available capabilities: {}", all_capabilities.len());
    /// ```
    pub fn list(&self) -> Vec<CapabilityDescriptor> {
        self.snapshot.iter().map(|e| e.clone()).collect()
    }

    /// Finds all capability versions matching a specific capability name.
    ///
    /// Returns all registered versions of a capability (e.g., all versions of "translate").
    /// Useful when you want to see available versions or select the latest.
    ///
    /// # Arguments
    ///
    /// * `name` - The capability name to search for (exact match)
    ///
    /// # Returns
    ///
    /// Vector of matching capability descriptors. Empty if the capability name
    /// is not found. The order is undefined.
    ///
    /// # Examples
    ///
    /// ```
    /// use loom_core::{CapabilityDirectory, ActionBroker};
    ///
    /// let broker = ActionBroker::new();
    /// let dir = CapabilityDirectory::new();
    ///
    /// dir.refresh_from_broker(&broker);
    ///
    /// // Find all versions of "translate" capability
    /// let translate_versions = dir.find_by_name("translate");
    /// for cap in translate_versions {
    ///     println!("translate v{}", cap.version);
    /// }
    /// ```
    pub fn find_by_name(&self, name: &str) -> Vec<CapabilityDescriptor> {
        self.snapshot
            .iter()
            .filter(|e| e.name == name)
            .map(|e| e.clone())
            .collect()
    }

    /// Retrieves a specific capability by name and version.
    ///
    /// Performs an exact lookup using both name and version. This is the most
    /// precise way to resolve a capability.
    ///
    /// # Arguments
    ///
    /// * `name` - The capability name (e.g., "weather.get")
    /// * `version` - The capability version (e.g., "1.0.0")
    ///
    /// # Returns
    ///
    /// * `Some(CapabilityDescriptor)` - If the exact name:version exists
    /// * `None` - If not found in the snapshot
    ///
    /// # Examples
    ///
    /// ```
    /// use loom_core::{CapabilityDirectory, ActionBroker};
    ///
    /// let broker = ActionBroker::new();
    /// let dir = CapabilityDirectory::new();
    ///
    /// dir.refresh_from_broker(&broker);
    ///
    /// if let Some(cap) = dir.get("translate", "2.1.0") {
    ///     println!("Found: {} v{}", cap.name, cap.version);
    /// } else {
    ///     println!("Capability not found");
    /// }
    /// ```
    pub fn get(&self, name: &str, version: &str) -> Option<CapabilityDescriptor> {
        let key = format!("{}:{}", name, version);
        self.snapshot.get(&key).map(|e| e.clone())
    }
}
