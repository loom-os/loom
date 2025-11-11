//! Private Agent Reply Topic Tests
//!
//! Tests the automatic subscription to private agent reply topics
//! and point-to-point agent communication patterns.

use super::*;

/// Test: Agent Auto-subscribed to Private Reply Topic
///
/// Validates that every agent is automatically subscribed to its
/// private reply topic `agent.{agent_id}.replies` at creation.
#[tokio::test]
async fn test_agent_auto_subscribed_to_private_reply_topic() {
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
        agent_id: "agent_1".to_string(),
        agent_type: "test".to_string(),
        subscribed_topics: vec![], // No explicit subscriptions
        capabilities: vec![],
        parameters: HashMap::new(),
    };

    agent_runtime.create_agent(config, behavior).await.unwrap();
    sleep(Duration::from_millis(100)).await;

    // Verify agent is subscribed to its private reply topic
    let subs = agent_runtime.get_agent_subscriptions("agent_1").unwrap();
    assert!(subs.contains(&"agent.agent_1.replies".to_string()));

    // Publish directly to private reply topic
    let event = Event {
        id: "evt_private".to_string(),
        r#type: "private_message".to_string(),
        timestamp_ms: chrono::Utc::now().timestamp_millis(),
        source: "agent_2".to_string(),
        metadata: HashMap::new(),
        payload: b"Private message".to_vec(),
        confidence: 1.0,
        tags: vec![],
        priority: 50,
    };

    event_bus
        .publish("agent.agent_1.replies", event)
        .await
        .unwrap();

    sleep(Duration::from_millis(200)).await;

    // Verify agent received the private message
    let events = received.lock().await;
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].id, "evt_private");

    println!("✓ Agent auto-subscribed to private reply topic");
}

/// Test: Point-to-Point Agent Communication
///
/// Validates that agents can send messages directly to each other
/// using the private reply topics without thread involvement.
#[tokio::test]
async fn test_point_to_point_agent_communication() {
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

    // Create two agents
    let received_1 = Arc::new(Mutex::new(Vec::new()));
    let received_2 = Arc::new(Mutex::new(Vec::new()));

    let behavior_1 = Box::new(MockEchoBehavior {
        received_events: Arc::clone(&received_1),
    });
    let behavior_2 = Box::new(MockEchoBehavior {
        received_events: Arc::clone(&received_2),
    });

    let config_1 = AgentConfig {
        agent_id: "sender".to_string(),
        agent_type: "requester".to_string(),
        subscribed_topics: vec![],
        capabilities: vec![],
        parameters: HashMap::new(),
    };

    let config_2 = AgentConfig {
        agent_id: "receiver".to_string(),
        agent_type: "responder".to_string(),
        subscribed_topics: vec![],
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

    // Sender sends message directly to receiver's private topic
    let message = Event {
        id: "evt_p2p".to_string(),
        r#type: "direct_message".to_string(),
        timestamp_ms: chrono::Utc::now().timestamp_millis(),
        source: "sender".to_string(),
        metadata: {
            let mut m = HashMap::new();
            m.insert("sender".to_string(), "agent.sender".to_string());
            m
        },
        payload: b"Direct question".to_vec(),
        confidence: 1.0,
        tags: vec![],
        priority: 50,
    };

    // Use the helper function to get receiver's private topic
    let receiver_topic = loom_core::agent_reply_topic("receiver");
    event_bus.publish(&receiver_topic, message).await.unwrap();

    sleep(Duration::from_millis(200)).await;

    // Verify only receiver got the message
    {
        let events_1 = received_1.lock().await;
        assert_eq!(
            events_1.len(),
            0,
            "Sender should not receive its own message"
        );

        let events_2 = received_2.lock().await;
        assert_eq!(events_2.len(), 1);
        assert_eq!(events_2[0].id, "evt_p2p");
    }

    println!("✓ Point-to-point communication works correctly");
}

/// Test: Envelope Helper - agent_reply_topic
///
/// Validates the Envelope::agent_reply_topic() helper method.
#[tokio::test]
async fn test_envelope_agent_reply_topic_helper() {
    use loom_core::Envelope;

    let env = Envelope::new("req-1", "agent.worker-1");
    assert_eq!(env.agent_reply_topic(), "agent.worker-1.replies");

    // Test with different sender format
    let mut env2 = Envelope::new("req-2", "coordinator");
    env2.sender = "agent.coordinator-5".to_string();
    assert_eq!(env2.agent_reply_topic(), "agent.coordinator-5.replies");

    // Test with invalid sender format
    let env3 = Envelope::new("req-3", "invalid_sender");
    assert_eq!(env3.agent_reply_topic(), "");

    println!("✓ Envelope::agent_reply_topic() helper works correctly");
}

/// Test: Envelope with_agent_reply Constructor
///
/// Validates creating envelopes with agent-specific reply topics.
#[tokio::test]
async fn test_envelope_with_agent_reply_constructor() {
    use loom_core::Envelope;

    let env = Envelope::with_agent_reply("task-1", "agent.coordinator", "agent.coordinator");
    assert_eq!(env.thread_id, "task-1");
    assert_eq!(env.sender, "agent.coordinator");
    assert_eq!(env.reply_to, "agent.agent.coordinator.replies");

    // Verify it's different from thread reply
    let thread_env = Envelope::new("task-1", "agent.coordinator");
    assert_eq!(thread_env.reply_to, "thread.task-1.reply");
    assert_ne!(env.reply_to, thread_env.reply_to);

    println!("✓ Envelope::with_agent_reply() constructor works correctly");
}

/// Test: Private Reply vs Thread Reply Semantics
///
/// Demonstrates the distinction between thread-scoped replies
/// and agent-specific private replies.
#[tokio::test]
async fn test_private_reply_vs_thread_reply() {
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

    // Create agent subscribed to thread
    let thread_id = "collab-1";
    let thread_reply_topic = format!("thread.{}.reply", thread_id);

    let received = Arc::new(Mutex::new(Vec::new()));
    let behavior = Box::new(MockEchoBehavior {
        received_events: Arc::clone(&received),
    });

    let config = AgentConfig {
        agent_id: "agent_dual".to_string(),
        agent_type: "test".to_string(),
        subscribed_topics: vec![thread_reply_topic.clone()],
        capabilities: vec![],
        parameters: HashMap::new(),
    };

    agent_runtime.create_agent(config, behavior).await.unwrap();
    sleep(Duration::from_millis(100)).await;

    // Verify agent has both subscriptions
    let subs = agent_runtime.get_agent_subscriptions("agent_dual").unwrap();
    assert_eq!(subs.len(), 2);
    assert!(subs.contains(&"agent.agent_dual.replies".to_string()));
    assert!(subs.contains(&thread_reply_topic));

    // Send to thread reply topic
    let thread_event = Event {
        id: "evt_thread".to_string(),
        r#type: "thread_reply".to_string(),
        timestamp_ms: chrono::Utc::now().timestamp_millis(),
        source: "participant".to_string(),
        metadata: HashMap::new(),
        payload: b"Thread reply".to_vec(),
        confidence: 1.0,
        tags: vec![],
        priority: 50,
    };

    event_bus
        .publish(&thread_reply_topic, thread_event)
        .await
        .unwrap();

    sleep(Duration::from_millis(100)).await;

    // Send to private reply topic
    let private_event = Event {
        id: "evt_private".to_string(),
        r#type: "private_reply".to_string(),
        timestamp_ms: chrono::Utc::now().timestamp_millis(),
        source: "specific_agent".to_string(),
        metadata: HashMap::new(),
        payload: b"Private reply".to_vec(),
        confidence: 1.0,
        tags: vec![],
        priority: 50,
    };

    event_bus
        .publish("agent.agent_dual.replies", private_event)
        .await
        .unwrap();

    sleep(Duration::from_millis(200)).await;

    // Verify agent received both
    let events = received.lock().await;
    assert_eq!(events.len(), 2);
    let event_ids: Vec<&str> = events.iter().map(|e| e.id.as_str()).collect();
    assert!(event_ids.contains(&"evt_thread"));
    assert!(event_ids.contains(&"evt_private"));

    println!("✓ Agent can receive both thread replies and private replies");
}

/// Test: Multiple Agents with Private Topics Don't Interfere
///
/// Validates that each agent's private topic is isolated.
#[tokio::test]
async fn test_private_topics_are_isolated() {
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

    // Create three agents
    let mut agents = Vec::new();
    for i in 1..=3 {
        let received = Arc::new(Mutex::new(Vec::new()));
        let behavior = Box::new(MockEchoBehavior {
            received_events: Arc::clone(&received),
        });

        let config = AgentConfig {
            agent_id: format!("agent_{}", i),
            agent_type: "test".to_string(),
            subscribed_topics: vec![],
            capabilities: vec![],
            parameters: HashMap::new(),
        };

        agent_runtime.create_agent(config, behavior).await.unwrap();
        agents.push(received);
    }

    sleep(Duration::from_millis(100)).await;

    // Send message to agent_2's private topic only
    let event = Event {
        id: "evt_targeted".to_string(),
        r#type: "test".to_string(),
        timestamp_ms: chrono::Utc::now().timestamp_millis(),
        source: "test".to_string(),
        metadata: HashMap::new(),
        payload: b"Target agent 2".to_vec(),
        confidence: 1.0,
        tags: vec![],
        priority: 50,
    };

    event_bus
        .publish("agent.agent_2.replies", event)
        .await
        .unwrap();

    sleep(Duration::from_millis(200)).await;

    // Verify only agent_2 received it
    for (i, received) in agents.iter().enumerate() {
        let events = received.lock().await;
        if i == 1 {
            // agent_2 (index 1)
            assert_eq!(events.len(), 1);
            assert_eq!(events[0].id, "evt_targeted");
        } else {
            assert_eq!(
                events.len(),
                0,
                "Agent {} should not receive message",
                i + 1
            );
        }
    }

    println!("✓ Private topics are properly isolated");
}
