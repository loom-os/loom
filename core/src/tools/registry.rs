use super::error::{ToolError, ToolResult};
use super::traits::Tool;
use dashmap::DashMap;
use opentelemetry::{
    global,
    metrics::{Counter, Histogram, UpDownCounter},
    KeyValue,
};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;
use tracing::{debug, info, warn};

/// A registry for managing available tools
#[derive(Clone)]
pub struct ToolRegistry {
    tools: Arc<DashMap<String, Arc<dyn Tool>>>,

    // OpenTelemetry metrics
    invocations_counter: Counter<u64>,
    errors_counter: Counter<u64>,
    timeouts_counter: Counter<u64>,
    invoke_latency: Histogram<f64>,
    registered_tools_gauge: UpDownCounter<i64>,
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolRegistry {
    pub fn new() -> Self {
        let meter = global::meter("loom.tool_registry");

        let invocations_counter = meter
            .u64_counter("loom.tool_registry.invocations_total")
            .with_description("Total number of tool invocations")
            .init();

        let errors_counter = meter
            .u64_counter("loom.tool_registry.errors_total")
            .with_description("Total number of tool errors")
            .init();

        let timeouts_counter = meter
            .u64_counter("loom.tool_registry.timeouts_total")
            .with_description("Total number of tool timeouts")
            .init();

        let invoke_latency = meter
            .f64_histogram("loom.tool_registry.invoke_latency_ms")
            .with_description("Tool invocation latency in milliseconds")
            .init();

        let registered_tools_gauge = meter
            .i64_up_down_counter("loom.tool_registry.registered_tools")
            .with_description("Number of registered tools")
            .init();

        Self {
            tools: Arc::new(DashMap::new()),
            invocations_counter,
            errors_counter,
            timeouts_counter,
            invoke_latency,
            registered_tools_gauge,
        }
    }

    /// Register a new tool
    pub async fn register(&self, tool: Arc<dyn Tool>) {
        let name = tool.name();
        info!(target: "tool_registry", tool = %name, "Registering tool");

        if self.tools.insert(name.clone(), tool).is_none() {
            self.registered_tools_gauge.add(1, &[]);
        }
    }

    /// Get a tool by name
    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.get(name).map(|t| t.clone())
    }

    /// List all registered tools
    pub fn list_tools(&self) -> Vec<Arc<dyn Tool>> {
        self.tools.iter().map(|t| t.clone()).collect()
    }

    /// Call a tool by name with timeout
    #[tracing::instrument(skip(self, arguments), fields(tool.name = %name))]
    pub async fn call(
        &self,
        name: &str,
        arguments: serde_json::Value,
    ) -> ToolResult<serde_json::Value> {
        let start_time = std::time::Instant::now();

        let tool = self
            .get(name)
            .ok_or_else(|| ToolError::NotFound(name.to_string()))?;

        debug!(target: "tool_registry", tool = %name, "Invoking tool");

        // Default timeout 30s, TODO: make configurable
        let timeout_duration = Duration::from_secs(30);

        // Execute tool with timeout
        let fut = tool.call(arguments);
        let result = match timeout(timeout_duration, fut).await {
            Ok(res) => res,
            Err(_) => {
                warn!(target: "tool_registry", tool = %name, "Tool execution timed out");
                self.timeouts_counter
                    .add(1, &[KeyValue::new("tool", name.to_string())]);
                Err(ToolError::Timeout)
            }
        };

        let elapsed_ms = start_time.elapsed().as_secs_f64() * 1000.0;
        self.invoke_latency
            .record(elapsed_ms, &[KeyValue::new("tool", name.to_string())]);

        match &result {
            Ok(_) => {
                self.invocations_counter.add(
                    1,
                    &[
                        KeyValue::new("tool", name.to_string()),
                        KeyValue::new("status", "success"),
                    ],
                );
            }
            Err(e) => {
                warn!(target: "tool_registry", tool = %name, error = %e, "Tool execution failed");
                self.errors_counter.add(
                    1,
                    &[
                        KeyValue::new("tool", name.to_string()),
                        KeyValue::new("error", e.to_string()),
                    ],
                );
            }
        }

        result
    }
}
