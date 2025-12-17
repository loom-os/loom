//! Integration test for EventBus to WebSocket event streaming
//!
//! Tests the complete flow:
//! 1. EventBus publishes events to configured topics
//! 2. EventBusBridge subscribes and forwards to WebSocket
//! 3. WebSocket clients receive event_stream messages

use loom_core::messaging::event_bus::EventBus;
use loom_core::proto::Event;
use loom_dashboard::{
    config::{DashboardConfig, ObservabilityConfig},
    ws::{ConnectionManager, EventBusBridge, WsMessage},
};
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};

#[tokio::test]
async fn test_event_bridge_forwards_to_websocket() {
    // Setup
    let event_bus = Arc::new(EventBus::new().await.unwrap());
    let connection_manager = Arc::new(ConnectionManager::new());
    let mut config = ObservabilityConfig::default();
    config.topics = vec!["test.topic".to_string()];

    // Create and start bridge
    let bridge = Arc::new(EventBusBridge::new(
        event_bus.clone(),
        connection_manager.clone(),
        config,
    ));
    bridge.clone().start().await.unwrap();

    // Give bridge time to subscribe
    sleep(Duration::from_millis(100)).await;

    // Add a WebSocket connection
    let (tx, mut rx) = mpsc::unbounded_channel();
    connection_manager.add("test-conn".to_string(), tx);

    // Publish an event
    let mut metadata = std::collections::HashMap::new();
    metadata.insert("agent_id".to_string(), "test-agent".to_string());
    metadata.insert("thread_id".to_string(), "thread-123".to_string());

    let event = Event {
        id: uuid::Uuid::new_v4().to_string(),
        r#type: "test.event".to_string(),
        source: "test.topic".to_string(),
        timestamp_ms: chrono::Utc::now().timestamp_millis(),
        metadata,
        payload: b"test payload".to_vec(),
        ..Default::default()
    };

    event_bus
        .publish("test.topic", event.clone())
        .await
        .unwrap();

    // Wait for message
    let message = tokio::time::timeout(Duration::from_secs(2), rx.recv())
        .await
        .expect("Timeout waiting for message")
        .expect("Channel closed");

    // Verify message
    match message {
        WsMessage::EventStream {
            event_id,
            topic,
            sender,
            thread_id,
            ..
        } => {
            assert_eq!(event_id, event.id);
            assert_eq!(topic, "test.topic");
            assert_eq!(sender, Some("test-agent".to_string()));
            assert_eq!(thread_id, Some("thread-123".to_string()));
        }
        _ => panic!("Expected EventStream message, got {:?}", message),
    }
}

#[tokio::test]
async fn test_bridge_subscribes_to_wildcard_topics() {
    // Setup
    let event_bus = Arc::new(EventBus::new().await.unwrap());
    let connection_manager = Arc::new(ConnectionManager::new());
    let mut config = ObservabilityConfig::default();
    config.topics = vec!["agent.*".to_string()];

    // Create and start bridge
    let bridge = Arc::new(EventBusBridge::new(
        event_bus.clone(),
        connection_manager.clone(),
        config,
    ));
    bridge.clone().start().await.unwrap();

    // Give bridge time to subscribe
    sleep(Duration::from_millis(100)).await;

    // Add a WebSocket connection
    let (tx, mut rx) = mpsc::unbounded_channel();
    connection_manager.add("test-conn".to_string(), tx);

    // Publish events to different agent topics
    for agent_id in ["agent.planner", "agent.researcher", "agent.writer"] {
        let event = Event {
            id: uuid::Uuid::new_v4().to_string(),
            r#type: "agent.activity".to_string(),
            source: agent_id.to_string(),
            timestamp_ms: chrono::Utc::now().timestamp_millis(),
            metadata: std::collections::HashMap::new(),
            payload: b"activity".to_vec(),
            ..Default::default()
        };

        event_bus.publish(agent_id, event).await.unwrap();
    }

    // Should receive all three events
    for _ in 0..3 {
        let message = tokio::time::timeout(Duration::from_secs(2), rx.recv())
            .await
            .expect("Timeout waiting for message")
            .expect("Channel closed");

        assert!(matches!(message, WsMessage::EventStream { .. }));
    }
}

#[tokio::test]
async fn test_bridge_respects_payload_config() {
    // Setup with payload disabled
    let event_bus = Arc::new(EventBus::new().await.unwrap());
    let connection_manager = Arc::new(ConnectionManager::new());
    let mut config = ObservabilityConfig::default();
    config.topics = vec!["test.topic".to_string()];
    config.include_payload = false;

    // Create and start bridge
    let bridge = Arc::new(EventBusBridge::new(
        event_bus.clone(),
        connection_manager.clone(),
        config,
    ));
    bridge.clone().start().await.unwrap();

    // Give bridge time to subscribe
    sleep(Duration::from_millis(100)).await;

    // Add a WebSocket connection
    let (tx, mut rx) = mpsc::unbounded_channel();
    connection_manager.add("test-conn".to_string(), tx);

    // Publish an event with payload
    let event = Event {
        id: uuid::Uuid::new_v4().to_string(),
        r#type: "test.event".to_string(),
        source: "test.topic".to_string(),
        timestamp_ms: chrono::Utc::now().timestamp_millis(),
        metadata: std::collections::HashMap::new(),
        payload: b"secret data that should not be shown".to_vec(),
        ..Default::default()
    };

    event_bus.publish("test.topic", event).await.unwrap();

    // Wait for message
    let message = tokio::time::timeout(Duration::from_secs(2), rx.recv())
        .await
        .expect("Timeout waiting for message")
        .expect("Channel closed");

    // Verify payload is not included
    match message {
        WsMessage::EventStream {
            payload_preview, ..
        } => {
            // Should only show byte count
            assert!(payload_preview.contains("bytes"));
            assert!(!payload_preview.contains("secret"));
        }
        _ => panic!("Expected EventStream message"),
    }
}

#[tokio::test]
async fn test_bridge_handles_multiple_connections() {
    // Setup
    let event_bus = Arc::new(EventBus::new().await.unwrap());
    let connection_manager = Arc::new(ConnectionManager::new());
    let mut config = ObservabilityConfig::default();
    config.topics = vec!["test.topic".to_string()];

    // Create and start bridge
    let bridge = Arc::new(EventBusBridge::new(
        event_bus.clone(),
        connection_manager.clone(),
        config,
    ));
    bridge.clone().start().await.unwrap();

    // Give bridge time to subscribe
    sleep(Duration::from_millis(100)).await;

    // Add multiple WebSocket connections
    let (tx1, mut rx1) = mpsc::unbounded_channel();
    let (tx2, mut rx2) = mpsc::unbounded_channel();
    let (tx3, mut rx3) = mpsc::unbounded_channel();

    connection_manager.add("conn1".to_string(), tx1);
    connection_manager.add("conn2".to_string(), tx2);
    connection_manager.add("conn3".to_string(), tx3);

    // Publish an event
    let event = Event {
        id: uuid::Uuid::new_v4().to_string(),
        r#type: "test.event".to_string(),
        source: "test.topic".to_string(),
        timestamp_ms: chrono::Utc::now().timestamp_millis(),
        metadata: std::collections::HashMap::new(),
        payload: b"broadcast test".to_vec(),
        ..Default::default()
    };

    event_bus.publish("test.topic", event).await.unwrap();

    // All connections should receive the event
    for rx in [&mut rx1, &mut rx2, &mut rx3] {
        let message = tokio::time::timeout(Duration::from_secs(2), rx.recv())
            .await
            .expect("Timeout waiting for message")
            .expect("Channel closed");

        assert!(matches!(message, WsMessage::EventStream { .. }));
    }
}

#[tokio::test]
async fn test_default_observability_config() {
    let config = ObservabilityConfig::default();

    // Should include common topics
    assert!(config.topics.contains(&"dashboard".to_string()));
    assert!(config.topics.contains(&"agent.*".to_string()));
    assert!(config.topics.contains(&"stream.*".to_string()));

    // Should have sensible defaults
    assert!(config.include_payload);
    assert_eq!(config.max_payload_preview, 200);
}

#[tokio::test]
async fn test_dashboard_config_with_observability() {
    let config = DashboardConfig::default();

    // Should have observability config
    assert!(!config.observability.topics.is_empty());
}
