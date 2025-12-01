// Telemetry and observability with OpenTelemetry support
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};
use tokio::sync::RwLock;
use tracing::info;

use opentelemetry::trace::TracerProvider as _;
use opentelemetry::KeyValue;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::export::trace::SpanData as OtelSpanData;
use opentelemetry_sdk::trace::SpanProcessor;
use opentelemetry_sdk::Resource;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

/// Performance metrics (legacy, kept for backward compatibility)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metrics {
    pub total_events: u64,
    pub events_per_second: f64,
    pub avg_latency_ms: f64,
    pub p99_latency_ms: f64,
    pub cloud_fallback_rate: f64,
    pub error_rate: f64,
}

/// Metrics collector (legacy)
pub struct MetricsCollector {
    metrics: Arc<RwLock<Metrics>>,
    latencies: Arc<RwLock<Vec<Duration>>>,
    start_time: Instant,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(RwLock::new(Metrics {
                total_events: 0,
                events_per_second: 0.0,
                avg_latency_ms: 0.0,
                p99_latency_ms: 0.0,
                cloud_fallback_rate: 0.0,
                error_rate: 0.0,
            })),
            latencies: Arc::new(RwLock::new(Vec::new())),
            start_time: Instant::now(),
        }
    }

    /// Record event
    pub async fn record_event(&self, latency: Duration) {
        let mut metrics = self.metrics.write().await;
        metrics.total_events += 1;

        let mut latencies = self.latencies.write().await;
        latencies.push(latency);

        // Calculate average latency
        let total_ms: f64 = latencies.iter().map(|d| d.as_millis() as f64).sum();
        metrics.avg_latency_ms = total_ms / latencies.len() as f64;

        // Calculate P99
        let mut sorted = latencies.clone();
        sorted.sort();
        let p99_idx = (sorted.len() as f64 * 0.99) as usize;
        if p99_idx < sorted.len() {
            metrics.p99_latency_ms = sorted[p99_idx].as_millis() as f64;
        }

        // Calculate event throughput
        let elapsed = self.start_time.elapsed().as_secs_f64();
        metrics.events_per_second = metrics.total_events as f64 / elapsed;
    }

    /// Get current metrics
    pub async fn get_metrics(&self) -> Metrics {
        self.metrics.read().await.clone()
    }

    /// Print metrics to log
    pub async fn print_metrics(&self) {
        let metrics = self.get_metrics().await;
        info!("=== Metrics ===");
        info!("Total Events: {}", metrics.total_events);
        info!("Events/sec: {:.2}", metrics.events_per_second);
        info!("Avg Latency: {:.2}ms", metrics.avg_latency_ms);
        info!("P99 Latency: {:.2}ms", metrics.p99_latency_ms);
        info!(
            "Cloud Fallback Rate: {:.2}%",
            metrics.cloud_fallback_rate * 100.0
        );
        info!("Error Rate: {:.2}%", metrics.error_rate * 100.0);
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

// ==============================================================================
// OpenTelemetry Integration
// ==============================================================================

/// Initialize OpenTelemetry with OTLP exporter
///
/// Sets up:
/// - Trace exporter to OTLP endpoint (default: http://localhost:4317)
/// - Metrics exporter to OTLP endpoint
/// - Tracing subscriber with OpenTelemetry layer
/// - Resource attributes (service.name, service.version, etc.)
///
/// Returns a SpanCollector for Dashboard Timeline visualization.
///
/// # Environment Variables
///
/// - `OTEL_EXPORTER_OTLP_ENDPOINT`: OTLP collector endpoint (default: http://localhost:4317)
/// - `OTEL_SERVICE_NAME`: Service name (default: loom-core)
/// - `OTEL_TRACE_SAMPLER`: Sampling strategy (default: always_on)
///   - `always_on`: Sample all traces (100%)
///   - `always_off`: Sample no traces
///   - `traceidratio`: Sample based on trace ID ratio (e.g., `traceidratio=0.1` for 10%)
///
/// # Example
///
/// ```no_run
/// use loom_core::telemetry::{init_telemetry, shutdown_telemetry};
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
///     // Initialize telemetry and get SpanCollector
///     let span_collector = init_telemetry()?;
///
///     // Use span_collector for Dashboard APIs
///     let recent = span_collector.get_recent(100).await;
///
///     // Your application code...
///
///     // Shutdown gracefully
///     shutdown_telemetry();
///     Ok(())
/// }
/// ```
pub fn init_telemetry() -> Result<SpanCollector, Box<dyn std::error::Error + Send + Sync>> {
    let otlp_endpoint = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
        .unwrap_or_else(|_| "http://localhost:4317".to_string());

    let service_name =
        std::env::var("OTEL_SERVICE_NAME").unwrap_or_else(|_| "loom-core".to_string());

    info!(
        target: "telemetry",
        otlp_endpoint = %otlp_endpoint,
        service_name = %service_name,
        "Initializing OpenTelemetry"
    );

    // Create resource with service attributes
    let resource = Resource::new(vec![
        KeyValue::new("service.name", service_name.clone()),
        KeyValue::new("service.version", env!("CARGO_PKG_VERSION")),
        KeyValue::new(
            "deployment.environment",
            std::env::var("DEPLOYMENT_ENV").unwrap_or_else(|_| "development".to_string()),
        ),
    ]);

    // Create SpanCollector for Dashboard
    let span_collector = SpanCollector::new();

    // Build trace config with SpanCollector
    use opentelemetry_sdk::trace::TracerProvider;
    let tracer_provider = TracerProvider::builder()
        .with_config(
            opentelemetry_sdk::trace::config()
                .with_resource(resource.clone())
                .with_sampler(get_sampler_from_env()),
        )
        .with_span_processor(span_collector.clone())
        .with_batch_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint(otlp_endpoint.clone())
                .build_span_exporter()?,
            opentelemetry_sdk::runtime::Tokio,
        )
        .build();

    // Set as global tracer provider
    opentelemetry::global::set_tracer_provider(tracer_provider.clone());
    let tracer = tracer_provider.tracer("loom-core");

    // Initialize metrics provider with OTLP exporter
    let meter_provider = opentelemetry_otlp::new_pipeline()
        .metrics(opentelemetry_sdk::runtime::Tokio)
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint(otlp_endpoint),
        )
        .with_resource(resource)
        .with_period(std::time::Duration::from_secs(10))
        .build()?;

    // Set as global meter provider
    opentelemetry::global::set_meter_provider(meter_provider);

    // Create OpenTelemetry tracing layer
    let telemetry_layer = tracing_opentelemetry::layer().with_tracer(tracer);

    // Set up tracing subscriber with OpenTelemetry layer
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(env_filter)
        .with(tracing_subscriber::fmt::layer())
        .with(telemetry_layer)
        .try_init()?;

    info!(
        target: "telemetry",
        "OpenTelemetry initialized successfully with SpanCollector"
    );

    Ok(span_collector)
}

/// Get sampler from environment variable
fn get_sampler_from_env() -> opentelemetry_sdk::trace::Sampler {
    let sampler_str =
        std::env::var("OTEL_TRACE_SAMPLER").unwrap_or_else(|_| "always_on".to_string());

    match sampler_str.as_str() {
        "always_on" => opentelemetry_sdk::trace::Sampler::AlwaysOn,
        "always_off" => opentelemetry_sdk::trace::Sampler::AlwaysOff,
        s if s.starts_with("traceidratio=") => {
            let ratio = s
                .trim_start_matches("traceidratio=")
                .parse::<f64>()
                .unwrap_or(1.0);
            opentelemetry_sdk::trace::Sampler::TraceIdRatioBased(ratio)
        }
        _ => {
            tracing::warn!(
                target: "telemetry",
                sampler = %sampler_str,
                "Unknown sampler, defaulting to always_on"
            );
            opentelemetry_sdk::trace::Sampler::AlwaysOn
        }
    }
}

/// Shutdown OpenTelemetry gracefully
///
/// Flushes all pending traces and metrics before shutting down.
/// Should be called before application exit.
pub fn shutdown_telemetry() {
    info!(target: "telemetry", "Shutting down OpenTelemetry");

    // Shutdown tracer provider
    opentelemetry::global::shutdown_tracer_provider();

    // Note: MeterProvider will automatically flush on drop

    info!(target: "telemetry", "OpenTelemetry shutdown complete");
}

// ==============================================================================
// Span Collection for Dashboard
// ==============================================================================

/// Simplified span data for Dashboard Timeline visualization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpanData {
    /// W3C trace ID (hex string)
    pub trace_id: String,
    /// W3C span ID (hex string)
    pub span_id: String,
    /// Parent span ID if exists (hex string)
    pub parent_span_id: Option<String>,
    /// Span name (e.g., "sensor.emit", "bridge.forward")
    pub name: String,
    /// Start time (Unix timestamp in nanoseconds)
    pub start_time: u64,
    /// Duration in nanoseconds
    pub duration: u64,
    /// Span attributes (agent_id, topic, correlation_id, etc.)
    pub attributes: HashMap<String, String>,
    /// Span status: "ok", "error", "unset"
    pub status: String,
    /// Error message if status is "error"
    pub error_message: Option<String>,
}

impl SpanData {
    /// Convert OpenTelemetry SpanData to Dashboard SpanData
    fn from_otel(span: &OtelSpanData) -> Self {
        let trace_id = format!("{:032x}", span.span_context.trace_id());
        let span_id = format!("{:016x}", span.span_context.span_id());

        // Check if parent span ID is valid (non-zero)
        let parent_span_id = {
            let parent_bytes = span.parent_span_id.to_bytes();
            let is_zero = parent_bytes.iter().all(|&b| b == 0);
            if !is_zero {
                Some(format!("{:016x}", span.parent_span_id))
            } else {
                None
            }
        };

        // Extract attributes
        let mut attributes = HashMap::new();
        for kv in &span.attributes {
            attributes.insert(kv.key.to_string(), kv.value.to_string());
        }

        // Determine status
        let (status, error_message) = match &span.status {
            opentelemetry::trace::Status::Error { description } => {
                ("error".to_string(), Some(description.to_string()))
            }
            opentelemetry::trace::Status::Ok => ("ok".to_string(), None),
            opentelemetry::trace::Status::Unset => ("unset".to_string(), None),
        };

        // Calculate timestamps
        let start_time = span
            .start_time
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;

        let end_time = span
            .end_time
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;

        let duration = end_time.saturating_sub(start_time);

        SpanData {
            trace_id,
            span_id,
            parent_span_id,
            name: span.name.to_string(),
            start_time,
            duration,
            attributes,
            status,
            error_message,
        }
    }
}

/// SpanCollector implements OpenTelemetry SpanProcessor to collect spans for Dashboard
///
/// Maintains a ring buffer of recent spans (default: 10,000) and provides
/// query APIs for Dashboard Timeline visualization.
#[derive(Debug)]
pub struct SpanCollector {
    /// Ring buffer of collected spans
    spans: Arc<RwLock<VecDeque<SpanData>>>,
    /// Maximum buffer size
    max_size: usize,
    /// Index by trace_id for fast lookup
    trace_index: Arc<RwLock<HashMap<String, Vec<usize>>>>,
}

impl SpanCollector {
    /// Create a new SpanCollector with default buffer size (10,000)
    pub fn new() -> Self {
        Self::with_capacity(10_000)
    }

    /// Create a new SpanCollector with custom buffer size
    pub fn with_capacity(max_size: usize) -> Self {
        Self {
            spans: Arc::new(RwLock::new(VecDeque::with_capacity(max_size))),
            max_size,
            trace_index: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get recent spans (up to `limit`, newest first)
    pub async fn get_recent(&self, limit: usize) -> Vec<SpanData> {
        let spans = self.spans.read().await;
        spans.iter().rev().take(limit).cloned().collect()
    }

    /// Get all spans for a specific trace_id
    pub async fn get_trace(&self, trace_id: &str) -> Vec<SpanData> {
        let index = self.trace_index.read().await;
        if let Some(indices) = index.get(trace_id) {
            let spans = self.spans.read().await;
            indices
                .iter()
                .filter_map(|&i| spans.get(i).cloned())
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Get total number of collected spans
    pub async fn count(&self) -> usize {
        self.spans.read().await.len()
    }

    /// Clear all collected spans
    pub async fn clear(&self) {
        self.spans.write().await.clear();
        self.trace_index.write().await.clear();
    }

    /// Internal: Add a span to the buffer
    async fn add_span(&self, span_data: SpanData) {
        let mut spans = self.spans.write().await;
        let mut trace_index = self.trace_index.write().await;

        // Enforce max size (ring buffer behavior)
        if spans.len() >= self.max_size {
            if let Some(removed) = spans.pop_front() {
                // Remove from trace index
                if let Some(indices) = trace_index.get_mut(&removed.trace_id) {
                    indices.retain(|&i| i != 0);
                    if indices.is_empty() {
                        trace_index.remove(&removed.trace_id);
                    }
                }
                // Shift all indices down by 1
                for indices in trace_index.values_mut() {
                    for idx in indices.iter_mut() {
                        *idx = idx.saturating_sub(1);
                    }
                }
            }
        }

        // Add new span
        let index = spans.len();
        let trace_id = span_data.trace_id.clone();
        spans.push_back(span_data);

        // Update trace index
        trace_index
            .entry(trace_id)
            .or_insert_with(Vec::new)
            .push(index);
    }
}

impl Default for SpanCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl SpanProcessor for SpanCollector {
    fn on_start(&self, _span: &mut opentelemetry_sdk::trace::Span, _cx: &opentelemetry::Context) {
        // We only collect on span end
    }

    fn on_end(&self, span: OtelSpanData) {
        let span_data = SpanData::from_otel(&span);
        let collector = self.clone();

        // Spawn async task to add span (SpanProcessor is sync but we need async)
        tokio::spawn(async move {
            collector.add_span(span_data).await;
        });
    }

    fn force_flush(&self) -> opentelemetry::trace::TraceResult<()> {
        // No-op: spans are already in memory
        Ok(())
    }

    fn shutdown(&mut self) -> opentelemetry::trace::TraceResult<()> {
        // No-op: spans will be dropped with the collector
        Ok(())
    }
}

// Make SpanCollector cloneable for sharing across threads
impl Clone for SpanCollector {
    fn clone(&self) -> Self {
        Self {
            spans: Arc::clone(&self.spans),
            max_size: self.max_size,
            trace_index: Arc::clone(&self.trace_index),
        }
    }
}
