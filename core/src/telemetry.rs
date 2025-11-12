// Telemetry and observability with OpenTelemetry support
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::info;

use opentelemetry::KeyValue;
use opentelemetry_otlp::WithExportConfig;
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
/// use loom_core::telemetry::init_telemetry;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     // Initialize telemetry (reads from env vars)
///     init_telemetry()?;
///
///     // Your application code...
///
///     // Shutdown gracefully
///     shutdown_telemetry();
///     Ok(())
/// }
/// ```
pub fn init_telemetry() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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

    // Initialize tracer provider with OTLP exporter
    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint(otlp_endpoint.clone()),
        )
        .with_trace_config(
            opentelemetry_sdk::trace::config()
                .with_resource(resource.clone())
                .with_sampler(get_sampler_from_env()),
        )
        .install_batch(opentelemetry_sdk::runtime::Tokio)?;

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
        "OpenTelemetry initialized successfully"
    );

    Ok(())
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
