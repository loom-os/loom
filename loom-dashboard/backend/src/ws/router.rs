//! Chat Router Module
//!
//! Routes chat requests from WebSocket clients to appropriate agents via EventBus.
//!
//! # Architecture
//!
//! ```text
//! WebSocket Client
//!     │
//!     ├─ ChatRequest(agent_id, content)
//!     │
//!     ▼
//! ChatRouter
//!     │
//!     ├─ Create stream.request Envelope
//!     ├─ Publish to agent's input_topic
//!     ├─ Subscribe to reply_topic
//!     │
//!     ▼
//! EventBus ──────────▶ Target Agent
//!     │                     │
//!     │                     ├─ Process request
//!     │                     ├─ Generate chunks
//!     │                     │
//!     ◀─────────────────────┘
//!     │
//!     ├─ stream.chunk events
//!     ├─ stream.complete event
//!     │
//!     ▼
//! ChatRouter ────────▶ WebSocket Client
//! ```
//!
//! # Stream Lifecycle
//!
//! 1. Client sends ChatRequest with thread_id and agent_id
//! 2. Router creates unique correlation_id
//! 3. Router subscribes to reply_topic: `agent.dashboard.replies.{correlation_id}`
//! 4. Router publishes stream.request to agent's input_topic
//! 5. Agent processes and sends stream.chunk events
//! 6. Router forwards chunks to WebSocket connection
//! 7. Agent sends stream.complete
//! 8. Router unsubscribes and cleans up
//!
//! # Error Handling
//!
//! - Agent not found → Send error to client
//! - Timeout → Send error to client after 120s
//! - Agent disconnected during stream → Send error to client

use crate::ws::connection::ConnectionManager;
use crate::ws::message::WsMessage;
use dashmap::DashMap;
use loom_core::agent::directory::AgentDirectory;
use loom_core::messaging::event_bus::EventBus;
use loom_core::proto::{Event, QoSLevel};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Active chat session tracking
#[derive(Debug)]
struct ChatSession {
    /// Thread ID for this conversation
    thread_id: String,
    /// Correlation ID for this stream
    correlation_id: String,
    /// Target agent ID
    agent_id: String,
    /// Reply topic for this stream
    reply_topic: String,
    /// EventBus subscription ID for cleanup
    subscription_id: String,
    /// WebSocket connection ID to send responses
    connection_id: String,
    /// Sequence counter for chunks
    sequence: u32,
    /// Timestamp of stream start
    started_at: i64,
}

/// Chat routing manager
pub struct ChatRouter {
    event_bus: Arc<EventBus>,
    agent_directory: Arc<AgentDirectory>,
    connection_manager: Arc<ConnectionManager>,
    /// Active sessions: correlation_id -> ChatSession
    sessions: Arc<DashMap<String, ChatSession>>,
}

impl ChatRouter {
    /// Create a new ChatRouter
    pub fn new(
        event_bus: Arc<EventBus>,
        agent_directory: Arc<AgentDirectory>,
        connection_manager: Arc<ConnectionManager>,
    ) -> Self {
        Self {
            event_bus,
            agent_directory,
            connection_manager,
            sessions: Arc::new(DashMap::new()),
        }
    }

    /// Handle a chat request from WebSocket
    ///
    /// # Arguments
    ///
    /// * `connection_id` - WebSocket connection ID
    /// * `thread_id` - Conversation thread ID
    /// * `content` - Message content
    /// * `agent_id` - Target agent ID (optional, uses default if not provided)
    ///
    /// # Returns
    ///
    /// Ok if request was successfully routed, Err otherwise
    pub async fn handle_chat_request(
        &self,
        connection_id: String,
        thread_id: String,
        content: String,
        agent_id: Option<String>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Determine target agent
        let target_agent = match agent_id {
            Some(id) => id,
            None => {
                // Try to find first cognitive agent
                let agents = self.agent_directory.all();
                let cognitive_agents: Vec<_> = agents
                    .into_iter()
                    .filter(|agent| {
                        agent.subscribed_topics.iter().any(|topic| {
                            topic.contains("input") || topic.contains("chat")
                        }) || agent.metadata.get("type").map(|t| t == "cognitive").unwrap_or(false)
                    })
                    .collect();

                if cognitive_agents.is_empty() {
                    self.send_error(
                        &connection_id,
                        &thread_id,
                        "NO_AGENT",
                        "No cognitive agents available",
                    )
                    .await?;
                    return Err("No cognitive agents available".into());
                }

                cognitive_agents[0].agent_id.clone()
            }
        };

        // Get agent info
        let agent_info = self.agent_directory.get(&target_agent);
        if agent_info.is_none() {
            self.send_error(
                &connection_id,
                &thread_id,
                "AGENT_NOT_FOUND",
                &format!("Agent '{}' not found", target_agent),
            )
            .await?;
            return Err(format!("Agent '{}' not found", target_agent).into());
        }

        let agent_info = agent_info.unwrap();

        // Check agent status
        if agent_info.status != loom_core::agent::directory::AgentStatus::Active
            && agent_info.status != loom_core::agent::directory::AgentStatus::Idle
        {
            self.send_error(
                &connection_id,
                &thread_id,
                "AGENT_UNAVAILABLE",
                &format!("Agent '{}' is not available (status: {:?})", target_agent, agent_info.status),
            )
            .await?;
            return Err(format!("Agent '{}' is not available", target_agent).into());
        }

        // Find agent's input topic
        let input_topic = agent_info
            .subscribed_topics
            .iter()
            .find(|t| t.contains("input") || t.contains("chat"))
            .cloned();

        if input_topic.is_none() {
            self.send_error(
                &connection_id,
                &thread_id,
                "NO_INPUT_TOPIC",
                &format!("Agent '{}' has no input topic", target_agent),
            )
            .await?;
            return Err(format!("Agent '{}' has no input topic", target_agent).into());
        }

        let input_topic = input_topic.unwrap();

        // Create correlation ID and reply topic
        let correlation_id = Uuid::new_v4().to_string();
        let reply_topic = format!("agent.dashboard.replies.{}", correlation_id);

        info!(
            connection_id = %connection_id,
            thread_id = %thread_id,
            agent_id = %target_agent,
            correlation_id = %correlation_id,
            input_topic = %input_topic,
            reply_topic = %reply_topic,
            "Routing chat request"
        );

        // Subscribe to reply topic first
        let (subscription_id, reply_receiver) = self
            .event_bus
            .subscribe(reply_topic.clone(), vec![], QoSLevel::QosRealtime)
            .await?;

        // Store session
        let session = ChatSession {
            thread_id: thread_id.clone(),
            correlation_id: correlation_id.clone(),
            agent_id: target_agent.clone(),
            reply_topic: reply_topic.clone(),
            subscription_id: subscription_id.clone(),
            connection_id: connection_id.clone(),
            sequence: 0,
            started_at: chrono::Utc::now().timestamp_millis(),
        };

        self.sessions.insert(correlation_id.clone(), session);

        // Spawn task to handle replies
        let router = self.clone();
        let correlation_id_for_task = correlation_id.clone();
        tokio::spawn(async move {
            router
                .handle_reply_stream(correlation_id_for_task, reply_receiver)
                .await;
        });

        // Create and send stream.request event
        let mut metadata = std::collections::HashMap::new();
        metadata.insert("sender".to_string(), "dashboard".to_string());
        metadata.insert("reply_to".to_string(), reply_topic.clone());
        metadata.insert("correlation_id".to_string(), correlation_id.clone());
        metadata.insert("thread_id".to_string(), thread_id.clone());

        let event = Event {
            id: Uuid::new_v4().to_string(),
            r#type: "stream.request".to_string(),
            source: "dashboard".to_string(),
            timestamp_ms: chrono::Utc::now().timestamp_millis(),
            metadata,
            payload: content.into_bytes(),
            ..Default::default()
        };

        // Publish to agent's input topic
        match self.event_bus.publish(&input_topic, event).await {
            Ok(_) => {
                debug!(
                    correlation_id = %correlation_id,
                    "Published stream.request to agent"
                );
                Ok(())
            }
            Err(e) => {
                error!(
                    correlation_id = %correlation_id,
                    error = %e,
                    "Failed to publish stream.request"
                );
                self.cleanup_session(&correlation_id).await;
                self.send_error(
                    &connection_id,
                    &thread_id,
                    "PUBLISH_FAILED",
                    &format!("Failed to send request: {}", e),
                )
                .await?;
                Err(e.into())
            }
        }
    }

    /// Handle reply stream from agent
    async fn handle_reply_stream(&self, correlation_id: String, mut receiver: mpsc::Receiver<Event>) {
        let timeout = tokio::time::Duration::from_secs(120);
        let deadline = tokio::time::Instant::now() + timeout;

        loop {
            match tokio::time::timeout_at(deadline, receiver.recv()).await {
                Ok(Some(event)) => {
                    if let Err(e) = self.handle_reply_event(&correlation_id, event).await {
                        error!(
                            correlation_id = %correlation_id,
                            error = %e,
                            "Failed to handle reply event"
                        );
                        break;
                    }

                    // Check if stream is complete
                    if !self.sessions.contains_key(&correlation_id) {
                        break;
                    }
                }
                Ok(None) => {
                    // Channel closed
                    warn!(correlation_id = %correlation_id, "Reply channel closed");
                    break;
                }
                Err(_) => {
                    // Timeout
                    warn!(correlation_id = %correlation_id, "Stream timeout");
                    if let Some(session) = self.sessions.get(&correlation_id) {
                        let _ = self
                            .send_error(
                                &session.connection_id,
                                &session.thread_id,
                                "TIMEOUT",
                                "Stream timed out after 120s",
                            )
                            .await;
                    }
                    break;
                }
            }
        }

        // Cleanup
        self.cleanup_session(&correlation_id).await;
    }

    /// Handle a single reply event
    async fn handle_reply_event(
        &self,
        correlation_id: &str,
        event: Event,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut session = match self.sessions.get_mut(correlation_id) {
            Some(s) => s,
            None => {
                warn!(correlation_id = %correlation_id, "Session not found");
                return Ok(());
            }
        };

        match event.r#type.as_str() {
            "stream.chunk" => {
                // Forward chunk to WebSocket
                let content = String::from_utf8_lossy(&event.payload).to_string();
                let content_type = event
                    .metadata
                    .get("stream.content_type")
                    .map(|s| s.as_str())
                    .unwrap_or("text");

                let message = WsMessage::ChatChunk {
                    thread_id: session.thread_id.clone(),
                    content,
                    content_type: content_type.to_string(),
                    sequence: session.sequence,
                };

                session.sequence += 1;

                let _ = self.connection_manager.send_to(&session.connection_id, message);
            }

            "stream.complete" => {
                // Send completion
                let duration_ms = event
                    .metadata
                    .get("stream.duration_ms")
                    .and_then(|s| s.parse::<i32>().ok())
                    .unwrap_or(0);

                let total_tokens = event
                    .metadata
                    .get("stream.total_tokens")
                    .and_then(|s| s.parse::<i32>().ok())
                    .unwrap_or(0);

                let tool_calls = event
                    .metadata
                    .get("stream.tool_calls")
                    .and_then(|s| s.parse::<i32>().ok())
                    .unwrap_or(0);

                let message = WsMessage::ChatComplete {
                    thread_id: session.thread_id.clone(),
                    stats: super::message::StreamStats {
                        duration_ms: duration_ms as u64,
                        total_tokens: total_tokens as u32,
                        tool_calls: tool_calls as u32,
                    },
                };

                let _ = self.connection_manager.send_to(&session.connection_id, message);

                // Mark for cleanup
                drop(session);
                self.sessions.remove(correlation_id);
            }

            _ => {
                debug!(
                    correlation_id = %correlation_id,
                    event_type = %event.r#type,
                    "Ignoring unknown event type"
                );
            }
        }

        Ok(())
    }

    /// Send error message to WebSocket
    async fn send_error(
        &self,
        connection_id: &str,
        thread_id: &str,
        code: &str,
        message: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let error_msg = WsMessage::Error {
            code: code.to_string(),
            message: message.to_string(),
        };

        let _ = self.connection_manager.send_to(connection_id, error_msg);

        // Also send as chat message for better UX
        let chat_msg = WsMessage::ChatChunk {
            thread_id: thread_id.to_string(),
            content: format!("❌ Error: {}", message),
            content_type: "error".to_string(),
            sequence: 0,
        };

        let _ = self.connection_manager.send_to(connection_id, chat_msg);

        Ok(())
    }

    /// Cleanup session and unsubscribe
    async fn cleanup_session(&self, correlation_id: &str) {
        if let Some((_, session)) = self.sessions.remove(correlation_id) {
            // Unsubscribe from EventBus
            if let Err(e) = self
                .event_bus
                .unsubscribe(&session.subscription_id)
                .await
            {
                warn!(
                    correlation_id = %correlation_id,
                    error = %e,
                    "Failed to unsubscribe from reply topic"
                );
            }

            debug!(
                correlation_id = %correlation_id,
                duration_ms = chrono::Utc::now().timestamp_millis() - session.started_at,
                "Session cleaned up"
            );
        }
    }
}

// Make ChatRouter clonable for spawning tasks
impl Clone for ChatRouter {
    fn clone(&self) -> Self {
        Self {
            event_bus: Arc::clone(&self.event_bus),
            agent_directory: Arc::clone(&self.agent_directory),
            connection_manager: Arc::clone(&self.connection_manager),
            sessions: Arc::clone(&self.sessions),
        }
    }
}
