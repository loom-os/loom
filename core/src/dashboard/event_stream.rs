// Event streaming for Dashboard
//
// Uses tokio broadcast channel to stream events to multiple SSE clients

use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

/// Event sent to Dashboard clients
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DashboardEvent {
    /// Timestamp (ISO 8601)
    pub timestamp: String,
    /// Event type
    pub event_type: DashboardEventType,
    /// Event ID
    pub event_id: String,
    /// Topic
    pub topic: String,
    /// Sender agent ID
    pub sender: Option<String>,
    /// Thread ID
    pub thread_id: Option<String>,
    /// Correlation ID
    pub correlation_id: Option<String>,
    /// Payload preview (first 100 chars)
    pub payload_preview: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DashboardEventType {
    /// Event published to EventBus
    EventPublished,
    /// Event delivered to subscriber
    EventDelivered,
    /// Agent registered
    AgentRegistered,
    /// Agent unregistered
    AgentUnregistered,
    /// Tool invoked
    ToolInvoked,
    /// Routing decision
    RoutingDecision,
}

/// Event broadcaster for Dashboard
#[derive(Clone)]
pub struct EventBroadcaster {
    sender: broadcast::Sender<DashboardEvent>,
}

impl EventBroadcaster {
    /// Create a new broadcaster with buffer size
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }

    /// Broadcast an event to all subscribers
    pub fn broadcast(&self, event: DashboardEvent) {
        // Ignore error if no subscribers
        let _ = self.sender.send(event);
    }

    /// Subscribe to events
    pub fn subscribe(&self) -> broadcast::Receiver<DashboardEvent> {
        self.sender.subscribe()
    }

    /// Get number of active subscribers
    pub fn subscriber_count(&self) -> usize {
        self.sender.receiver_count()
    }
}

impl Default for EventBroadcaster {
    fn default() -> Self {
        Self::new(1000) // Buffer last 1000 events
    }
}
