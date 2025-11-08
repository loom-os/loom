//! Error Handling Tests
//!
//! Tests error propagation from capability providers through to result events.

use super::*;

/// Test: ActionBroker Error Handling
///
/// Validates error propagation from capability providers through to result events.
///
/// Setup:
/// - Mock failing provider that always returns error
///
/// Flow:
/// 1. Agent invokes "failing" capability
/// 2. Provider throws error
/// 3. ActionBroker catches error and creates error result
/// 4. Error result published as action_result event
#[tokio::test]
async fn test_action_broker_error_propagation() {
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
