//! EventBus Bridge
//!
//! Connects WebSocket messages to the EventBus, enabling
//! bidirectional communication between frontend and agents.

use super::connection::ConnectionManager;
use super::message::WsMessage;
use loom_core::messaging::event_bus::EventBus;
use loom_core::proto::{Event, QoSLevel};
use std::sync::Arc;
use tracing::{debug, error, info};

/// EventBus bridge for WebSocket integration
pub struct EventBusBridge {
    event_bus: Arc<EventBus>,
    connection_manager: Arc<ConnectionManager>,
}

impl EventBusBridge {
    /// Create a new EventBus bridge
    pub fn new(event_bus: Arc<EventBus>, connection_manager: Arc<ConnectionManager>) -> Self {
        Self {
            event_bus,
            connection_manager,
        }
    }

    /// Start the bridge, forwarding events from EventBus to WebSocket clients
    pub async fn start(self: Arc<Self>) -> anyhow::Result<()> {
        info!("Starting EventBus bridge");

        // Subscribe to all events on "dashboard" topic
        let (_subscription_id, mut receiver) = self
            .event_bus
            .subscribe("dashboard".to_string(), vec![], QoSLevel::QosRealtime)
            .await?;

        // Forward events to WebSocket clients
        tokio::spawn(async move {
            loop {
                match receiver.recv().await {
                    Some(event) => {
                        if let Err(e) = self.handle_event(event).await {
                            error!(error = %e, "Failed to handle event");
                        }
                    }
                    None => {
                        info!("EventBus channel closed, stopping bridge");
                        break;
                    }
                }
            }
        });

        Ok(())
    }

    /// Handle an event from EventBus
    async fn handle_event(&self, event: Event) -> anyhow::Result<()> {
        debug!(
            event_id = %event.id,
            event_type = %event.r#type,
            "Processing event"
        );

        // Convert EventBus message to WebSocket message
        let ws_message = self.convert_event_to_ws(&event)?;

        // Broadcast to all connected clients
        self.connection_manager.broadcast(ws_message);

        Ok(())
    }

    /// Convert EventBus event to WebSocket message
    fn convert_event_to_ws(&self, event: &Event) -> anyhow::Result<WsMessage> {
        // Convert proto Event to EventStream WsMessage
        let payload_preview = if !event.payload.is_empty() {
            format!(
                "{}... ({} bytes)",
                String::from_utf8_lossy(&event.payload[..event.payload.len().min(50)]),
                event.payload.len()
            )
        } else {
            String::new()
        };

        // Extract thread_id from metadata if present
        let thread_id = event.metadata.get("thread_id").cloned();
        let sender = event.metadata.get("agent_id").cloned();

        Ok(WsMessage::EventStream {
            event_id: event.id.clone(),
            timestamp: chrono::DateTime::from_timestamp_millis(event.timestamp_ms)
                .unwrap_or_default()
                .to_rfc3339(),
            topic: event.source.clone(),
            sender,
            thread_id,
            payload_preview,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[tokio::test]
    async fn test_bridge_creation() {
        let event_bus = Arc::new(EventBus::new().await.unwrap());
        let connection_manager = Arc::new(ConnectionManager::new());

        let _bridge = EventBusBridge::new(event_bus, connection_manager);
    }

    #[tokio::test]
    async fn test_event_conversion() {
        let event_bus = Arc::new(EventBus::new().await.unwrap());
        let connection_manager = Arc::new(ConnectionManager::new());
        let bridge = EventBusBridge::new(event_bus, connection_manager);

        let mut metadata = std::collections::HashMap::new();
        metadata.insert("agent_id".to_string(), "agent-1".to_string());
        metadata.insert("thread_id".to_string(), "thread-123".to_string());

        let event = Event {
            id: uuid::Uuid::new_v4().to_string(),
            r#type: "test.event".to_string(),
            source: "test".to_string(),
            timestamp_ms: Utc::now().timestamp_millis(),
            metadata,
            payload: b"test payload".to_vec(),
            ..Default::default()
        };

        let ws_message = bridge.convert_event_to_ws(&event).unwrap();

        match ws_message {
            WsMessage::EventStream {
                event_id,
                topic,
                sender,
                thread_id,
                ..
            } => {
                assert_eq!(event_id, event.id);
                assert_eq!(topic, "test");
                assert_eq!(sender, Some("agent-1".to_string()));
                assert_eq!(thread_id, Some("thread-123".to_string()));
            }
            _ => panic!("Expected EventStream message"),
        }
    }
}
