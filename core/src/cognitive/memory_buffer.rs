//! Simple memory buffer for cognitive loop.
//!
//! This is a minimal in-process memory buffer that replaces the deprecated
//! `WorkingMemory`. For persistent and advanced memory features, use
//! `context::AgentContext` instead.

use std::collections::VecDeque;

/// Type of memory item
#[derive(Debug, Clone, PartialEq)]
pub enum MemoryItemType {
    UserMessage,
    AgentResponse,
    ToolObservation,
    EventSummary,
}

/// A single item in the memory buffer
#[derive(Debug, Clone)]
pub struct MemoryItem {
    /// Type of memory item
    pub item_type: MemoryItemType,
    /// Content of the item
    pub content: String,
    /// Timestamp when added
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl MemoryItem {
    /// Create a new memory item
    pub fn new(item_type: MemoryItemType, content: impl Into<String>) -> Self {
        Self {
            item_type,
            content: content.into(),
            timestamp: chrono::Utc::now(),
        }
    }
}

/// Simple in-process memory buffer for cognitive loop.
///
/// Maintains a sliding window of recent items for context building.
/// For persistent storage, use `context::AgentContext` which integrates
/// with the full context pipeline.
#[derive(Debug)]
pub struct MemoryBuffer {
    /// Maximum number of items to keep
    max_items: usize,
    /// Items in chronological order
    items: VecDeque<MemoryItem>,
}

impl Default for MemoryBuffer {
    fn default() -> Self {
        Self::new(50)
    }
}

impl MemoryBuffer {
    /// Create a new memory buffer with specified capacity
    pub fn new(max_items: usize) -> Self {
        Self {
            max_items,
            items: VecDeque::with_capacity(max_items),
        }
    }

    /// Add a user message
    pub fn add_user_message(&mut self, content: &str) {
        self.add(MemoryItem::new(MemoryItemType::UserMessage, content));
    }

    /// Add an agent response
    pub fn add_agent_response(&mut self, content: &str) {
        self.add(MemoryItem::new(MemoryItemType::AgentResponse, content));
    }

    /// Add a tool observation
    pub fn add_observation(&mut self, tool_name: &str, output: &str) {
        let content = format!("[{}] {}", tool_name, output);
        self.add(MemoryItem::new(MemoryItemType::ToolObservation, content));
    }

    /// Add an event summary
    pub fn add_event_summary(&mut self, event: &crate::proto::Event) {
        let content = format!(
            "[Event: {}] {}",
            event.r#type,
            String::from_utf8_lossy(&event.payload)
                .chars()
                .take(200)
                .collect::<String>()
        );
        self.add(MemoryItem::new(MemoryItemType::EventSummary, content));
    }

    /// Add an item to the buffer
    fn add(&mut self, item: MemoryItem) {
        if self.items.len() >= self.max_items {
            self.items.pop_front();
        }
        self.items.push_back(item);
    }

    /// Get the most recent N items
    pub fn recent(&self, n: usize) -> Vec<&MemoryItem> {
        self.items.iter().rev().take(n).collect()
    }

    /// Convert to context string for prompts
    pub fn to_context_string(&self) -> String {
        self.items
            .iter()
            .map(|item| {
                let prefix = match item.item_type {
                    MemoryItemType::UserMessage => "User",
                    MemoryItemType::AgentResponse => "Assistant",
                    MemoryItemType::ToolObservation => "Tool",
                    MemoryItemType::EventSummary => "Event",
                };
                format!("{}: {}", prefix, item.content)
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Clear all items
    pub fn clear(&mut self) {
        self.items.clear();
    }

    /// Get the number of items
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_buffer_basic() {
        let mut buffer = MemoryBuffer::new(10);
        assert!(buffer.is_empty());

        buffer.add_user_message("Hello");
        buffer.add_agent_response("Hi there!");

        assert_eq!(buffer.len(), 2);

        let recent = buffer.recent(1);
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].item_type, MemoryItemType::AgentResponse);
    }

    #[test]
    fn test_memory_buffer_overflow() {
        let mut buffer = MemoryBuffer::new(3);

        buffer.add_user_message("1");
        buffer.add_user_message("2");
        buffer.add_user_message("3");
        buffer.add_user_message("4");

        assert_eq!(buffer.len(), 3);
        let recent = buffer.recent(3);
        assert!(recent[0].content.contains("4"));
    }

    #[test]
    fn test_to_context_string() {
        let mut buffer = MemoryBuffer::new(10);
        buffer.add_user_message("What's the weather?");
        buffer.add_agent_response("Let me check...");

        let context = buffer.to_context_string();
        assert!(context.contains("User:"));
        assert!(context.contains("Assistant:"));
    }
}
