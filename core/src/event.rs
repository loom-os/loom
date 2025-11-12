// Event bus implementation
use crate::Result;
use async_trait::async_trait;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{broadcast, mpsc};
use tracing::{debug, info, warn, Span};

pub use crate::proto::{Event, QoSLevel};

// OpenTelemetry imports
use opentelemetry::{
    global,
    metrics::{Counter, Histogram, UpDownCounter},
    KeyValue,
};

/// Extension trait for Event providing fluent helpers for envelope metadata.
///
/// Simplifies reading and writing envelope metadata without verbose Envelope::from_event() calls.
pub trait EventExt {
    /// Sets the thread_id metadata and returns self for chaining.
    fn with_thread(self, thread_id: String) -> Self;

    /// Sets the correlation_id metadata and returns self for chaining.
    fn with_correlation(self, correlation_id: String) -> Self;

    /// Sets the reply_to metadata and returns self for chaining.
    fn with_reply_to(self, reply_to: String) -> Self;

    /// Sets the sender metadata and returns self for chaining.
    fn with_sender(self, sender: String) -> Self;

    /// Reads thread_id from metadata.
    fn thread_id(&self) -> Option<&str>;

    /// Reads correlation_id from metadata.
    fn correlation_id(&self) -> Option<&str>;

    /// Reads reply_to from metadata.
    fn reply_to(&self) -> Option<&str>;

    /// Reads sender from metadata.
    fn sender(&self) -> Option<&str>;
}

impl EventExt for Event {
    fn with_thread(mut self, thread_id: String) -> Self {
        self.metadata
            .insert(crate::envelope::keys::THREAD_ID.to_string(), thread_id);
        self
    }

    fn with_correlation(mut self, correlation_id: String) -> Self {
        self.metadata.insert(
            crate::envelope::keys::CORRELATION_ID.to_string(),
            correlation_id,
        );
        self
    }

    fn with_reply_to(mut self, reply_to: String) -> Self {
        self.metadata
            .insert(crate::envelope::keys::REPLY_TO.to_string(), reply_to);
        self
    }

    fn with_sender(mut self, sender: String) -> Self {
        self.metadata
            .insert(crate::envelope::keys::SENDER.to_string(), sender);
        self
    }

    fn thread_id(&self) -> Option<&str> {
        self.metadata
            .get(crate::envelope::keys::THREAD_ID)
            .map(|s| s.as_str())
    }

    fn correlation_id(&self) -> Option<&str> {
        self.metadata
            .get(crate::envelope::keys::CORRELATION_ID)
            .map(|s| s.as_str())
    }

    fn reply_to(&self) -> Option<&str> {
        self.metadata
            .get(crate::envelope::keys::REPLY_TO)
            .map(|s| s.as_str())
    }

    fn sender(&self) -> Option<&str> {
        self.metadata
            .get(crate::envelope::keys::SENDER)
            .map(|s| s.as_str())
    }
}

/// Event handler trait
#[async_trait]
pub trait EventHandler: Send + Sync {
    async fn handle(&self, event: Event) -> Result<()>;
}

/// Subscription information
#[derive(Debug, Clone)]
struct Subscription {
    id: String,
    #[allow(dead_code)]
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
    #[allow(dead_code)]
    broadcast_tx: broadcast::Sender<Event>,

    // Statistics
    stats: Arc<DashMap<String, EventBusStats>>,

    // Backpressure threshold
    backpressure_threshold: usize,

    // Dashboard event broadcaster (optional)
    dashboard_broadcaster: Option<crate::dashboard::EventBroadcaster>,

    // OpenTelemetry metrics
    published_counter: Counter<u64>,
    delivered_counter: Counter<u64>,
    dropped_counter: Counter<u64>,
    backlog_gauge: UpDownCounter<i64>,
    active_subscriptions_gauge: UpDownCounter<i64>,
    publish_latency: Histogram<f64>,
}
impl EventBus {
    pub async fn new() -> Result<Self> {
        let (broadcast_tx, _) = broadcast::channel(1000);

        // Initialize OpenTelemetry metrics
        let meter = global::meter("loom.event_bus");

        let published_counter = meter
            .u64_counter("loom.event_bus.published_total")
            .with_description("Total number of events published")
            .init();

        let delivered_counter = meter
            .u64_counter("loom.event_bus.delivered_total")
            .with_description("Total number of events delivered to subscribers")
            .init();

        let dropped_counter = meter
            .u64_counter("loom.event_bus.dropped_total")
            .with_description("Total number of events dropped")
            .init();

        let backlog_gauge = meter
            .i64_up_down_counter("loom.event_bus.backlog_size")
            .with_description("Current backlog size per topic")
            .init();

        let active_subscriptions_gauge = meter
            .i64_up_down_counter("loom.event_bus.active_subscriptions")
            .with_description("Number of active subscriptions")
            .init();

        let publish_latency = meter
            .f64_histogram("loom.event_bus.publish_latency_ms")
            .with_description("Event publish latency in milliseconds")
            .init();

        Ok(Self {
            subscriptions: Arc::new(DashMap::new()),
            broadcast_tx,
            stats: Arc::new(DashMap::new()),
            backpressure_threshold: 10_000,
            dashboard_broadcaster: None,
            published_counter,
            delivered_counter,
            dropped_counter,
            backlog_gauge,
            active_subscriptions_gauge,
            publish_latency,
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

    /// Set dashboard broadcaster for real-time event streaming
    pub fn set_dashboard_broadcaster(&mut self, broadcaster: crate::dashboard::EventBroadcaster) {
        self.dashboard_broadcaster = Some(broadcaster);
    }

    /// Publish event to topic
    #[tracing::instrument(skip(self, event), fields(topic = %topic, event_id = %event.id, event_type = %event.r#type, qos_level = "unknown"))]
    pub async fn publish(&self, topic: &str, event: Event) -> Result<u64> {
        let start_time = Instant::now();

        debug!("Publishing event {} to topic {}", event.id, topic);

        // Broadcast to Dashboard (if enabled)
        if let Some(ref broadcaster) = self.dashboard_broadcaster {
            let payload_preview = String::from_utf8_lossy(&event.payload)
                .chars()
                .take(100)
                .collect::<String>();

            broadcaster.broadcast(crate::dashboard::DashboardEvent {
                timestamp: chrono::Utc::now().to_rfc3339(),
                event_type: crate::dashboard::DashboardEventType::EventPublished,
                event_id: event.id.clone(),
                topic: topic.to_string(),
                sender: event.sender().map(|s| s.to_string()),
                thread_id: event.thread_id().map(|s| s.to_string()),
                correlation_id: event.correlation_id().map(|s| s.to_string()),
                payload_preview,
            });
        }

        // Record published metric
        self.published_counter.add(
            1,
            &[
                KeyValue::new("topic", topic.to_string()),
                KeyValue::new("event_type", event.r#type.clone()),
            ],
        );

        // Update stats: published and backlog increase
        let mut over_threshold = false;
        let current_backlog = self
            .update_stats_and_get(topic, |stats| {
                stats.total_published += 1;
                stats.backlog_size = stats.backlog_size.saturating_add(1);
                stats.backlog_size
            })
            .unwrap_or(0);

        // Update backlog gauge
        self.backlog_gauge
            .add(1, &[KeyValue::new("topic", topic.to_string())]);

        if current_backlog >= self.backpressure_threshold {
            over_threshold = true;
            // Record backpressure event in span
            Span::current().record("backpressure", true);
            tracing::warn!(
                target: "event_bus",
                topic = %topic,
                backlog = current_backlog,
                threshold = self.backpressure_threshold,
                "Backpressure threshold exceeded"
            );
        }

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
                        // Realtime mode: drop aggressively when backpressured, and drop on full queue
                        if over_threshold {
                            dropped += 1;
                            continue;
                        }
                        if sub.sender.try_send(event.clone()).is_ok() {
                            delivered += 1;
                        } else {
                            dropped += 1;
                            warn!("Dropped realtime event for subscription {}", sub.id);
                        }
                    }
                    QoSLevel::QosBatched | QoSLevel::QosBackground => {
                        // Batch/background mode: queue (bounded mpsc); await if necessary
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
                stats.backlog_size = stats.backlog_size.saturating_sub(1);
            });

            // Record metrics
            if delivered > 0 {
                self.delivered_counter
                    .add(delivered, &[KeyValue::new("topic", topic.to_string())]);
            }
            if dropped > 0 {
                let reason = if over_threshold {
                    "backpressure"
                } else {
                    "queue_full"
                };
                self.dropped_counter.add(
                    dropped,
                    &[
                        KeyValue::new("topic", topic.to_string()),
                        KeyValue::new("reason", reason),
                    ],
                );
            }

            // Update backlog gauge (decrement)
            self.backlog_gauge
                .add(-1, &[KeyValue::new("topic", topic.to_string())]);

            // Record publish latency
            let elapsed_ms = start_time.elapsed().as_secs_f64() * 1000.0;
            self.publish_latency
                .record(elapsed_ms, &[KeyValue::new("topic", topic.to_string())]);

            Span::current().record("delivered_count", delivered);
            Span::current().record("dropped_count", dropped);
            Span::current().record("latency_ms", elapsed_ms);

            Ok(delivered)
        } else {
            warn!("No subscriptions for topic: {}", topic);
            // Decrement backlog for the publish that had no subscribers
            self.update_stats(topic, |stats| {
                stats.backlog_size = stats.backlog_size.saturating_sub(1);
            });

            // Update backlog gauge
            self.backlog_gauge
                .add(-1, &[KeyValue::new("topic", topic.to_string())]);

            // Record publish latency even for no subscribers
            let elapsed_ms = start_time.elapsed().as_secs_f64() * 1000.0;
            self.publish_latency
                .record(elapsed_ms, &[KeyValue::new("topic", topic.to_string())]);

            Ok(0)
        }
    }

    /// Subscribe to topic
    #[tracing::instrument(skip(self, event_types), fields(topic = %topic, subscription_id, qos = ?qos))]
    pub async fn subscribe(
        &self,
        topic: String,
        event_types: Vec<String>,
        qos: QoSLevel,
    ) -> Result<(String, mpsc::Receiver<Event>)> {
        let subscription_id = format!("sub_{}_{}", topic, uuid::Uuid::new_v4());
        Span::current().record("subscription_id", &subscription_id);

        let cap = match qos {
            QoSLevel::QosRealtime => 64,
            QoSLevel::QosBatched => 1024,
            QoSLevel::QosBackground => 4096,
        };
        let (tx, rx) = mpsc::channel(cap);

        let subscription = Subscription {
            id: subscription_id.clone(),
            topic: topic.clone(),
            event_types,
            qos,
            sender: tx,
        };

        self.subscriptions
            .entry(topic.clone())
            .or_default()
            .push(subscription);

        self.update_stats(&topic, |stats| {
            stats.active_subscriptions += 1;
        });

        // Update active subscriptions gauge
        self.active_subscriptions_gauge
            .add(1, &[KeyValue::new("topic", topic.clone())]);

        info!(
            "Created subscription {} for topic {}",
            subscription_id, topic
        );
        Ok((subscription_id, rx))
    }

    /// Unsubscribe from topic
    #[tracing::instrument(skip(self), fields(subscription_id = %subscription_id))]
    pub async fn unsubscribe(&self, subscription_id: &str) -> Result<()> {
        for mut entry in self.subscriptions.iter_mut() {
            let topic = entry.key().clone();
            let before_count = entry.value().len();
            entry.value_mut().retain(|sub| sub.id != subscription_id);
            let after_count = entry.value().len();

            if before_count != after_count {
                self.update_stats(&topic, |stats| {
                    stats.active_subscriptions = stats.active_subscriptions.saturating_sub(1);
                });

                // Update active subscriptions gauge
                self.active_subscriptions_gauge
                    .add(-1, &[KeyValue::new("topic", topic)]);
            }
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
            .or_default()
            .value_mut()
            .apply(f);
    }

    // Update stats and return a value from the closure
    fn update_stats_and_get<F>(&self, topic: &str, f: F) -> Option<usize>
    where
        F: FnOnce(&mut EventBusStats) -> usize,
    {
        let mut entry = self.stats.entry(topic.to_string()).or_default();
        let val = f(entry.value_mut());
        Some(val)
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
