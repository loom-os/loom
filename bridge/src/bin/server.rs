use std::net::SocketAddr;
use std::sync::Arc;
use tracing_subscriber::fmt;

use loom_bridge::start_server_with_dashboard;
use loom_core::dashboard::{DashboardConfig, DashboardServer, EventBroadcaster, FlowTracker};
use loom_core::Loom;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    fmt().compact().init();

    // Initialize OpenTelemetry for distributed tracing
    if let Err(e) = loom_core::telemetry::init_telemetry() {
        tracing::warn!("Failed to initialize telemetry: {}", e);
    } else {
        tracing::info!("OpenTelemetry initialized for bridge server");
    }

    let mut loom = Loom::new().await?;

    // Check if Dashboard is enabled
    let dashboard_enabled = DashboardConfig::enabled();
    let broadcaster_opt;
    let flow_tracker_opt;

    let dashboard_handle = if dashboard_enabled {
        let config = DashboardConfig::from_env();

        // Create dashboard broadcaster and flow tracker
        let broadcaster = EventBroadcaster::new(1000);
        let flow_tracker = Arc::new(FlowTracker::new());

        // Connect EventBus to Dashboard (need to modify before wrapping in Arc)
        // Dashboard broadcaster needs to be set before loom.start()
        let event_bus_ptr = Arc::as_ptr(&loom.event_bus) as *mut loom_core::EventBus;
        unsafe {
            (*event_bus_ptr).set_dashboard_broadcaster(broadcaster.clone());
            (*event_bus_ptr).set_flow_tracker(flow_tracker.clone());
        }

        // Get agent directory
        let agent_directory = loom.agent_directory.clone();

        // Create dashboard server
        let dashboard = DashboardServer::new(config.clone(), broadcaster.clone(), agent_directory)
            .with_flow_tracker(flow_tracker.clone());

        tracing::info!(
            "Dashboard enabled at http://{}:{}",
            config.host,
            config.port
        );

        // Store for later use
        broadcaster_opt = Some(broadcaster);
        flow_tracker_opt = Some(flow_tracker);

        // Spawn dashboard server
        Some(tokio::spawn(async move {
            if let Err(e) = dashboard.serve().await {
                tracing::error!("Dashboard error: {}", e);
            }
        }))
    } else {
        broadcaster_opt = None;
        flow_tracker_opt = None;
        None
    };

    loom.start().await?;

    let addr: SocketAddr = std::env::var("LOOM_BRIDGE_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:50051".into())
        .parse()?;

    // Start bridge server with dashboard integration (this will block)
    let server_result = start_server_with_dashboard(
        addr,
        loom.event_bus.clone(),
        loom.action_broker.clone(),
        loom.agent_directory.clone(),
        broadcaster_opt,
        flow_tracker_opt,
    )
    .await;

    // If dashboard was started, wait for it to finish
    if let Some(handle) = dashboard_handle {
        let _ = handle.await;
    }

    server_result.map_err(|e| e.into())
}
