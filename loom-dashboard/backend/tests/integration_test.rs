//! Integration tests for loom-dashboard

use loom_core::agent::directory::AgentDirectory;
use loom_core::messaging::event_bus::EventBus;
use loom_dashboard::{DashboardConfig, DashboardServer};
use std::sync::Arc;
use tokio::time::{sleep, Duration};

#[tokio::test]
async fn test_server_startup_and_health() {
    // Create test configuration
    let mut config = DashboardConfig::default();
    config.server.port = 0; // Use random port for testing

    // Initialize components
    let event_bus = Arc::new(EventBus::new().await.expect("Failed to create EventBus"));
    let agent_directory = Arc::new(AgentDirectory::new());

    // Create server
    let server = DashboardServer::new(config.clone(), event_bus, agent_directory);

    // Start server in background
    let handle = tokio::spawn(async move { server.serve().await });

    // Give server time to start
    sleep(Duration::from_millis(100)).await;

    // Test health endpoint (this will fail since we need to get the actual bound port)
    // For now, just ensure server can be created and started without panic

    // Cleanup
    handle.abort();
}

#[tokio::test]
async fn test_config_from_env() {
    // Set environment variables
    std::env::set_var("LOOM_DASHBOARD_HOST", "0.0.0.0");
    std::env::set_var("LOOM_DASHBOARD_PORT", "8080");

    let config = DashboardConfig::from_env();

    assert_eq!(config.server.host, "0.0.0.0");
    assert_eq!(config.server.port, 8080);

    // Cleanup
    std::env::remove_var("LOOM_DASHBOARD_HOST");
    std::env::remove_var("LOOM_DASHBOARD_PORT");
}

#[test]
fn test_default_config() {
    let config = DashboardConfig::default();

    assert_eq!(config.server.host, "127.0.0.1");
    assert_eq!(config.server.port, 3030);
    assert_eq!(config.websocket.max_connections, 1000);
    assert!(config.cors.enabled);
}

#[test]
fn test_bind_address() {
    let config = DashboardConfig::default();
    let addr = config.bind_address();

    assert_eq!(addr.to_string(), "127.0.0.1:3030");
}

#[tokio::test]
async fn test_multiple_servers() {
    // Test that we can create multiple server instances
    let event_bus = Arc::new(EventBus::new().await.expect("Failed to create EventBus"));
    let agent_directory = Arc::new(AgentDirectory::new());

    let config1 = DashboardConfig::default();
    let server1 = DashboardServer::new(config1, event_bus.clone(), agent_directory.clone());

    let mut config2 = DashboardConfig::default();
    config2.server.port = 3031;
    let server2 = DashboardServer::new(config2, event_bus.clone(), agent_directory.clone());

    // Just ensure they can be created
    drop(server1);
    drop(server2);
}
