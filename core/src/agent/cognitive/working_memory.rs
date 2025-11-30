//! Working memory for cognitive agents.
//!
//! Working memory holds the current context for a cognitive cycle, including:
//! - Recent events processed
//! - Current task state
//! - Short-term memory items

use std::collections::{HashMap, VecDeque};

use serde::{Deserialize, Serialize};

use crate::proto::Event;

/// Type of memory item
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryItemType {
    /// Event summary
    Event,
    /// Tool observation/result
    Observation,
    /// User message
    UserMessage,
    /// Agent response
    AgentResponse,
    /// System notification
    System,
    /// Custom type
    Custom,
}

/// A single item in working memory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryItem {
    /// Unique ID for this item
    pub id: String,

    /// Type of memory item
    pub item_type: MemoryItemType,

    /// Timestamp when this item was added (ms since epoch)
    pub timestamp_ms: i64,

    /// The content of this memory item
    pub content: String,

    /// Additional metadata
    pub metadata: HashMap<String, String>,

    /// Relevance score (0.0 - 1.0)
    pub relevance: f32,
}

impl MemoryItem {
    /// Create a new memory item
    pub fn new(
        id: impl Into<String>,
        item_type: MemoryItemType,
        content: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            item_type,
            timestamp_ms: chrono::Utc::now().timestamp_millis(),
            content: content.into(),
            metadata: HashMap::new(),
            relevance: 1.0,
        }
    }

    /// Create a memory item from an event
    pub fn from_event(event: &Event) -> Self {
        let summary = format!(
            "[{}] {} from {}: {}",
            event.r#type,
            event.id,
            event.source,
            Self::summarize_payload(&event.payload, 100)
        );

        let mut item = Self::new(&event.id, MemoryItemType::Event, summary);
        item.timestamp_ms = event.timestamp_ms;
        item.metadata
            .insert("event_type".to_string(), event.r#type.clone());
        item.metadata
            .insert("source".to_string(), event.source.clone());
        item
    }

    /// Create a memory item for an observation
    pub fn observation(tool_name: impl Into<String>, result: impl Into<String>) -> Self {
        let tool = tool_name.into();
        let content = format!("Tool '{}' returned: {}", tool, result.into());
        let mut item = Self::new(
            format!(
                "obs_{}",
                chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
            ),
            MemoryItemType::Observation,
            content,
        );
        item.metadata.insert("tool".to_string(), tool);
        item
    }

    /// Create a memory item for a user message
    pub fn user_message(content: impl Into<String>) -> Self {
        Self::new(
            format!(
                "user_{}",
                chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
            ),
            MemoryItemType::UserMessage,
            content,
        )
    }

    /// Create a memory item for an agent response
    pub fn agent_response(content: impl Into<String>) -> Self {
        Self::new(
            format!(
                "agent_{}",
                chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
            ),
            MemoryItemType::AgentResponse,
            content,
        )
    }

    /// Add metadata to this item
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Set relevance score
    pub fn with_relevance(mut self, relevance: f32) -> Self {
        self.relevance = relevance.clamp(0.0, 1.0);
        self
    }

    /// Summarize payload bytes to a string
    fn summarize_payload(payload: &[u8], max_len: usize) -> String {
        if payload.is_empty() {
            return "<empty>".to_string();
        }

        match std::str::from_utf8(payload) {
            Ok(s) => {
                let trimmed = s.trim();
                if trimmed.len() <= max_len {
                    trimmed.to_string()
                } else {
                    let mut end = max_len.saturating_sub(1);
                    while end > 0 && !trimmed.is_char_boundary(end) {
                        end -= 1;
                    }
                    format!("{}…", &trimmed[..end])
                }
            }
            Err(_) => format!("<{} bytes>", payload.len()),
        }
    }
}

/// Working memory for a cognitive agent.
///
/// This is a bounded buffer that holds recent context for the current
/// cognitive cycle. Items are automatically evicted when the capacity
/// is reached (FIFO).
#[derive(Debug, Clone, Default)]
pub struct WorkingMemory {
    /// Memory items in chronological order
    items: VecDeque<MemoryItem>,

    /// Maximum number of items to keep
    capacity: usize,

    /// Current task state (key-value pairs)
    task_state: HashMap<String, String>,

    /// Session ID for this memory context
    session_id: Option<String>,
}

impl WorkingMemory {
    /// Create a new working memory with the given capacity
    pub fn new(capacity: usize) -> Self {
        Self {
            items: VecDeque::with_capacity(capacity),
            capacity,
            task_state: HashMap::new(),
            session_id: None,
        }
    }

    /// Create working memory with a session ID
    pub fn with_session(capacity: usize, session_id: impl Into<String>) -> Self {
        Self {
            items: VecDeque::with_capacity(capacity),
            capacity,
            task_state: HashMap::new(),
            session_id: Some(session_id.into()),
        }
    }

    /// Get the session ID
    pub fn session_id(&self) -> Option<&str> {
        self.session_id.as_deref()
    }

    /// Set the session ID
    pub fn set_session_id(&mut self, session_id: impl Into<String>) {
        self.session_id = Some(session_id.into());
    }

    /// Add an item to memory
    pub fn add(&mut self, item: MemoryItem) {
        // Evict oldest item if at capacity
        while self.items.len() >= self.capacity {
            self.items.pop_front();
        }
        self.items.push_back(item);
    }

    /// Add a summary of an event to memory
    pub fn add_event_summary(&mut self, event: &Event) {
        self.add(MemoryItem::from_event(event));
    }

    /// Add an observation to memory
    pub fn add_observation(&mut self, tool_name: impl Into<String>, result: impl Into<String>) {
        self.add(MemoryItem::observation(tool_name, result));
    }

    /// Add a user message to memory
    pub fn add_user_message(&mut self, content: impl Into<String>) {
        self.add(MemoryItem::user_message(content));
    }

    /// Add an agent response to memory
    pub fn add_agent_response(&mut self, content: impl Into<String>) {
        self.add(MemoryItem::agent_response(content));
    }

    /// Get all items
    pub fn items(&self) -> &VecDeque<MemoryItem> {
        &self.items
    }

    /// Get the most recent N items
    pub fn recent(&self, n: usize) -> Vec<&MemoryItem> {
        self.items.iter().rev().take(n).collect()
    }

    /// Get items by type
    pub fn items_by_type(&self, item_type: MemoryItemType) -> Vec<&MemoryItem> {
        self.items
            .iter()
            .filter(|item| item.item_type == item_type)
            .collect()
    }

    /// Get the number of items
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Check if memory is empty
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Clear all items
    pub fn clear(&mut self) {
        self.items.clear();
        self.task_state.clear();
    }

    /// Set a task state value
    pub fn set_state(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.task_state.insert(key.into(), value.into());
    }

    /// Get a task state value
    pub fn get_state(&self, key: &str) -> Option<&String> {
        self.task_state.get(key)
    }

    /// Remove a task state value
    pub fn remove_state(&mut self, key: &str) -> Option<String> {
        self.task_state.remove(key)
    }

    /// Get all task state
    pub fn task_state(&self) -> &HashMap<String, String> {
        &self.task_state
    }

    /// Format memory items as context for LLM
    pub fn to_context_string(&self) -> String {
        if self.items.is_empty() {
            return String::new();
        }

        let mut lines = Vec::with_capacity(self.items.len());
        for item in &self.items {
            let prefix = match item.item_type {
                MemoryItemType::Event => "[Event]",
                MemoryItemType::Observation => "[Observation]",
                MemoryItemType::UserMessage => "[User]",
                MemoryItemType::AgentResponse => "[Agent]",
                MemoryItemType::System => "[System]",
                MemoryItemType::Custom => "[Note]",
            };
            lines.push(format!("{} {}", prefix, item.content));
        }

        lines.join("\n")
    }

    /// Search memory items by content (simple substring match)
    pub fn search(&self, query: &str) -> Vec<&MemoryItem> {
        let query_lower = query.to_lowercase();
        self.items
            .iter()
            .filter(|item| item.content.to_lowercase().contains(&query_lower))
            .collect()
    }
}
