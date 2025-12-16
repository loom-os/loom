use std::net::SocketAddr;

use loom_bridge::start_server;
use loom_core::Loom;
use loom_dashboard::{DashboardConfig, DashboardServer};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load .env file if present (from current directory or parent directories)
    match dotenvy::dotenv() {
        Ok(path) => {
            eprintln!("[loom-bridge] Loaded .env from: {:?}", path);
        }
        Err(e) => {
            eprintln!("[loom-bridge] Note: .env not loaded: {}", e);
        }
    }

    // Debug: print BRAVE_API_KEY status
    match std::env::var("BRAVE_API_KEY") {
        Ok(key) => eprintln!(
            "[loom-bridge] BRAVE_API_KEY loaded: {}...",
            &key[..key.len().min(10)]
        ),
        Err(_) => eprintln!("[loom-bridge] WARNING: BRAVE_API_KEY not set!"),
    }

    // Initialize OpenTelemetry for distributed tracing with SpanCollector
    let span_collector = match loom_core::telemetry::init_telemetry() {
        Ok(collector) => {
            tracing::info!("OpenTelemetry initialized for bridge server");
            collector
        }
        Err(e) => {
            tracing::warn!("Failed to initialize telemetry: {}", e);
            // Create a default SpanCollector even if OTLP fails
            loom_core::SpanCollector::new()
        }
    };

    let mut loom = Loom::new().await?;
    loom.start().await?;

    let bridge_addr: SocketAddr = std::env::var("LOOM_BRIDGE_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:50051".into())
        .parse()?;

    // Load dashboard configuration
    let dashboard_config = DashboardConfig::from_env();

    // Start dashboard server in background (shares EventBus & AgentDirectory with bridge)
    let dashboard = DashboardServer::new(
        dashboard_config.clone(),
        loom.event_bus.clone(),
        loom.agent_directory.clone(),
    );

    tracing::info!(
        "Starting Dashboard on http://{}:{}",
        dashboard_config.server.host,
        dashboard_config.server.port
    );

    let dashboard_task = tokio::spawn(async move {
        if let Err(e) = dashboard.serve().await {
            tracing::error!("Dashboard server error: {}", e);
        }
    });

    // Start bridge server (this will block)
    let bridge_result = start_server(
        loom.event_bus.clone(),
        loom.tool_registry.clone(),
        loom.agent_directory.clone(),
        bridge_addr,
    )
    .await;

    // If bridge exits, abort dashboard
    dashboard_task.abort();

    bridge_result.map_err(|e| e.into())
}
