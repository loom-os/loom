use crate::proto::{ActionCall, ActionResult, ActionStatus, CapabilityDescriptor};
use crate::{Envelope, LoomError, Result};
use async_trait::async_trait;
use dashmap::DashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::time::{timeout, Duration};
use tracing::{debug, info, warn, Span};

// OpenTelemetry imports
use opentelemetry::{
    global,
    metrics::{Counter, Histogram, UpDownCounter},
    KeyValue,
};

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

    // OpenTelemetry metrics
    invocations_counter: Counter<u64>,
    cache_hits_counter: Counter<u64>,
    timeouts_counter: Counter<u64>,
    errors_counter: Counter<u64>,
    invoke_latency: Histogram<f64>,
    registered_capabilities_gauge: UpDownCounter<i64>,
}

impl ActionBroker {
    pub fn new() -> Self {
        // Initialize OpenTelemetry metrics
        let meter = global::meter("loom.action_broker");

        let invocations_counter = meter
            .u64_counter("loom.action_broker.invocations_total")
            .with_description("Total number of capability invocations")
            .init();

        let cache_hits_counter = meter
            .u64_counter("loom.action_broker.cache_hits_total")
            .with_description("Total number of cache hits")
            .init();

        let timeouts_counter = meter
            .u64_counter("loom.action_broker.timeouts_total")
            .with_description("Total number of timeouts")
            .init();

        let errors_counter = meter
            .u64_counter("loom.action_broker.errors_total")
            .with_description("Total number of errors")
            .init();

        let invoke_latency = meter
            .f64_histogram("loom.action_broker.invoke_latency_ms")
            .with_description("Capability invocation latency in milliseconds")
            .init();

        let registered_capabilities_gauge = meter
            .i64_up_down_counter("loom.action_broker.registered_capabilities")
            .with_description("Number of registered capabilities")
            .init();

        Self {
            registry: DashMap::new(),
            cache: DashMap::new(),
            invocations_counter,
            cache_hits_counter,
            timeouts_counter,
            errors_counter,
            invoke_latency,
            registered_capabilities_gauge,
        }
    }

    /// Register a provider; later registrations with the same name replace the previous one
    #[tracing::instrument(skip(self, provider), fields(capability, version, provider_type))]
    pub fn register_provider(&self, provider: Arc<dyn CapabilityProvider>) {
        let desc = provider.descriptor();
        let key = format!("{}:{}", desc.name, desc.version);

        // Record span attributes
        Span::current().record("capability", &desc.name);
        Span::current().record("version", &desc.version);
        Span::current().record("provider_type", desc.provider);

        info!(target: "action_broker", capability = %desc.name, version = %desc.version, "Registering capability provider");
        self.registry.insert(key, provider);

        // Update registered capabilities gauge
        self.registered_capabilities_gauge.add(
            1,
            &[KeyValue::new(
                "provider_type",
                format!("{:?}", desc.provider),
            )],
        );
    }

    /// List all registered capabilities
    pub fn list_capabilities(&self) -> Vec<CapabilityDescriptor> {
        self.registry
            .iter()
            .map(|e| e.value().descriptor())
            .collect()
    }

    /// Invoke a capability by name with timeout handling
    #[tracing::instrument(skip(self, call), fields(capability = %call.capability, version = %call.version, call_id = %call.id, timeout_ms = call.timeout_ms))]
    pub async fn invoke(&self, mut call: ActionCall) -> Result<ActionResult> {
        let start_time = Instant::now();
        let cap_name = call.capability.clone();
        let version = call.version.clone();
        let call_id = call.id.clone();

        // Ensure envelope metadata present in headers
        let env = Envelope::from_metadata(&call.headers, &call_id);
        env.apply_to_action_call(&mut call);

        // Idempotency shortcut
        if let Some(hit) = self.cache.get(&call_id) {
            debug!(target: "action_broker", call_id = %call_id, "Idempotent cache hit");

            // Record cache hit metric
            self.cache_hits_counter
                .add(1, &[KeyValue::new("capability", cap_name.clone())]);
            Span::current().record("cache_hit", true);

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
            Ok(Ok(res)) => {
                // Success case
                let status_str = if res.status == (ActionStatus::ActionOk as i32) {
                    "success"
                } else {
                    "error"
                };

                self.invocations_counter.add(
                    1,
                    &[
                        KeyValue::new("capability", cap_name.clone()),
                        KeyValue::new("status", status_str),
                    ],
                );
                Span::current().record("status", status_str);

                res
            }
            Ok(Err(err)) => {
                warn!(target: "action_broker", capability = %cap_name, error = %err, "Capability error");

                // Record error metric
                self.errors_counter.add(
                    1,
                    &[
                        KeyValue::new("capability", cap_name.clone()),
                        KeyValue::new("error_code", "CAPABILITY_ERROR"),
                    ],
                );
                self.invocations_counter.add(
                    1,
                    &[
                        KeyValue::new("capability", cap_name.clone()),
                        KeyValue::new("status", "error"),
                    ],
                );

                Span::current().record("status", "error");
                Span::current().record("error_code", "CAPABILITY_ERROR");

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

                // Record timeout metrics
                self.timeouts_counter
                    .add(1, &[KeyValue::new("capability", cap_name.clone())]);
                self.errors_counter.add(
                    1,
                    &[
                        KeyValue::new("capability", cap_name.clone()),
                        KeyValue::new("error_code", "TIMEOUT"),
                    ],
                );
                self.invocations_counter.add(
                    1,
                    &[
                        KeyValue::new("capability", cap_name.clone()),
                        KeyValue::new("status", "timeout"),
                    ],
                );

                Span::current().record("status", "timeout");

                ActionResult {
                    id: call_id.clone(),
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

        // Record invocation latency
        let elapsed_ms = start_time.elapsed().as_secs_f64() * 1000.0;
        self.invoke_latency.record(
            elapsed_ms,
            &[
                KeyValue::new("capability", cap_name),
                KeyValue::new("version", version),
            ],
        );
        Span::current().record("latency_ms", elapsed_ms);

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
