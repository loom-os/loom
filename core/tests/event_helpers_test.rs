//! Event Helper Functions Tests
//!
//! Tests the fluent Event extension trait for envelope metadata access.

use loom_core::{Event, EventExt};
use std::collections::HashMap;

#[test]
fn test_with_thread_helper() {
    let event = Event {
        id: "evt-1".to_string(),
        r#type: "test".to_string(),
        timestamp_ms: 0,
        source: "test".to_string(),
        metadata: HashMap::new(),
        payload: vec![],
        confidence: 1.0,
        tags: vec![],
        priority: 50,
    };

    let event = event.with_thread("thread-123".to_string());
    
    assert_eq!(event.thread_id(), Some("thread-123"));
    assert_eq!(event.metadata.get("thread_id"), Some(&"thread-123".to_string()));
}

#[test]
fn test_with_correlation_helper() {
    let event = Event {
        id: "evt-1".to_string(),
        r#type: "test".to_string(),
        timestamp_ms: 0,
        source: "test".to_string(),
        metadata: HashMap::new(),
        payload: vec![],
        confidence: 1.0,
        tags: vec![],
        priority: 50,
    };

    let event = event.with_correlation("corr-456".to_string());
    
    assert_eq!(event.correlation_id(), Some("corr-456"));
    assert_eq!(event.metadata.get("correlation_id"), Some(&"corr-456".to_string()));
}

#[test]
fn test_with_reply_to_helper() {
    let event = Event {
        id: "evt-1".to_string(),
        r#type: "test".to_string(),
        timestamp_ms: 0,
        source: "test".to_string(),
        metadata: HashMap::new(),
        payload: vec![],
        confidence: 1.0,
        tags: vec![],
        priority: 50,
    };

    let event = event.with_reply_to("thread.task-1.reply".to_string());
    
    assert_eq!(event.reply_to(), Some("thread.task-1.reply"));
    assert_eq!(event.metadata.get("reply_to"), Some(&"thread.task-1.reply".to_string()));
}

#[test]
fn test_with_sender_helper() {
    let event = Event {
        id: "evt-1".to_string(),
        r#type: "test".to_string(),
        timestamp_ms: 0,
        source: "test".to_string(),
        metadata: HashMap::new(),
        payload: vec![],
        confidence: 1.0,
        tags: vec![],
        priority: 50,
    };

    let event = event.with_sender("agent.coordinator".to_string());
    
    assert_eq!(event.sender(), Some("agent.coordinator"));
    assert_eq!(event.metadata.get("sender"), Some(&"agent.coordinator".to_string()));
}

#[test]
fn test_fluent_chaining() {
    let event = Event {
        id: "evt-chain".to_string(),
        r#type: "request".to_string(),
        timestamp_ms: chrono::Utc::now().timestamp_millis(),
        source: "coordinator".to_string(),
        metadata: HashMap::new(),
        payload: b"test payload".to_vec(),
        confidence: 1.0,
        tags: vec!["test".to_string()],
        priority: 50,
    }
    .with_thread("task-99".to_string())
    .with_correlation("task-99".to_string())
    .with_sender("agent.coordinator".to_string())
    .with_reply_to("thread.task-99.reply".to_string());

    // Verify all fields were set
    assert_eq!(event.thread_id(), Some("task-99"));
    assert_eq!(event.correlation_id(), Some("task-99"));
    assert_eq!(event.sender(), Some("agent.coordinator"));
    assert_eq!(event.reply_to(), Some("thread.task-99.reply"));
    
    // Verify original fields preserved
    assert_eq!(event.id, "evt-chain");
    assert_eq!(event.r#type, "request");
    assert_eq!(event.payload, b"test payload");
}

#[test]
fn test_read_helpers_missing_fields() {
    let event = Event {
        id: "evt-empty".to_string(),
        r#type: "test".to_string(),
        timestamp_ms: 0,
        source: "test".to_string(),
        metadata: HashMap::new(), // Empty metadata
        payload: vec![],
        confidence: 1.0,
        tags: vec![],
        priority: 50,
    };

    assert_eq!(event.thread_id(), None);
    assert_eq!(event.correlation_id(), None);
    assert_eq!(event.sender(), None);
    assert_eq!(event.reply_to(), None);
}

#[test]
fn test_read_helpers_with_existing_metadata() {
    let mut metadata = HashMap::new();
    metadata.insert("thread_id".to_string(), "existing-thread".to_string());
    metadata.insert("custom_field".to_string(), "custom_value".to_string());

    let event = Event {
        id: "evt-existing".to_string(),
        r#type: "test".to_string(),
        timestamp_ms: 0,
        source: "test".to_string(),
        metadata,
        payload: vec![],
        confidence: 1.0,
        tags: vec![],
        priority: 50,
    };

    assert_eq!(event.thread_id(), Some("existing-thread"));
    assert_eq!(event.metadata.get("custom_field"), Some(&"custom_value".to_string()));
}

#[test]
fn test_with_helpers_override_existing() {
    let mut metadata = HashMap::new();
    metadata.insert("thread_id".to_string(), "old-thread".to_string());

    let event = Event {
        id: "evt-override".to_string(),
        r#type: "test".to_string(),
        timestamp_ms: 0,
        source: "test".to_string(),
        metadata,
        payload: vec![],
        confidence: 1.0,
        tags: vec![],
        priority: 50,
    };

    let event = event.with_thread("new-thread".to_string());
    
    assert_eq!(event.thread_id(), Some("new-thread"));
    assert_eq!(event.metadata.get("thread_id"), Some(&"new-thread".to_string()));
}

#[test]
fn test_helpers_vs_envelope() {
    use loom_core::Envelope;

    // Create event using helpers
    let event_helpers = Event {
        id: "evt-compare".to_string(),
        r#type: "test".to_string(),
        timestamp_ms: 0,
        source: "test".to_string(),
        metadata: HashMap::new(),
        payload: vec![],
        confidence: 1.0,
        tags: vec![],
        priority: 50,
    }
    .with_thread("task-1".to_string())
    .with_sender("agent.worker".to_string())
    .with_correlation("task-1".to_string())
    .with_reply_to("thread.task-1.reply".to_string());

    // Create event using Envelope (old way)
    let mut event_envelope = Event {
        id: "evt-compare".to_string(),
        r#type: "test".to_string(),
        timestamp_ms: 0,
        source: "test".to_string(),
        metadata: HashMap::new(),
        payload: vec![],
        confidence: 1.0,
        tags: vec![],
        priority: 50,
    };
    let env = Envelope::new("task-1", "agent.worker");
    env.attach_to_event(&mut event_envelope);

    // Both should have the same envelope fields
    assert_eq!(event_helpers.thread_id(), event_envelope.thread_id());
    assert_eq!(event_helpers.sender(), event_envelope.sender());
    assert_eq!(event_helpers.correlation_id(), event_envelope.correlation_id());
    // Note: Envelope sets additional fields like ttl, hop, ts
}

#[test]
fn test_real_world_usage_pattern() {
    // Simulate creating a request event for multi-agent collaboration
    let request = Event {
        id: format!("evt_{}", chrono::Utc::now().timestamp_millis()),
        r#type: "collab.request".to_string(),
        timestamp_ms: chrono::Utc::now().timestamp_millis(),
        source: "coordinator".to_string(),
        metadata: HashMap::new(),
        payload: b"analyze this data".to_vec(),
        confidence: 1.0,
        tags: vec!["collaboration".to_string()],
        priority: 70,
    }
    .with_thread("analysis-task-1".to_string())
    .with_correlation("analysis-task-1".to_string())
    .with_sender("agent.coordinator".to_string())
    .with_reply_to("thread.analysis-task-1.reply".to_string());

    // Verify it's ready for collaboration
    assert!(request.thread_id().is_some());
    assert!(request.sender().is_some());
    assert!(request.reply_to().is_some());
    
    println!("✓ Real-world usage pattern test passed");
}
