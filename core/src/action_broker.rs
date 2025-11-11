use crate::proto::{ActionCall, ActionResult, ActionStatus, CapabilityDescriptor};
use crate::{Envelope, LoomError, Result};
use async_trait::async_trait;
use dashmap::DashMap;
use std::sync::Arc;
use tokio::time::{timeout, Duration};
use tracing::{debug, info, warn};

/// Trait implemented by concrete capability providers (Native, WASM, gRPC/MCP adapters)
#[async_trait]
pub trait CapabilityProvider: Send + Sync {
    /// Static descriptor for discovery/registration
    fn descriptor(&self) -> CapabilityDescriptor;

    /// Invoke the capability with an ActionCall and return an ActionResult
    async fn invoke(&self, call: ActionCall) -> Result<ActionResult>;
}

/// Action/Tool Broker: centralized registry and invoker
pub struct ActionBroker {
    // key: "name:version" for disambiguation
    registry: DashMap<String, Arc<dyn CapabilityProvider>>, // capability name:version -> provider
    // idempotency cache: call_id -> result
    cache: DashMap<String, ActionResult>,
}

impl ActionBroker {
    pub fn new() -> Self {
        Self {
            registry: DashMap::new(),
            cache: DashMap::new(),
        }
    }

    /// Register a provider; later registrations with the same name replace the previous one
    pub fn register_provider(&self, provider: Arc<dyn CapabilityProvider>) {
        let desc = provider.descriptor();
        let key = format!("{}:{}", desc.name, desc.version);
        info!(target: "action_broker", capability = %desc.name, version = %desc.version, "Registering capability provider");
        self.registry.insert(key, provider);
    }

    /// List all registered capabilities
    pub fn list_capabilities(&self) -> Vec<CapabilityDescriptor> {
        self.registry
            .iter()
            .map(|e| e.value().descriptor())
            .collect()
    }

    /// Invoke a capability by name with timeout handling
    pub async fn invoke(&self, mut call: ActionCall) -> Result<ActionResult> {
        let cap_name = call.capability.clone();
        let version = call.version.clone();
        let call_id = call.id.clone();

        // Ensure envelope metadata present in headers
        let env = Envelope::from_metadata(&call.headers, &call_id);
        env.apply_to_action_call(&mut call);

        // Idempotency shortcut
        if let Some(hit) = self.cache.get(&call_id) {
            debug!(target: "action_broker", call_id = %call_id, "Idempotent cache hit");
            return Ok(hit.clone());
        }

        // Resolve provider by name:version if provided, otherwise first match by name
        let provider_arc: Arc<dyn CapabilityProvider> = if !version.is_empty() {
            let key = format!("{}:{}", cap_name, version);
            self.registry
                .get(&key)
                .map(|r| Arc::clone(r.value()))
                .ok_or_else(|| {
                    LoomError::PluginError(format!(
                        "Capability not found: {} (version {})",
                        cap_name, version
                    ))
                })?
        } else {
            // fallback: pick first provider matching the name (undefined order)
            self.registry
                .iter()
                .find(|e| e.key().starts_with(&format!("{}:", cap_name)))
                .map(|e| Arc::clone(e.value()))
                .ok_or_else(|| {
                    LoomError::PluginError(format!("Capability not found: {}", cap_name))
                })?
        };

        let dur = if call.timeout_ms <= 0 {
            30_000
        } else {
            call.timeout_ms
        };
        debug!(target: "action_broker", capability = %cap_name, timeout_ms = dur, "Invoking capability");

        let fut = provider_arc.invoke(call);
        let res = match timeout(Duration::from_millis(dur as u64), fut).await {
            Ok(Ok(res)) => res,
            Ok(Err(err)) => {
                warn!(target: "action_broker", capability = %cap_name, error = %err, "Capability error");
                ActionResult {
                    id: call_id.clone(),
                    status: ActionStatus::ActionError as i32,
                    output: Vec::new(),
                    error: Some(crate::proto::ActionError {
                        code: "CAPABILITY_ERROR".to_string(),
                        message: err.to_string(),
                        details: Default::default(),
                    }),
                }
            }
            Err(_) => {
                warn!(target: "action_broker", capability = %cap_name, "Capability timeout");
                ActionResult {
                    id: call_id,
                    status: ActionStatus::ActionTimeout as i32,
                    output: Vec::new(),
                    error: Some(crate::proto::ActionError {
                        code: "TIMEOUT".to_string(),
                        message: "Action timed out".to_string(),
                        details: Default::default(),
                    }),
                }
            }
        };

        // Cache result for idempotency
        self.cache.insert(res.id.clone(), res.clone());
        self.trim_cache(1024);
        Ok(res)
    }
}

impl Default for ActionBroker {
    fn default() -> Self {
        Self::new()
    }
}

impl ActionBroker {
    fn trim_cache(&self, max: usize) {
        if self.cache.len() > max {
            // remove a few arbitrary entries to keep size under control
            let mut removed = 0;
            for k in self.cache.iter().take(self.cache.len().saturating_sub(max)) {
                if self.cache.remove(k.key()).is_some() {
                    removed += 1;
                }
                if removed >= 16 {
                    break;
                }
            }
        }
    }
}
