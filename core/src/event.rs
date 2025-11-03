// Event bus implementation
use crate::Result;
use async_trait::async_trait;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc};
use tracing::{debug, info, warn};

pub use crate::proto::{Event, QoSLevel};

/// Event handler trait
#[async_trait]
pub trait EventHandler: Send + Sync {
    async fn handle(&self, event: Event) -> Result<()>;
}

/// Subscription information
#[derive(Debug, Clone)]
struct Subscription {
    id: String,
    topic: String,
    event_types: Vec<String>,
    qos: QoSLevel,
    sender: mpsc::Sender<Event>,
}

/// Event bus statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EventBusStats {
    pub total_published: u64,
    pub total_delivered: u64,
    pub active_subscriptions: usize,
    pub backlog_size: usize,
    pub dropped_events: u64,
}

/// Event bus core implementation
pub struct EventBus {
    // Topic -> Subscriber list
    subscriptions: Arc<DashMap<String, Vec<Subscription>>>,

    // Broadcast channel for high priority events
    broadcast_tx: broadcast::Sender<Event>,

    // Statistics
    stats: Arc<DashMap<String, EventBusStats>>,

    // Backpressure threshold
    backpressure_threshold: usize,
}
impl EventBus {
    pub async fn new() -> Result<Self> {
        let (broadcast_tx, _) = broadcast::channel(1000);

        Ok(Self {
            subscriptions: Arc::new(DashMap::new()),
            broadcast_tx,
            stats: Arc::new(DashMap::new()),
            backpressure_threshold: 10000,
        })
    }

    pub async fn start(&self) -> Result<()> {
        info!("Event Bus started");
        Ok(())
    }

    pub async fn shutdown(&self) -> Result<()> {
        info!("Event Bus shutting down");
        self.subscriptions.clear();
        Ok(())
    }

    /// Publish event to topic
    pub async fn publish(&self, topic: &str, event: Event) -> Result<u64> {
        debug!("Publishing event {} to topic {}", event.id, topic);

        // Update stats
        self.update_stats(topic, |stats| {
            stats.total_published += 1;
        });

        // Get subscribers
        if let Some(subs) = self.subscriptions.get(topic) {
            let mut delivered = 0;
            let mut dropped = 0;

            for sub in subs.value() {
                // Check event type filtering
                if !sub.event_types.is_empty() && !sub.event_types.contains(&event.r#type) {
                    continue;
                }

                // Handle based on QoS level
                match sub.qos {
                    QoSLevel::QosRealtime => {
                        // Realtime mode: try to send, drop if fails
                        if sub.sender.try_send(event.clone()).is_ok() {
                            delivered += 1;
                        } else {
                            dropped += 1;
                            warn!("Dropped realtime event for subscription {}", sub.id);
                        }
                    }
                    QoSLevel::QosBatched | QoSLevel::QosBackground => {
                        // Batch/background mode: wait to send
                        match sub.sender.send(event.clone()).await {
                            Ok(_) => delivered += 1,
                            Err(_) => {
                                dropped += 1;
                                warn!("Failed to send event to subscription {}", sub.id);
                            }
                        }
                    }
                }
            }

            self.update_stats(topic, |stats| {
                stats.total_delivered += delivered;
                stats.dropped_events += dropped;
            });

            Ok(delivered)
        } else {
            warn!("No subscriptions for topic: {}", topic);
            Ok(0)
        }
    }

    /// Subscribe to topic
    pub async fn subscribe(
        &self,
        topic: String,
        event_types: Vec<String>,
        qos: QoSLevel,
    ) -> Result<(String, mpsc::Receiver<Event>)> {
        let subscription_id = format!("sub_{}_{}", topic, uuid::Uuid::new_v4());
        let (tx, rx) = mpsc::channel(1000);

        let subscription = Subscription {
            id: subscription_id.clone(),
            topic: topic.clone(),
            event_types,
            qos,
            sender: tx,
        };

        self.subscriptions
            .entry(topic.clone())
            .or_insert_with(Vec::new)
            .push(subscription);

        self.update_stats(&topic, |stats| {
            stats.active_subscriptions += 1;
        });

        info!(
            "Created subscription {} for topic {}",
            subscription_id, topic
        );
        Ok((subscription_id, rx))
    }

    /// Unsubscribe from topic
    pub async fn unsubscribe(&self, subscription_id: &str) -> Result<()> {
        for mut entry in self.subscriptions.iter_mut() {
            let topic = entry.key().clone();
            entry.value_mut().retain(|sub| sub.id != subscription_id);

            self.update_stats(&topic, |stats| {
                stats.active_subscriptions = stats.active_subscriptions.saturating_sub(1);
            });
        }

        info!("Unsubscribed {}", subscription_id);
        Ok(())
    }

    /// Get stats
    pub fn get_stats(&self, topic: &str) -> Option<EventBusStats> {
        self.stats.get(topic).map(|s| s.clone())
    }

    // Update stats helper function
    fn update_stats<F>(&self, topic: &str, f: F)
    where
        F: FnOnce(&mut EventBusStats),
    {
        self.stats
            .entry(topic.to_string())
            .or_insert_with(EventBusStats::default)
            .value_mut()
            .apply(f);
    }
}

// Helper trait for chaining calls
trait Apply {
    fn apply<F>(&mut self, f: F)
    where
        F: FnOnce(&mut Self);
}

impl<T> Apply for T {
    fn apply<F>(&mut self, f: F)
    where
        F: FnOnce(&mut Self),
    {
        f(self)
    }
}

// UUID generation placeholder
mod uuid {
    pub struct Uuid;
    impl Uuid {
        pub fn new_v4() -> String {
            use std::time::{SystemTime, UNIX_EPOCH};
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            format!("{:x}", now)
        }
    }
}
