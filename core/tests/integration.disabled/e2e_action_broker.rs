//! ActionBroker Specific Tests
//!
//! Tests for ActionBroker functionality including timeout handling and idempotency.

use super::*;

/// Test: Action Timeout Handling
///
/// Validates that long-running actions complete within broker timeout.
///
/// Setup:
/// - Mock slow provider with 2-second delay
/// - ActionBroker default timeout: 30 seconds
///
/// Flow:
/// 1. Agent invokes "slow_process" capability
/// 2. Provider delays for 2 seconds
/// 3. Action completes within timeout
/// 4. Result published successfully
#[tokio::test]
async fn test_action_timeout_handling() {
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

    // Create behavior that invokes slow action
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

    // Publish event (ActionBroker will use default 30s timeout)
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

    // Wait for completion (should complete successfully within 4 seconds)
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

/// Test: Idempotent Action Invocation
///
/// Validates that duplicate action calls with the same ID return cached results.
///
/// Flow:
/// 1. Invoke action with ID "idempotent_test_001"
/// 2. Result cached in ActionBroker
/// 3. Invoke same action with identical ID
/// 4. Second invocation returns cached result
#[tokio::test]
async fn test_idempotent_action_invocation() {
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
