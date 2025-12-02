//! Action Broker Adapter
//!
//! Provides a compatibility layer between the new ToolRegistry-based architecture
//! and the existing ActionBroker gRPC interface used by remote agents.
//!
//! This adapter allows bridge to use ToolRegistry for local tool execution while
//! maintaining the same gRPC API (ActionCall -> ActionResult) for remote clients.

use async_trait::async_trait;
use loom_core::ToolRegistry;
use loom_proto::{ActionCall, ActionError, ActionResult, ActionStatus, CapabilityDescriptor, ProviderKind};
use std::sync::Arc;
use tracing::{debug, warn};

/// Error types for the ActionBroker
#[derive(thiserror::Error, Debug)]
pub enum ActionBrokerError {
    #[error("Capability not found: {0}")]
    CapabilityNotFound(String),

    #[error("Invocation failed: {0}")]
    InvocationFailed(String),

    #[error("Timeout")]
    Timeout,

    #[error("Invalid payload: {0}")]
    InvalidPayload(String),
}

/// Trait for custom capability providers that can be registered with the broker.
/// This maintains compatibility with the old ActionBroker interface.
#[async_trait]
pub trait CapabilityProvider: Send + Sync {
    /// Returns the capability descriptor for this provider.
    fn descriptor(&self) -> CapabilityDescriptor;

    /// Invoke the capability with the given action call.
    async fn invoke(&self, call: ActionCall) -> Result<ActionResult, ActionBrokerError>;
}

/// ActionBroker adapter that wraps ToolRegistry and custom CapabilityProviders.
///
/// This provides backward compatibility with the existing bridge API while
/// using the new unified tool system internally.
pub struct ActionBroker {
    /// Underlying tool registry for native tools
    tool_registry: Option<Arc<ToolRegistry>>,

    /// Custom capability providers (for legacy compatibility)
    providers: dashmap::DashMap<String, Arc<dyn CapabilityProvider>>,
}

impl ActionBroker {
    /// Create a new ActionBroker without a ToolRegistry.
    /// Use this for testing or when only custom providers are needed.
    pub fn new() -> Self {
        Self {
            tool_registry: None,
            providers: dashmap::DashMap::new(),
        }
    }

    /// Create a new ActionBroker with a ToolRegistry.
    pub fn with_registry(registry: Arc<ToolRegistry>) -> Self {
        Self {
            tool_registry: Some(registry),
            providers: dashmap::DashMap::new(),
        }
    }

    /// Register a custom capability provider.
    pub fn register_provider(&self, provider: Arc<dyn CapabilityProvider>) {
        let name = provider.descriptor().name.clone();
        debug!(capability = %name, "Registering capability provider");
        self.providers.insert(name, provider);
    }

    /// List all available capabilities.
    pub fn list_capabilities(&self) -> Vec<CapabilityDescriptor> {
        let mut caps = Vec::new();

        // Add capabilities from custom providers
        for entry in self.providers.iter() {
            caps.push(entry.value().descriptor());
        }

        // Add capabilities from tool registry
        if let Some(ref registry) = self.tool_registry {
            for tool in registry.list_tools() {
                caps.push(CapabilityDescriptor {
                    name: tool.name(),
                    version: "1.0.0".to_string(),
                    provider: ProviderKind::ProviderNative as i32,
                    metadata: {
                        let mut m = std::collections::HashMap::new();
                        m.insert("description".to_string(), tool.description());
                        m.insert("parameters".to_string(), tool.parameters().to_string());
                        m
                    },
                });
            }
        }

        caps
    }

    /// Invoke a capability by name.
    pub async fn invoke(&self, call: ActionCall) -> Result<ActionResult, ActionBrokerError> {
        let capability = &call.capability;

        // First, check custom providers
        if let Some(provider) = self.providers.get(capability) {
            debug!(capability = %capability, "Invoking custom provider");
            return provider.invoke(call).await;
        }

        // Then, check tool registry
        if let Some(ref registry) = self.tool_registry {
            if registry.get(capability).is_some() {
                debug!(capability = %capability, "Invoking tool from registry");

                // Parse payload as JSON
                let arguments: serde_json::Value = if call.payload.is_empty() {
                    serde_json::json!({})
                } else {
                    serde_json::from_slice(&call.payload)
                        .map_err(|e| ActionBrokerError::InvalidPayload(e.to_string()))?
                };

                // Invoke through registry
                match registry.call(capability, arguments).await {
                    Ok(result) => {
                        let output = serde_json::to_vec(&result)
                            .unwrap_or_default();

                        return Ok(ActionResult {
                            id: call.id,
                            status: ActionStatus::ActionOk as i32,
                            output,
                            error: None,
                        });
                    }
                    Err(e) => {
                        warn!(capability = %capability, error = %e, "Tool invocation failed");
                        return Ok(ActionResult {
                            id: call.id,
                            status: ActionStatus::ActionError as i32,
                            output: Vec::new(),
                            error: Some(ActionError {
                                code: "TOOL_ERROR".to_string(),
                                message: e.to_string(),
                                details: Default::default(),
                            }),
                        });
                    }
                }
            }
        }

        // Capability not found
        warn!(capability = %capability, "Capability not found");
        Err(ActionBrokerError::CapabilityNotFound(capability.clone()))
    }
}

impl Default for ActionBroker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct EchoProvider;

    #[async_trait]
    impl CapabilityProvider for EchoProvider {
        fn descriptor(&self) -> CapabilityDescriptor {
            CapabilityDescriptor {
                name: "test.echo".into(),
                version: "1.0".into(),
                provider: ProviderKind::ProviderNative as i32,
                metadata: Default::default(),
            }
        }

        async fn invoke(&self, call: ActionCall) -> Result<ActionResult, ActionBrokerError> {
            Ok(ActionResult {
                id: call.id,
                status: ActionStatus::ActionOk as i32,
                output: call.payload,
                error: None,
            })
        }
    }

    #[tokio::test]
    async fn test_custom_provider() {
        let broker = ActionBroker::new();
        broker.register_provider(Arc::new(EchoProvider));

        let call = ActionCall {
            id: "test-1".into(),
            capability: "test.echo".into(),
            version: "1.0".into(),
            payload: b"hello".to_vec(),
            headers: Default::default(),
            timeout_ms: 1000,
            correlation_id: "c1".into(),
            qos: 0,
        };

        let result = broker.invoke(call).await.unwrap();
        assert_eq!(result.status, ActionStatus::ActionOk as i32);
        assert_eq!(result.output, b"hello".to_vec());
    }

    #[tokio::test]
    async fn test_capability_not_found() {
        let broker = ActionBroker::new();

        let call = ActionCall {
            id: "test-2".into(),
            capability: "nonexistent".into(),
            version: "1.0".into(),
            payload: vec![],
            headers: Default::default(),
            timeout_ms: 1000,
            correlation_id: "c2".into(),
            qos: 0,
        };

        let result = broker.invoke(call).await;
        assert!(matches!(result, Err(ActionBrokerError::CapabilityNotFound(_))));
    }
}
