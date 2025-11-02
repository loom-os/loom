// Telemetry and observability
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::info;

/// Performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metrics {
    pub total_events: u64,
    pub events_per_second: f64,
    pub avg_latency_ms: f64,
    pub p99_latency_ms: f64,
    pub cloud_fallback_rate: f64,
    pub error_rate: f64,
}

/// Metrics collector
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
