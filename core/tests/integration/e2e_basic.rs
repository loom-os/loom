//! Basic End-to-End Pipeline Tests
//!
//! Tests the minimal pipeline: Event → Agent → ActionBroker → Result → EventBus

use super::*;

/// Test 1: Minimal End-to-End Pipeline
///
/// Validates the complete pipeline flow from event publication to result propagation.
///
/// Flow:
/// 1. Publish "test_input" event to EventBus
/// 2. Agent receives event via subscription
/// 3. Agent behavior generates "echo" action
/// 4. Router makes routing decision (publishes routing_decision event)
/// 5. ActionBroker invokes echo capability
/// 6. Echo provider returns result
/// 7. Agent publishes action_result event back to EventBus
/// 8. Subscriber receives both routing_decision and action_result events
#[tokio::test]
async fn test_e2e_event_to_action_to_result() {
    // 1. Setup: Create EventBus, ActionBroker, Router, and AgentRuntime
    let event_bus = Arc::new(EventBus::new().await.unwrap());
    let action_broker = Arc::new(ActionBroker::new());
    let model_router = ModelRouter::new().await.unwrap();

    // Register mock echo provider
    action_broker.register_provider(Arc::new(MockEchoProvider));

    let agent_runtime = AgentRuntime::new(
        Arc::clone(&event_bus),
        Arc::clone(&action_broker),
        model_router.clone(),
    )
    .await
    .unwrap();

    // 2. Subscribe to result events on agent topic
    let (result_sub_id, mut result_rx) = event_bus
        .subscribe(
            "agent.test_agent".to_string(),
            vec!["action_result".to_string(), "routing_decision".to_string()],
            QoSLevel::QosBatched,
        )
        .await
        .unwrap();

    // 3. Create agent with mock behavior
    let received_events = Arc::new(Mutex::new(Vec::new()));
    let behavior = Box::new(MockEchoBehavior {
        received_events: Arc::clone(&received_events),
    });

    let config = AgentConfig {
        agent_id: "test_agent".to_string(),
        agent_type: "test".to_string(),
        subscribed_topics: vec!["test_topic".to_string()],
        capabilities: vec!["echo".to_string()],
        parameters: HashMap::new(),
    };

    agent_runtime.create_agent(config, behavior).await.unwrap();

    // Give agent time to start
    sleep(Duration::from_millis(100)).await;

    // 4. Publish test event
    let test_event = Event {
        id: "evt_test_001".to_string(),
        r#type: "test_input".to_string(),
        timestamp_ms: chrono::Utc::now().timestamp_millis(),
        source: "test".to_string(),
        metadata: HashMap::new(),
        payload: b"Hello Integration Test".to_vec(),
        confidence: 1.0,
        tags: vec!["test".to_string()],
        priority: 50,
    };

    let delivered = event_bus
        .publish("test_topic", test_event.clone())
        .await
        .unwrap();
    assert!(delivered > 0, "Event should be delivered to subscribers");

    // 5. Wait for and verify routing_decision event
    let mut routing_decision_received = false;
    let mut action_result_received = false;

    for _ in 0..10 {
        tokio::select! {
            Some(evt) = result_rx.recv() => {
                if evt.r#type == "routing_decision" {
                    routing_decision_received = true;
                    assert_eq!(evt.source, "agent.test_agent");
                    assert!(evt.metadata.contains_key("route"));
                    assert!(evt.metadata.contains_key("reason"));
                    println!("✓ Routing decision: {:?}", evt.metadata.get("route"));
                }

                if evt.r#type == "action_result" {
                    action_result_received = true;
                    assert_eq!(evt.source, "agent.test_agent");
                    assert_eq!(evt.metadata.get("action_type").map(|s| s.as_str()), Some("echo"));
                    assert_eq!(evt.metadata.get("status").map(|s| s.as_str()), Some("ok"));

                    let output = String::from_utf8_lossy(&evt.payload);
                    assert!(output.contains("ECHO:"), "Output should contain echo prefix");
                    println!("✓ Action result: {}", output);
                }

                if routing_decision_received && action_result_received {
                    break;
                }
            }
            _ = sleep(Duration::from_millis(500)) => {
                break;
            }
        }
    }

    assert!(
        routing_decision_received,
        "Should receive routing_decision event"
    );
    assert!(action_result_received, "Should receive action_result event");

    // 6. Verify agent received the event
    let events = received_events.lock().await;
    assert_eq!(events.len(), 1, "Agent should receive exactly one event");
    assert_eq!(events[0].id, "evt_test_001");

    // Cleanup
    event_bus.unsubscribe(&result_sub_id).await.unwrap();
    agent_runtime.delete_agent("test_agent").await.unwrap();

    println!("✓ E2E test passed: Event → Agent → ActionBroker → Result → EventBus");
}

/// Test 2: Event Type Filtering
///
/// Validates that subscribers only receive events matching their type filter.
///
/// Setup:
/// - Subscriber filters for "action_result" events only
///
/// Flow:
/// 1. Agent processes "test_input" event → Generates action → Publishes "action_result"
/// 2. Agent processes "other_type" event → No action → No "action_result"
#[tokio::test]
async fn test_e2e_event_type_filtering() {
    let event_bus = Arc::new(EventBus::new().await.unwrap());
    let action_broker = Arc::new(ActionBroker::new());
    let model_router = ModelRouter::new().await.unwrap();

    action_broker.register_provider(Arc::new(MockEchoProvider));

    let agent_runtime = AgentRuntime::new(
        Arc::clone(&event_bus),
        Arc::clone(&action_broker),
        model_router.clone(),
    )
    .await
    .unwrap();

    let received = Arc::new(Mutex::new(Vec::new()));
    let behavior = Box::new(MockEchoBehavior {
        received_events: Arc::clone(&received),
    });

    let config = AgentConfig {
        agent_id: "filter_agent".to_string(),
        agent_type: "test".to_string(),
        subscribed_topics: vec!["filter_topic".to_string()],
        capabilities: vec!["echo".to_string()],
        parameters: HashMap::new(),
    };

    agent_runtime.create_agent(config, behavior).await.unwrap();

    sleep(Duration::from_millis(100)).await;

    // Subscribe with event type filter
    let (_sub, mut result_rx) = event_bus
        .subscribe(
            "agent.filter_agent".to_string(),
            vec!["action_result".to_string()], // Only action_result, not routing_decision
            QoSLevel::QosBatched,
        )
        .await
        .unwrap();

    // Publish test_input event (should generate action)
    let event_1 = Event {
        id: "evt_filter_1".to_string(),
        r#type: "test_input".to_string(),
        timestamp_ms: chrono::Utc::now().timestamp_millis(),
        source: "test".to_string(),
        metadata: HashMap::new(),
        payload: b"Filtered input".to_vec(),
        confidence: 1.0,
        tags: vec![],
        priority: 50,
    };

    event_bus.publish("filter_topic", event_1).await.unwrap();

    // Should receive action_result
    let result = tokio::time::timeout(Duration::from_secs(2), result_rx.recv())
        .await
        .expect("Should receive action_result")
        .expect("Channel should not be closed");

    assert_eq!(result.r#type, "action_result");

    // Publish other_type event (should NOT generate action)
    let event_2 = Event {
        id: "evt_filter_2".to_string(),
        r#type: "other_type".to_string(),
        timestamp_ms: chrono::Utc::now().timestamp_millis(),
        source: "test".to_string(),
        metadata: HashMap::new(),
        payload: b"Other input".to_vec(),
        confidence: 1.0,
        tags: vec![],
        priority: 50,
    };

    event_bus.publish("filter_topic", event_2).await.unwrap();

    // Should NOT receive any more action_result events
    let no_result = tokio::time::timeout(Duration::from_millis(500), result_rx.recv()).await;

    assert!(
        no_result.is_err(),
        "Should not receive action for filtered event type"
    );

    // Verify agent received both events
    let events = received.lock().await;
    assert_eq!(events.len(), 2);

    println!("✓ Event filtering test passed: Subscribers only receive matching event types");
}
