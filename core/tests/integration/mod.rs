//! Integration Test Module
//!
//! This module contains end-to-end integration tests for loom-core.
//! Tests are organized by functionality:
//!
//! - `e2e_basic`: Basic end-to-end pipeline tests
//! - `e2e_multi_agent`: Multi-agent interaction tests
//! - `e2e_error_handling`: Error propagation tests
//! - `e2e_routing`: Routing decision tests
//! - `e2e_action_broker`: ActionBroker-specific tests

use loom_core::proto::{
    Action, ActionCall, ActionResult, ActionStatus, AgentConfig, AgentState, CapabilityDescriptor,
    Event, ProviderKind, QoSLevel,
};
use loom_core::{CapabilityProvider, Result};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};

// Re-export commonly used types
pub use loom_core::{ActionBroker, AgentRuntime, EventBus, ModelRouter};

// Submodules
mod e2e_action_broker;
mod e2e_basic;
mod e2e_error_handling;
mod e2e_multi_agent;
mod e2e_routing;
mod e2e_tool_use;

// =============================================================================
// Shared Mock Components
// =============================================================================

/// Mock provider that echoes the input payload as output
pub struct MockEchoProvider;

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

/// Mock provider that simulates processing delay
pub struct MockSlowProvider {
    pub delay_ms: u64,
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

/// Mock provider that always fails
pub struct MockFailingProvider;

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

    async fn invoke(&self, _call: ActionCall) -> Result<ActionResult> {
        Err(loom_core::LoomError::PluginError(
            "Simulated failure".to_string(),
        ))
    }
}

/// Mock behavior that publishes echo action for specific event types
pub struct MockEchoBehavior {
    /// Track received events for verification
    pub received_events: Arc<Mutex<Vec<Event>>>,
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
