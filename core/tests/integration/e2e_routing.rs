//! Routing Decision Tests
//!
//! Tests routing decision logic and observability.

use super::*;

/// Test: Routing Decision Observation
///
/// Validates that routing decisions are observable and influenced by privacy policies.
///
/// Setup:
/// - Agent configured with `routing.privacy = "local-only"`
/// - Event metadata contains `privacy = "local-only"`
///
/// Flow:
/// 1. Agent receives event with local-only privacy
/// 2. Router evaluates privacy policy
/// 3. Router publishes routing_decision event
/// 4. Decision specifies "Local" route with reason
#[tokio::test]
async fn test_routing_decision_with_privacy_policy() {
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

    // Create agent with local-only privacy setting
    let mut params = HashMap::new();
    params.insert("routing.privacy".to_string(), "local-only".to_string());

    let config = AgentConfig {
        agent_id: "routing_agent".to_string(),
        agent_type: "test".to_string(),
        subscribed_topics: vec!["routing_topic".to_string()],
        capabilities: vec!["echo".to_string()],
        parameters: params,
    };

    let received = Arc::new(Mutex::new(Vec::new()));
    let behavior = Box::new(MockEchoBehavior {
        received_events: Arc::clone(&received),
    });

    agent_runtime.create_agent(config, behavior).await.unwrap();

    sleep(Duration::from_millis(100)).await;

    // Subscribe to routing_decision events
    let (_sub, mut decision_rx) = event_bus
        .subscribe(
            "agent.routing_agent".to_string(),
            vec!["routing_decision".to_string()],
            QoSLevel::QosBatched,
        )
        .await
        .unwrap();

    // Publish event with local-only metadata
    let mut metadata = HashMap::new();
    metadata.insert("privacy".to_string(), "local-only".to_string());

    let event = Event {
        id: "evt_routing".to_string(),
        r#type: "test_input".to_string(),
        timestamp_ms: chrono::Utc::now().timestamp_millis(),
        source: "test".to_string(),
        metadata,
        payload: b"Test routing".to_vec(),
        confidence: 1.0,
        tags: vec![],
        priority: 50,
    };

    event_bus.publish("routing_topic", event).await.unwrap();

    // Verify routing decision
    let decision = tokio::time::timeout(Duration::from_secs(2), decision_rx.recv())
        .await
        .expect("Should receive routing decision")
        .expect("Channel should not be closed");

    assert_eq!(decision.r#type, "routing_decision");
    assert_eq!(decision.source, "agent.routing_agent");
    assert!(decision.metadata.contains_key("route"));
    assert!(decision.metadata.contains_key("reason"));
    assert!(decision.metadata.contains_key("confidence"));

    let route = decision.metadata.get("route").unwrap();
    assert_eq!(
        route, "Local",
        "Should route to Local for local-only privacy"
    );

    println!("✓ Routing decision observation test passed");
}
