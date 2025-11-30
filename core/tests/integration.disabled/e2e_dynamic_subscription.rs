//! Dynamic Agent Subscription Tests
//!
//! Tests agents dynamically joining and leaving topics at runtime,
//! enabling multi-agent collaboration patterns like expert consultation
//! and task delegation.

use super::*;

/// Test: Agent Joins Thread Mid-Conversation
///
/// Validates that agents can dynamically subscribe to thread topics
/// and start receiving events immediately.
///
/// Setup:
/// - Agent 1 subscribes to thread broadcast at creation
/// - Agent 2 starts without thread subscription
///
/// Flow:
/// 1. Publish event to thread → Only Agent 1 receives
/// 2. Agent 2 subscribes to thread dynamically
/// 3. Publish another event → Both agents receive
#[tokio::test]
async fn test_agent_joins_thread_mid_conversation() {
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

    let thread_id = "thread-collab-1";
    let thread_topic = format!("thread.{}.broadcast", thread_id);

    // Create two agents: agent_1 starts subscribed, agent_2 does not
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
        agent_type: "participant".to_string(),
        subscribed_topics: vec![thread_topic.clone()],
        capabilities: vec![],
        parameters: HashMap::new(),
    };

    let config_2 = AgentConfig {
        agent_id: "agent_2".to_string(),
        agent_type: "expert".to_string(),
        subscribed_topics: vec![], // No initial subscription
        capabilities: vec![],
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

    // Phase 1: Only agent_1 is subscribed
    let event_1 = Event {
        id: "evt_before_join".to_string(),
        r#type: "collab.message".to_string(),
        timestamp_ms: chrono::Utc::now().timestamp_millis(),
        source: "coordinator".to_string(),
        metadata: {
            let mut m = HashMap::new();
            m.insert("thread_id".to_string(), thread_id.to_string());
            m
        },
        payload: b"Initial message".to_vec(),
        confidence: 1.0,
        tags: vec![],
        priority: 50,
    };

    event_bus
        .publish(&thread_topic, event_1.clone())
        .await
        .unwrap();

    sleep(Duration::from_millis(200)).await;

    // Verify: Only agent_1 received the event
    {
        let events_1 = received_1.lock().await;
        assert_eq!(events_1.len(), 1);
        assert_eq!(events_1[0].id, "evt_before_join");

        let events_2 = received_2.lock().await;
        assert_eq!(events_2.len(), 0, "Agent 2 should not receive event yet");
    }

    // Phase 2: Agent 2 joins the thread dynamically
    agent_runtime
        .subscribe_agent("agent_2", thread_topic.clone())
        .await
        .unwrap();

    sleep(Duration::from_millis(100)).await;

    // Verify subscription was tracked (agent has private reply topic + new thread topic)
    let subs = agent_runtime.get_agent_subscriptions("agent_2").unwrap();
    assert_eq!(subs.len(), 2); // private reply + thread_topic
    assert!(subs.contains(&thread_topic));
    assert!(subs.contains(&"agent.agent_2.replies".to_string()));

    // Phase 3: Both agents should receive new events
    let event_2 = Event {
        id: "evt_after_join".to_string(),
        r#type: "collab.message".to_string(),
        timestamp_ms: chrono::Utc::now().timestamp_millis(),
        source: "coordinator".to_string(),
        metadata: {
            let mut m = HashMap::new();
            m.insert("thread_id".to_string(), thread_id.to_string());
            m
        },
        payload: b"Message after join".to_vec(),
        confidence: 1.0,
        tags: vec![],
        priority: 50,
    };

    event_bus
        .publish(&thread_topic, event_2.clone())
        .await
        .unwrap();

    sleep(Duration::from_millis(200)).await;

    // Verify: Both agents received the second event
    {
        let events_1 = received_1.lock().await;
        assert_eq!(events_1.len(), 2);
        assert_eq!(events_1[1].id, "evt_after_join");

        let events_2 = received_2.lock().await;
        assert_eq!(events_2.len(), 1);
        assert_eq!(events_2[0].id, "evt_after_join");
    }

    println!("✓ Dynamic subscription test passed: Agent joined thread mid-conversation");
}

/// Test: Agent Leaves Thread
///
/// Validates that agents can unsubscribe from topics and stop receiving events.
#[tokio::test]
async fn test_agent_leaves_thread() {
    let event_bus = Arc::new(EventBus::new().await.unwrap());
    let action_broker = Arc::new(ActionBroker::new());
    let model_router = ModelRouter::new().await.unwrap();

    let agent_runtime = AgentRuntime::new(
        Arc::clone(&event_bus),
        Arc::clone(&action_broker),
        model_router,
    )
    .await
    .unwrap();

    let thread_topic = "thread.task-99.broadcast";

    let received = Arc::new(Mutex::new(Vec::new()));
    let behavior = Box::new(MockEchoBehavior {
        received_events: Arc::clone(&received),
    });

    let config = AgentConfig {
        agent_id: "agent_leaving".to_string(),
        agent_type: "worker".to_string(),
        subscribed_topics: vec![thread_topic.to_string()],
        capabilities: vec![],
        parameters: HashMap::new(),
    };

    agent_runtime.create_agent(config, behavior).await.unwrap();
    sleep(Duration::from_millis(100)).await;

    // Phase 1: Agent receives events
    let event_1 = Event {
        id: "evt_before_leave".to_string(),
        r#type: "task".to_string(),
        timestamp_ms: chrono::Utc::now().timestamp_millis(),
        source: "coordinator".to_string(),
        metadata: HashMap::new(),
        payload: b"Task 1".to_vec(),
        confidence: 1.0,
        tags: vec![],
        priority: 50,
    };

    event_bus.publish(thread_topic, event_1).await.unwrap();
    sleep(Duration::from_millis(200)).await;

    {
        let events = received.lock().await;
        assert_eq!(events.len(), 1);
    }

    // Phase 2: Agent unsubscribes
    agent_runtime
        .unsubscribe_agent("agent_leaving", thread_topic)
        .await
        .unwrap();

    sleep(Duration::from_millis(100)).await;

    // Verify subscription was removed (only private reply topic remains)
    let subs = agent_runtime
        .get_agent_subscriptions("agent_leaving")
        .unwrap();
    assert_eq!(subs.len(), 1); // Only private reply topic remains
    assert!(subs.contains(&"agent.agent_leaving.replies".to_string()));

    // Phase 3: Agent does not receive new events
    let event_2 = Event {
        id: "evt_after_leave".to_string(),
        r#type: "task".to_string(),
        timestamp_ms: chrono::Utc::now().timestamp_millis(),
        source: "coordinator".to_string(),
        metadata: HashMap::new(),
        payload: b"Task 2".to_vec(),
        confidence: 1.0,
        tags: vec![],
        priority: 50,
    };

    event_bus.publish(thread_topic, event_2).await.unwrap();
    sleep(Duration::from_millis(200)).await;

    {
        let events = received.lock().await;
        assert_eq!(
            events.len(),
            1,
            "Agent should not receive events after unsubscribe"
        );
    }

    println!("✓ Unsubscribe test passed: Agent left thread successfully");
}

/// Test: Multiple Dynamic Subscriptions
///
/// Validates that agents can manage multiple subscriptions simultaneously.
#[tokio::test]
async fn test_multiple_dynamic_subscriptions() {
    let event_bus = Arc::new(EventBus::new().await.unwrap());
    let action_broker = Arc::new(ActionBroker::new());
    let model_router = ModelRouter::new().await.unwrap();

    let agent_runtime = AgentRuntime::new(
        Arc::clone(&event_bus),
        Arc::clone(&action_broker),
        model_router,
    )
    .await
    .unwrap();

    let received = Arc::new(Mutex::new(Vec::new()));
    let behavior = Box::new(MockEchoBehavior {
        received_events: Arc::clone(&received),
    });

    let config = AgentConfig {
        agent_id: "agent_multi".to_string(),
        agent_type: "multi_subscriber".to_string(),
        subscribed_topics: vec!["topic_a".to_string()],
        capabilities: vec![],
        parameters: HashMap::new(),
    };

    agent_runtime.create_agent(config, behavior).await.unwrap();
    sleep(Duration::from_millis(100)).await;

    // Subscribe to additional topics
    agent_runtime
        .subscribe_agent("agent_multi", "topic_b".to_string())
        .await
        .unwrap();
    agent_runtime
        .subscribe_agent("agent_multi", "topic_c".to_string())
        .await
        .unwrap();

    sleep(Duration::from_millis(100)).await;

    // Verify all subscriptions (3 topics + 1 private reply topic)
    let subs = agent_runtime
        .get_agent_subscriptions("agent_multi")
        .unwrap();
    assert_eq!(subs.len(), 4); // topic_a, topic_b, topic_c + private reply
    assert!(subs.contains(&"topic_a".to_string()));
    assert!(subs.contains(&"topic_b".to_string()));
    assert!(subs.contains(&"topic_c".to_string()));
    assert!(subs.contains(&"agent.agent_multi.replies".to_string()));

    // Publish to each topic
    for (idx, topic) in ["topic_a", "topic_b", "topic_c"].iter().enumerate() {
        let event = Event {
            id: format!("evt_{}", idx),
            r#type: "test".to_string(),
            timestamp_ms: chrono::Utc::now().timestamp_millis(),
            source: "test".to_string(),
            metadata: HashMap::new(),
            payload: vec![idx as u8],
            confidence: 1.0,
            tags: vec![],
            priority: 50,
        };
        event_bus.publish(topic, event).await.unwrap();
    }

    sleep(Duration::from_millis(300)).await;

    // Verify agent received all events
    {
        let events = received.lock().await;
        assert_eq!(events.len(), 3);
    }

    // Unsubscribe from one topic
    agent_runtime
        .unsubscribe_agent("agent_multi", "topic_b")
        .await
        .unwrap();

    let subs = agent_runtime
        .get_agent_subscriptions("agent_multi")
        .unwrap();
    assert_eq!(subs.len(), 3); // topic_a, topic_c + private reply
    assert!(!subs.contains(&"topic_b".to_string()));

    println!("✓ Multiple subscriptions test passed");
}

/// Test: Error Handling - Subscribe Twice to Same Topic
#[tokio::test]
async fn test_subscribe_twice_error() {
    let event_bus = Arc::new(EventBus::new().await.unwrap());
    let action_broker = Arc::new(ActionBroker::new());
    let model_router = ModelRouter::new().await.unwrap();

    let agent_runtime = AgentRuntime::new(
        Arc::clone(&event_bus),
        Arc::clone(&action_broker),
        model_router,
    )
    .await
    .unwrap();

    let received = Arc::new(Mutex::new(Vec::new()));
    let behavior = Box::new(MockEchoBehavior {
        received_events: Arc::clone(&received),
    });

    let config = AgentConfig {
        agent_id: "agent_dup".to_string(),
        agent_type: "test".to_string(),
        subscribed_topics: vec!["topic_dup".to_string()],
        capabilities: vec![],
        parameters: HashMap::new(),
    };

    agent_runtime.create_agent(config, behavior).await.unwrap();
    sleep(Duration::from_millis(100)).await;

    // Try to subscribe to same topic again
    let result = agent_runtime
        .subscribe_agent("agent_dup", "topic_dup".to_string())
        .await;

    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("already subscribed"));

    println!("✓ Error handling test passed: Duplicate subscription detected");
}

/// Test: Error Handling - Nonexistent Agent
#[tokio::test]
async fn test_subscribe_nonexistent_agent() {
    let event_bus = Arc::new(EventBus::new().await.unwrap());
    let action_broker = Arc::new(ActionBroker::new());
    let model_router = ModelRouter::new().await.unwrap();

    let agent_runtime = AgentRuntime::new(event_bus, action_broker, model_router)
        .await
        .unwrap();

    let result = agent_runtime
        .subscribe_agent("nonexistent", "topic".to_string())
        .await;

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not found"));

    println!("✓ Error handling test passed: Nonexistent agent detected");
}
