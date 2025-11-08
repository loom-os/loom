//! Shared utilities and helpers for pressure tests

use loom_core::proto::Event;

/// Helper to create a test event with minimal overhead
pub fn make_event(id: u64, event_type: &str) -> Event {
    Event {
        id: format!("evt_{}", id),
        r#type: event_type.to_string(),
        timestamp_ms: 0,
        source: "pressure_test".to_string(),
        metadata: Default::default(),
        payload: vec![],
        confidence: 1.0,
        tags: vec![],
        priority: 0,
    }
}

// Test modules
pub mod backpressure;
pub mod filtering;
pub mod latency;
pub mod qos_behavior;
pub mod stats;
pub mod throughput;
