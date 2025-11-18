//! Messaging layer: Event Bus, Envelope coordination, and Event extensions.
//!
//! This module provides the core messaging infrastructure for Loom:
//! - `EventBus`: Topic-based pub/sub with QoS and backpressure
//! - `Envelope`: Coordination metadata for thread/correlation/routing/tracing
//! - `EventExt`: Fluent helpers for reading/writing envelope fields on Events

pub mod envelope;
pub mod event_bus;
pub mod event_ext;

// Re-export key types for ergonomic access
pub use envelope::{agent_reply_topic, Envelope, ThreadTopicKind};
pub use event_bus::{EventBus, EventBusStats, EventHandler};
pub use event_ext::EventExt;
