// Dashboard module - Real-time event flow visualization
//
// Provides a simple HTTP server with SSE for streaming events to a web UI.

mod api;
mod event_stream;
mod flow_tracker;
mod static_assets;
mod topology;

pub use api::DashboardServer;
pub use event_stream::{DashboardEvent, DashboardEventType, EventBroadcaster};
pub use flow_tracker::{EventFlow, FlowGraph, FlowNode, FlowTracker, NodeType};

/// Dashboard configuration
#[derive(Clone, Debug)]
pub struct DashboardConfig {
    pub port: u16,
    pub host: String,
}

impl Default for DashboardConfig {
    fn default() -> Self {
        Self {
            port: 3030,
            host: "127.0.0.1".to_string(),
        }
    }
}

impl DashboardConfig {
    pub fn from_env() -> Self {
        Self {
            port: std::env::var("LOOM_DASHBOARD_PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(3030),
            host: std::env::var("LOOM_DASHBOARD_HOST").unwrap_or_else(|_| "127.0.0.1".to_string()),
        }
    }

    pub fn enabled() -> bool {
        std::env::var("LOOM_DASHBOARD")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(false)
    }
}
