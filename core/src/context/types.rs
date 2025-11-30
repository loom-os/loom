//! Core types for the context system.
//!
//! This module defines the fundamental types used throughout the context pipeline:
//! - ContextItem: The atomic unit of context
//! - ContextContent: The actual content and metadata
//! - MemoryQuery: Query interface for retrieving items

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// The atomic unit of context that can be stored, retrieved, and assembled into prompts.
///
/// Everything that enters the context pipeline is a ContextItem: messages, tool calls,
/// tool results, events, observations. Items are never deleted or summarized - they are
/// preserved in their original form for full traceability.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextItem {
    /// Unique identifier for this item
    pub id: String,

    /// Type of context item
    pub item_type: ContextItemType,

    /// The actual content
    pub content: ContextContent,

    /// Metadata about this item
    pub metadata: ContextMetadata,
}

impl ContextItem {
    /// Create a new ContextItem with a generated UUID
    pub fn new(
        item_type: ContextItemType,
        content: ContextContent,
        metadata: ContextMetadata,
    ) -> Self {
        // Generate a unique ID using timestamp + thread-local counter
        use std::sync::atomic::{AtomicU32, Ordering};
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let counter = COUNTER.fetch_add(1, Ordering::Relaxed);

        let id = format!("{}-{}", chrono::Utc::now().timestamp_millis(), counter);

        Self {
            id,
            item_type,
            content,
            metadata,
        }
    }

    /// Check if this item is a message
    pub fn is_message(&self) -> bool {
        matches!(self.item_type, ContextItemType::Message { .. })
    }

    /// Check if this item is a tool call
    pub fn is_tool_call(&self) -> bool {
        matches!(self.item_type, ContextItemType::ToolCall { .. })
    }

    /// Check if this item is a tool result
    pub fn is_tool_result(&self) -> bool {
        matches!(self.item_type, ContextItemType::ToolResult { .. })
    }

    /// Get the timestamp of this item
    pub fn timestamp(&self) -> i64 {
        self.metadata.timestamp_ms
    }
}

/// Types of context items
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ContextItemType {
    /// A message in the conversation (user, assistant, system)
    Message { role: MessageRole },

    /// A tool call made by the agent
    ToolCall { tool_name: String },

    /// The result of a tool execution
    ToolResult { tool_name: String, success: bool },

    /// An event from the event bus
    Event { event_type: String },

    /// An observation or perception from the environment
    Observation { source: String },
}

/// Roles for messages in conversation
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    System,
    User,
    Assistant,
}

/// The content of a context item.
///
/// Content includes both the raw data (never modified) and derived representations
/// like text and embeddings for retrieval.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextContent {
    /// Original content in JSON format (immutable)
    pub raw: Value,

    /// Text representation for display and retrieval
    pub text: String,

    /// Cached token count (computed lazily)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_count: Option<usize>,

    /// Vector embedding for semantic retrieval (computed lazily)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding: Option<Vec<f32>>,
}

impl ContextContent {
    /// Create content from a JSON value
    pub fn from_value(value: Value) -> Self {
        let text = match &value {
            Value::String(s) => s.clone(),
            v => v.to_string(),
        };

        Self {
            raw: value,
            text,
            token_count: None,
            embedding: None,
        }
    }

    /// Create content from a string
    pub fn from_string(text: String) -> Self {
        Self {
            raw: Value::String(text.clone()),
            text,
            token_count: None,
            embedding: None,
        }
    }

    /// Create content from an event
    pub fn from_event(event: &crate::proto::Event) -> Self {
        let text = format!(
            "[{}] {} from {}",
            event.timestamp_ms, event.r#type, event.source
        );

        // Extract relevant fields manually to avoid protobuf serialization issues
        let raw = serde_json::json!({
            "id": event.id,
            "type": event.r#type,
            "source": event.source,
            "timestamp_ms": event.timestamp_ms,
        });

        Self {
            raw,
            text,
            token_count: None,
            embedding: None,
        }
    }

    /// Create content from an action result
    pub fn from_action_result(result: &crate::proto::ActionResult) -> Self {
        let text = if result.status == crate::proto::ActionStatus::ActionOk as i32 {
            String::from_utf8_lossy(&result.output).to_string()
        } else {
            result
                .error
                .as_ref()
                .map(|e| format!("Error: {}", e.message))
                .unwrap_or_else(|| "Unknown error".to_string())
        };

        // Extract relevant fields manually
        let raw = serde_json::json!({
            "id": result.id,
            "status": result.status,
            "output": String::from_utf8_lossy(&result.output).to_string(),
            "error": result.error.as_ref().map(|e| serde_json::json!({
                "code": e.code,
                "message": &e.message,
            })),
        });

        Self {
            raw,
            text,
            token_count: None,
            embedding: None,
        }
    }
}

/// Metadata about a context item.
///
/// Includes timing, provenance, importance, and relationships to other items.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextMetadata {
    /// When this item was created (Unix timestamp in milliseconds)
    pub timestamp_ms: i64,

    /// Session this item belongs to
    pub session_id: String,

    /// Agent that created or received this item
    pub agent_id: String,

    /// Importance score (0.0 to 1.0) - higher means more important
    /// Used for ranking and selection when context window is limited
    pub importance: f32,

    /// IDs of related context items (e.g., tool result relates to tool call)
    pub related_items: Vec<String>,

    /// Custom tags for filtering and retrieval
    pub tags: HashMap<String, String>,

    /// OpenTelemetry trace ID for distributed tracing
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,

    /// OpenTelemetry span ID for distributed tracing
    #[serde(skip_serializing_if = "Option::is_none")]
    pub span_id: Option<String>,
}

impl ContextMetadata {
    /// Create metadata with current timestamp
    pub fn new(session_id: String, agent_id: String) -> Self {
        Self {
            timestamp_ms: chrono::Utc::now().timestamp_millis(),
            session_id,
            agent_id,
            importance: 0.5, // Default neutral importance
            related_items: Vec::new(),
            tags: HashMap::new(),
            trace_id: None,
            span_id: None,
        }
    }

    /// Set importance score
    pub fn with_importance(mut self, importance: f32) -> Self {
        self.importance = importance.clamp(0.0, 1.0);
        self
    }

    /// Add a related item ID
    pub fn with_related_item(mut self, item_id: String) -> Self {
        self.related_items.push(item_id);
        self
    }

    /// Add a tag
    pub fn with_tag(mut self, key: String, value: String) -> Self {
        self.tags.insert(key, value);
        self
    }

    /// Capture current OpenTelemetry context
    pub fn with_current_trace(mut self) -> Self {
        use opentelemetry::trace::TraceContextExt;

        let context = opentelemetry::Context::current();
        let span = context.span();
        let span_context = span.span_context();

        if span_context.is_valid() {
            self.trace_id = Some(span_context.trace_id().to_string());
            self.span_id = Some(span_context.span_id().to_string());
        }

        self
    }
}

/// Query for retrieving context items from memory
#[derive(Debug, Clone, Default)]
pub struct MemoryQuery {
    /// Filter by session ID
    pub session_id: Option<String>,

    /// Filter by agent ID
    pub agent_id: Option<String>,

    /// Filter by item types
    pub item_types: Option<Vec<ContextItemType>>,

    /// Filter by time range (start_ms, end_ms)
    pub time_range: Option<(i64, i64)>,

    /// Filter by tags (all must match)
    pub tags: Option<HashMap<String, String>>,

    /// Filter by minimum importance
    pub min_importance: Option<f32>,

    /// Maximum number of results
    pub limit: usize,

    /// Offset for pagination
    pub offset: usize,
}

impl MemoryQuery {
    /// Create a new query with default limit
    pub fn new() -> Self {
        Self {
            limit: 100,
            ..Default::default()
        }
    }

    /// Filter by session
    pub fn for_session(mut self, session_id: String) -> Self {
        self.session_id = Some(session_id);
        self
    }

    /// Filter by agent
    pub fn for_agent(mut self, agent_id: String) -> Self {
        self.agent_id = Some(agent_id);
        self
    }

    /// Filter by item types
    pub fn with_types(mut self, types: Vec<ContextItemType>) -> Self {
        self.item_types = Some(types);
        self
    }

    /// Filter by time range
    pub fn in_time_range(mut self, start_ms: i64, end_ms: i64) -> Self {
        self.time_range = Some((start_ms, end_ms));
        self
    }

    /// Set result limit
    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = limit;
        self
    }

    /// Set offset for pagination
    pub fn offset(mut self, offset: usize) -> Self {
        self.offset = offset;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_item_creation() {
        let metadata = ContextMetadata::new("session1".to_string(), "agent1".to_string())
            .with_importance(0.8)
            .with_tag("key".to_string(), "value".to_string());

        let content = ContextContent::from_string("test message".to_string());

        let item = ContextItem::new(
            ContextItemType::Message {
                role: MessageRole::User,
            },
            content,
            metadata,
        );

        assert!(item.is_message());
        assert!(!item.is_tool_call());
        assert_eq!(item.metadata.importance, 0.8);
        assert_eq!(item.metadata.tags.get("key").unwrap(), "value");
    }

    #[test]
    fn test_memory_query_builder() {
        let query = MemoryQuery::new()
            .for_session("session1".to_string())
            .for_agent("agent1".to_string())
            .with_types(vec![ContextItemType::Message {
                role: MessageRole::User,
            }])
            .limit(50);

        assert_eq!(query.session_id.unwrap(), "session1");
        assert_eq!(query.agent_id.unwrap(), "agent1");
        assert_eq!(query.limit, 50);
        assert_eq!(query.item_types.as_ref().unwrap().len(), 1);
    }

    #[test]
    fn test_context_content_from_string() {
        let content = ContextContent::from_string("Hello, world!".to_string());
        assert_eq!(content.text, "Hello, world!");
        assert!(matches!(content.raw, Value::String(_)));
    }

    #[test]
    fn test_importance_clamping() {
        let metadata =
            ContextMetadata::new("s1".to_string(), "a1".to_string()).with_importance(1.5); // Over 1.0

        assert_eq!(metadata.importance, 1.0);

        let metadata2 =
            ContextMetadata::new("s1".to_string(), "a1".to_string()).with_importance(-0.5); // Under 0.0

        assert_eq!(metadata2.importance, 0.0);
    }
}
