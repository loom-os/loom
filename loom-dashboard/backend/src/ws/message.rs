//! WebSocket Message Protocol
//!
//! Defines the message types and protocol for WebSocket communication
//! between the Dashboard frontend and backend.
//!
//! # Message Flow
//!
//! ```text
//! Browser                    Dashboard Server              EventBus
//!    │                              │                         │
//!    ├─── ChatRequest ──────────────▶                         │
//!    │                              ├──── stream.request ────▶
//!    │                              │                         │
//!    │◀──── ChatChunk ──────────────┤◀─── stream.chunk ──────┤
//!    │◀──── ChatChunk ──────────────┤◀─── stream.chunk ──────┤
//!    │◀──── ChatComplete ───────────┤◀─── stream.complete ───┤
//!    │                              │                         │
//!    │◀──── EventStream ────────────┤◀─── dashboard.event ───┤
//!    │◀──── TopologyUpdate ─────────┤                         │
//! ```
//!
//! # Message Types
//!
//! ## Client → Server
//! - `ChatRequest`: Send a chat message
//! - `PermissionResponse`: Respond to tool permission request
//! - `Subscribe`: Subscribe to specific topics
//!
//! ## Server → Client
//! - `ChatChunk`: Streaming chat response chunk
//! - `ChatComplete`: Chat completion with stats
//! - `EventStream`: Real-time event from EventBus
//! - `TopologyUpdate`: Agent topology changes
//! - `MetricsUpdate`: Performance metrics
//! - `Error`: Error message

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// WebSocket message envelope
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WsMessage {
    // Client → Server messages
    /// Chat request
    ChatRequest {
        /// Thread ID for conversation continuity
        thread_id: String,
        /// Message content
        content: String,
        /// Optional settings override
        #[serde(skip_serializing_if = "Option::is_none")]
        settings: Option<ChatSettings>,
    },

    /// Permission response (approve/deny tool execution)
    PermissionResponse {
        /// Request ID from permission request
        request_id: String,
        /// Whether permission is granted
        approved: bool,
        /// Optional reason for denial
        #[serde(skip_serializing_if = "Option::is_none")]
        reason: Option<String>,
    },

    /// Subscribe to specific event topics
    Subscribe {
        /// Topics to subscribe to
        topics: Vec<String>,
    },

    // Server → Client messages
    /// Streaming chat chunk
    ChatChunk {
        /// Thread ID
        thread_id: String,
        /// Chunk content
        content: String,
        /// Content type (text, thinking, tool_call, etc.)
        content_type: String,
        /// Sequence number
        sequence: u32,
    },

    /// Chat completion
    ChatComplete {
        /// Thread ID
        thread_id: String,
        /// Completion statistics
        stats: StreamStats,
    },

    /// Permission request from agent
    PermissionRequest {
        /// Unique request ID
        request_id: String,
        /// Tool name
        tool_name: String,
        /// Tool arguments
        args: HashMap<String, serde_json::Value>,
        /// Reason for tool use
        reason: String,
    },

    /// Event stream from EventBus
    EventStream {
        /// Event ID
        event_id: String,
        /// Event timestamp
        timestamp: String,
        /// Event topic
        topic: String,
        /// Sender agent ID
        #[serde(skip_serializing_if = "Option::is_none")]
        sender: Option<String>,
        /// Thread ID
        #[serde(skip_serializing_if = "Option::is_none")]
        thread_id: Option<String>,
        /// Payload preview
        payload_preview: String,
    },

    /// Topology update
    TopologyUpdate {
        /// List of active agents
        agents: Vec<AgentInfo>,
    },

    /// Metrics update
    MetricsUpdate {
        /// Events per second
        events_per_sec: f64,
        /// Active agent count
        active_agents: usize,
        /// Average latency in milliseconds
        avg_latency_ms: f64,
    },

    /// Error message
    Error {
        /// Error code
        code: String,
        /// Error message
        message: String,
    },
}

/// Chat settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatSettings {
    /// Model name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    /// Temperature (0.0 - 2.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,

    /// Maximum tokens
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,

    /// System prompt override
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,
}

/// Stream statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamStats {
    /// Duration in milliseconds
    pub duration_ms: u64,
    /// Total tokens used
    pub total_tokens: u32,
    /// Number of tool calls
    pub tool_calls: u32,
}

/// Agent information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    /// Agent ID
    pub id: String,
    /// Topics subscribed
    pub topics: Vec<String>,
    /// Agent capabilities
    pub capabilities: Vec<String>,
}

impl WsMessage {
    /// Create a chat chunk message
    pub fn chat_chunk(
        thread_id: String,
        content: String,
        content_type: String,
        sequence: u32,
    ) -> Self {
        Self::ChatChunk {
            thread_id,
            content,
            content_type,
            sequence,
        }
    }

    /// Create a chat complete message
    pub fn chat_complete(thread_id: String, stats: StreamStats) -> Self {
        Self::ChatComplete { thread_id, stats }
    }

    /// Create an error message
    pub fn error(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Error {
            code: code.into(),
            message: message.into(),
        }
    }

    /// Create an event stream message
    pub fn event_stream(
        event_id: String,
        timestamp: String,
        topic: String,
        sender: Option<String>,
        thread_id: Option<String>,
        payload_preview: String,
    ) -> Self {
        Self::EventStream {
            event_id,
            timestamp,
            topic,
            sender,
            thread_id,
            payload_preview,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_chat_request() {
        let msg = WsMessage::ChatRequest {
            thread_id: "thread-123".to_string(),
            content: "Hello".to_string(),
            settings: None,
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("chat_request"));
        assert!(json.contains("thread-123"));
    }

    #[test]
    fn test_deserialize_chat_request() {
        let json = r#"{"type":"chat_request","thread_id":"thread-123","content":"Hello"}"#;
        let msg: WsMessage = serde_json::from_str(json).unwrap();

        match msg {
            WsMessage::ChatRequest {
                thread_id, content, ..
            } => {
                assert_eq!(thread_id, "thread-123");
                assert_eq!(content, "Hello");
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_chat_chunk_helper() {
        let msg = WsMessage::chat_chunk(
            "thread-123".to_string(),
            "Hello".to_string(),
            "text".to_string(),
            0,
        );

        match msg {
            WsMessage::ChatChunk {
                thread_id, content, ..
            } => {
                assert_eq!(thread_id, "thread-123");
                assert_eq!(content, "Hello");
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_error_helper() {
        let msg = WsMessage::error("NOT_FOUND", "Resource not found");

        match msg {
            WsMessage::Error { code, message } => {
                assert_eq!(code, "NOT_FOUND");
                assert_eq!(message, "Resource not found");
            }
            _ => panic!("Wrong message type"),
        }
    }
}
