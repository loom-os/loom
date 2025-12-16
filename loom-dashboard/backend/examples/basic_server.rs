//! Basic Dashboard server example
//!
//! This example demonstrates how to start a basic Dashboard server.
//!
//! Usage:
//!     cargo run --example basic_server
//!
//! Environment variables:
//!     LOOM_DASHBOARD_HOST=127.0.0.1
//!     LOOM_DASHBOARD_PORT=3030
//!     LOOM_DASHBOARD_DEV=true

use loom_core::agent::directory::AgentDirectory;
use loom_core::messaging::event_bus::EventBus;
use loom_dashboard::{DashboardConfig, DashboardServer};
use std::sync::Arc;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "loom_dashboard=debug,info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting Loom Dashboard example");

    // Load configuration from environment
    let config = DashboardConfig::from_env();
    tracing::info!(?config, "Loaded configuration");

    // Initialize Loom Core components
    tracing::info!("Initializing EventBus...");
    let event_bus = Arc::new(EventBus::new().await?);

    tracing::info!("Initializing AgentDirectory...");
    let agent_directory = Arc::new(AgentDirectory::new());

    // Create and start Dashboard server
    tracing::info!(
        "Starting Dashboard server on {}:{}",
        config.server.host,
        config.server.port
    );

    let server = DashboardServer::new(config.clone(), event_bus, agent_directory);

    tracing::info!(
        "Dashboard ready at http://{}:{}",
        config.server.host,
        config.server.port
    );
    tracing::info!("Press Ctrl+C to stop");

    // Serve (blocks until shutdown)
    server.serve().await?;

    Ok(())
}
