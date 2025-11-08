//! Multi-Agent Interaction Tests
//!
//! Tests multiple agents operating independently with different topics and behaviors.

use super::*;

/// Test: Multiple Agents with Different Topics
///
/// Validates correct event routing to multiple agents subscribed to different topics.
///
/// Setup:
/// - Agent 1 subscribes to "topic_a"
/// - Agent 2 subscribes to "topic_b"
///
/// Flow:
/// 1. Publish event to "topic_a" → Only Agent 1 receives and processes
/// 2. Publish event to "topic_b" → Only Agent 2 receives and processes
#[tokio::test]
async fn test_multiple_agents_different_topics() {
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

    // Create two agents subscribing to different topics
    let received_1 = Arc::new(Mutex::new(Vec::new()));
    let received_2 = Arc::new(Mutex::new(Vec::new()));

    let behavior_1 = Box::new(MockEchoBehavior {
        received_events: Arc::clone(&received_1),
    });
    let behavior_2 = Box::new(MockEchoBehavior {
        received_events: Arc::clone(&received_2),
    });

    let config_1 = AgentConfig {
        agent_id: "agent_1".to_string(),
        agent_type: "test".to_string(),
        subscribed_topics: vec!["topic_a".to_string()],
        capabilities: vec!["echo".to_string()],
        parameters: HashMap::new(),
    };

    let config_2 = AgentConfig {
        agent_id: "agent_2".to_string(),
        agent_type: "test".to_string(),
        subscribed_topics: vec!["topic_b".to_string()],
        capabilities: vec!["echo".to_string()],
        parameters: HashMap::new(),
    };

    agent_runtime
        .create_agent(config_1, behavior_1)
        .await
        .unwrap();
    agent_runtime
        .create_agent(config_2, behavior_2)
        .await
        .unwrap();

    sleep(Duration::from_millis(100)).await;

    // Subscribe to both agent result channels
    let (_sub_1, mut rx_1) = event_bus
        .subscribe(
            "agent.agent_1".to_string(),
            vec!["action_result".to_string()],
            QoSLevel::QosBatched,
        )
        .await
        .unwrap();

    let (_sub_2, mut rx_2) = event_bus
        .subscribe(
            "agent.agent_2".to_string(),
            vec!["action_result".to_string()],
            QoSLevel::QosBatched,
        )
        .await
        .unwrap();

    // Publish to topic_a (should only trigger agent_1)
    let event_a = Event {
        id: "evt_a".to_string(),
        r#type: "test_input".to_string(),
        timestamp_ms: chrono::Utc::now().timestamp_millis(),
        source: "test".to_string(),
        metadata: HashMap::new(),
        payload: b"Message A".to_vec(),
        confidence: 1.0,
        tags: vec![],
        priority: 50,
    };

    event_bus.publish("topic_a", event_a).await.unwrap();

    // Wait for agent_1 result
    let result_1 = tokio::time::timeout(Duration::from_secs(2), rx_1.recv())
        .await
        .expect("Should receive result from agent_1")
        .expect("Channel should not be closed");

    assert_eq!(result_1.source, "agent.agent_1");

    // Publish to topic_b (should only trigger agent_2)
    let event_b = Event {
        id: "evt_b".to_string(),
        r#type: "test_input".to_string(),
        timestamp_ms: chrono::Utc::now().timestamp_millis(),
        source: "test".to_string(),
        metadata: HashMap::new(),
        payload: b"Message B".to_vec(),
        confidence: 1.0,
        tags: vec![],
        priority: 50,
    };

    event_bus.publish("topic_b", event_b).await.unwrap();

    // Wait for agent_2 result
    let result_2 = tokio::time::timeout(Duration::from_secs(2), rx_2.recv())
        .await
        .expect("Should receive result from agent_2")
        .expect("Channel should not be closed");

    assert_eq!(result_2.source, "agent.agent_2");

    // Verify correct event distribution
    let events_1 = received_1.lock().await;
    let events_2 = received_2.lock().await;

    assert_eq!(events_1.len(), 1);
    assert_eq!(events_1[0].id, "evt_a");

    assert_eq!(events_2.len(), 1);
    assert_eq!(events_2[0].id, "evt_b");

    println!("✓ Multi-agent test passed: Correct event routing to different agents");
}
