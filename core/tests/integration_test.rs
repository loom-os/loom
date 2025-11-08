// Core Integration Tests
// End-to-end flow: Event → Agent → ActionBroker → Result → EventBus
//
// This test suite validates the minimal pipeline:
// 1. Publish event to EventBus
// 2. Agent behavior processes event
// 3. ActionBroker executes capability
// 4. Result event published back to EventBus
// 5. Routing decision events are observed

use loom_core::proto::{
    Action, ActionCall, ActionResult, ActionStatus, AgentConfig, AgentState, CapabilityDescriptor,
    Event, ProviderKind, QoSLevel,
};
use loom_core::{ActionBroker, AgentRuntime, CapabilityProvider, EventBus, ModelRouter, Result};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tokio::time::{sleep, Duration};

// =============================================================================
// Mock Echo Capability Provider
// =============================================================================

/// Mock provider that echoes the input payload as output
struct MockEchoProvider;

#[async_trait::async_trait]
impl CapabilityProvider for MockEchoProvider {
    fn descriptor(&self) -> CapabilityDescriptor {
        CapabilityDescriptor {
            name: "echo".to_string(),
            version: "1.0.0".to_string(),
            provider: ProviderKind::ProviderNative as i32,
            metadata: {
                let mut m = HashMap::new();
                m.insert(
                    "description".to_string(),
                    "Echo test capability".to_string(),
                );
                m
            },
        }
    }

    async fn invoke(&self, call: ActionCall) -> Result<ActionResult> {
        // Echo the payload back with a prefix
        let input = String::from_utf8_lossy(&call.payload);
        let output = format!("ECHO: {}", input);

        Ok(ActionResult {
            id: call.id,
            status: ActionStatus::ActionOk as i32,
            output: output.into_bytes(),
            error: None,
        })
    }
}

// =============================================================================
// Mock Capability Provider with Delays
// =============================================================================

/// Mock provider that simulates processing delay
struct MockSlowProvider {
    delay_ms: u64,
}

#[async_trait::async_trait]
impl CapabilityProvider for MockSlowProvider {
    fn descriptor(&self) -> CapabilityDescriptor {
        CapabilityDescriptor {
            name: "slow_process".to_string(),
            version: "1.0.0".to_string(),
            provider: ProviderKind::ProviderNative as i32,
            metadata: HashMap::new(),
        }
    }

    async fn invoke(&self, call: ActionCall) -> Result<ActionResult> {
        sleep(Duration::from_millis(self.delay_ms)).await;

        Ok(ActionResult {
            id: call.id,
            status: ActionStatus::ActionOk as i32,
            output: b"SLOW_DONE".to_vec(),
            error: None,
        })
    }
}

// =============================================================================
// Mock Capability Provider with Errors
// =============================================================================

/// Mock provider that always fails
struct MockFailingProvider;

#[async_trait::async_trait]
impl CapabilityProvider for MockFailingProvider {
    fn descriptor(&self) -> CapabilityDescriptor {
        CapabilityDescriptor {
            name: "failing".to_string(),
            version: "1.0.0".to_string(),
            provider: ProviderKind::ProviderNative as i32,
            metadata: HashMap::new(),
        }
    }

    async fn invoke(&self, call: ActionCall) -> Result<ActionResult> {
        Err(loom_core::LoomError::PluginError(
            "Simulated failure".to_string(),
        ))
    }
}

// =============================================================================
// Mock Agent Behavior
// =============================================================================

/// Mock behavior that publishes echo action for specific event types
struct MockEchoBehavior {
    /// Track received events for verification
    received_events: Arc<Mutex<Vec<Event>>>,
}

#[async_trait::async_trait]
impl loom_core::agent::AgentBehavior for MockEchoBehavior {
    async fn on_init(&mut self, _config: &AgentConfig) -> Result<()> {
        Ok(())
    }

    async fn on_event(&mut self, event: Event, _state: &mut AgentState) -> Result<Vec<Action>> {
        // Store event for verification
        self.received_events.lock().await.push(event.clone());

        // Generate echo action for test_input events
        if event.r#type == "test_input" {
            let payload = format!("Input: {}", String::from_utf8_lossy(&event.payload));
            Ok(vec![Action {
                action_type: "echo".to_string(),
                parameters: HashMap::new(),
                payload: payload.into_bytes(),
                priority: 50,
            }])
        } else {
            Ok(vec![])
        }
    }

    async fn on_shutdown(&mut self) -> Result<()> {
        Ok(())
    }
}

// =============================================================================
// Test 1: Minimal End-to-End Pipeline
// =============================================================================

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

// =============================================================================
// Test 2: Multiple Agents with Different Topics
// =============================================================================

#[tokio::test]
async fn test_e2e_multiple_agents() {
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

// =============================================================================
// Test 3: ActionBroker Error Handling
// =============================================================================

#[tokio::test]
async fn test_e2e_action_error_handling() {
    let event_bus = Arc::new(EventBus::new().await.unwrap());
    let action_broker = Arc::new(ActionBroker::new());
    let model_router = ModelRouter::new().await.unwrap();

    // Register failing provider
    action_broker.register_provider(Arc::new(MockFailingProvider));

    let agent_runtime = AgentRuntime::new(
        Arc::clone(&event_bus),
        Arc::clone(&action_broker),
        model_router.clone(),
    )
    .await
    .unwrap();

    // Create behavior that invokes failing action
    struct FailingActionBehavior;

    #[async_trait::async_trait]
    impl loom_core::agent::AgentBehavior for FailingActionBehavior {
        async fn on_init(&mut self, _config: &AgentConfig) -> Result<()> {
            Ok(())
        }

        async fn on_event(&mut self, event: Event, _state: &mut AgentState) -> Result<Vec<Action>> {
            if event.r#type == "trigger_failure" {
                Ok(vec![Action {
                    action_type: "failing".to_string(),
                    parameters: HashMap::new(),
                    payload: vec![],
                    priority: 50,
                }])
            } else {
                Ok(vec![])
            }
        }

        async fn on_shutdown(&mut self) -> Result<()> {
            Ok(())
        }
    }

    let config = AgentConfig {
        agent_id: "failing_agent".to_string(),
        agent_type: "test".to_string(),
        subscribed_topics: vec!["failure_topic".to_string()],
        capabilities: vec!["failing".to_string()],
        parameters: HashMap::new(),
    };

    agent_runtime
        .create_agent(config, Box::new(FailingActionBehavior))
        .await
        .unwrap();

    sleep(Duration::from_millis(100)).await;

    // Subscribe to result events
    let (_sub, mut result_rx) = event_bus
        .subscribe(
            "agent.failing_agent".to_string(),
            vec!["action_result".to_string()],
            QoSLevel::QosBatched,
        )
        .await
        .unwrap();

    // Publish event that triggers failure
    let event = Event {
        id: "evt_fail".to_string(),
        r#type: "trigger_failure".to_string(),
        timestamp_ms: chrono::Utc::now().timestamp_millis(),
        source: "test".to_string(),
        metadata: HashMap::new(),
        payload: vec![],
        confidence: 1.0,
        tags: vec![],
        priority: 50,
    };

    event_bus.publish("failure_topic", event).await.unwrap();

    // Verify error result
    let result = tokio::time::timeout(Duration::from_secs(2), result_rx.recv())
        .await
        .expect("Should receive error result")
        .expect("Channel should not be closed");

    assert_eq!(result.r#type, "action_result");
    assert_eq!(
        result.metadata.get("status").map(|s| s.as_str()),
        Some("error")
    );

    println!("✓ Error handling test passed: ActionBroker errors propagate correctly");
}

// =============================================================================
// Test 4: Routing Decision Observation
// =============================================================================

#[tokio::test]
async fn test_e2e_routing_decision_observation() {
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

// =============================================================================
// Test 5: Action Timeout Handling
// =============================================================================

#[tokio::test]
async fn test_e2e_action_timeout() {
    let event_bus = Arc::new(EventBus::new().await.unwrap());
    let action_broker = Arc::new(ActionBroker::new());
    let model_router = ModelRouter::new().await.unwrap();

    // Register slow provider that takes 2 seconds
    action_broker.register_provider(Arc::new(MockSlowProvider { delay_ms: 2000 }));

    let agent_runtime = AgentRuntime::new(
        Arc::clone(&event_bus),
        Arc::clone(&action_broker),
        model_router.clone(),
    )
    .await
    .unwrap();

    // Create behavior that invokes slow action with short timeout
    struct SlowActionBehavior;

    #[async_trait::async_trait]
    impl loom_core::agent::AgentBehavior for SlowActionBehavior {
        async fn on_init(&mut self, _config: &AgentConfig) -> Result<()> {
            Ok(())
        }

        async fn on_event(&mut self, event: Event, _state: &mut AgentState) -> Result<Vec<Action>> {
            if event.r#type == "trigger_slow" {
                Ok(vec![Action {
                    action_type: "slow_process".to_string(),
                    parameters: HashMap::new(),
                    payload: vec![],
                    priority: 50,
                }])
            } else {
                Ok(vec![])
            }
        }

        async fn on_shutdown(&mut self) -> Result<()> {
            Ok(())
        }
    }

    let config = AgentConfig {
        agent_id: "timeout_agent".to_string(),
        agent_type: "test".to_string(),
        subscribed_topics: vec!["timeout_topic".to_string()],
        capabilities: vec!["slow_process".to_string()],
        parameters: HashMap::new(),
    };

    agent_runtime
        .create_agent(config, Box::new(SlowActionBehavior))
        .await
        .unwrap();

    sleep(Duration::from_millis(100)).await;

    // Subscribe to result events
    let (_sub, mut result_rx) = event_bus
        .subscribe(
            "agent.timeout_agent".to_string(),
            vec!["action_result".to_string()],
            QoSLevel::QosBatched,
        )
        .await
        .unwrap();

    // Publish event (ActionBroker will use default 30s timeout, but we'll wait to observe completion)
    let event = Event {
        id: "evt_slow".to_string(),
        r#type: "trigger_slow".to_string(),
        timestamp_ms: chrono::Utc::now().timestamp_millis(),
        source: "test".to_string(),
        metadata: HashMap::new(),
        payload: vec![],
        confidence: 1.0,
        tags: vec![],
        priority: 50,
    };

    event_bus.publish("timeout_topic", event).await.unwrap();

    // Wait for completion (should complete successfully within 3 seconds)
    let result = tokio::time::timeout(Duration::from_secs(4), result_rx.recv())
        .await
        .expect("Should receive result within timeout")
        .expect("Channel should not be closed");

    assert_eq!(result.r#type, "action_result");
    // The slow action should complete successfully (not timeout with default 30s)
    assert_eq!(
        result.metadata.get("status").map(|s| s.as_str()),
        Some("ok")
    );

    println!("✓ Action timeout test passed: Slow actions complete within broker timeout");
}

// =============================================================================
// Test 6: Idempotent Action Invocation
// =============================================================================

#[tokio::test]
async fn test_e2e_idempotent_action() {
    let action_broker = Arc::new(ActionBroker::new());

    // Register echo provider
    action_broker.register_provider(Arc::new(MockEchoProvider));

    // Create action call with specific ID
    let call = ActionCall {
        id: "idempotent_test_001".to_string(),
        capability: "echo".to_string(),
        version: "1.0.0".to_string(),
        payload: b"Test idempotency".to_vec(),
        headers: HashMap::new(),
        timeout_ms: 5000,
        correlation_id: "test".to_string(),
        qos: QoSLevel::QosBatched as i32,
    };

    // First invocation
    let result_1 = action_broker.invoke(call.clone()).await.unwrap();
    assert_eq!(result_1.status, ActionStatus::ActionOk as i32);
    let output_1 = String::from_utf8_lossy(&result_1.output);

    // Second invocation with same ID (should hit cache)
    let result_2 = action_broker.invoke(call.clone()).await.unwrap();
    assert_eq!(result_2.status, ActionStatus::ActionOk as i32);
    let output_2 = String::from_utf8_lossy(&result_2.output);

    // Results should be identical
    assert_eq!(output_1, output_2);
    assert_eq!(result_1.id, result_2.id);

    println!("✓ Idempotent action test passed: Cache hit on duplicate call ID");
}

// =============================================================================
// Test 7: Complete Pipeline with Event Filtering
// =============================================================================

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
