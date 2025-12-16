//! Dashboard Configuration Module
//!
//! Manages configuration for the Loom Dashboard server, including:
//! - Server bind address and port
//! - CORS settings
//! - WebSocket configuration
//! - Frontend serving options
//!
//! Configuration can be loaded from:
//! - Environment variables (highest priority)
//! - Configuration file (TOML)
//! - Default values (lowest priority)

use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::path::PathBuf;

/// Dashboard server configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DashboardConfig {
    /// Server bind address and port
    pub server: ServerConfig,

    /// WebSocket configuration
    pub websocket: WebSocketConfig,

    /// CORS configuration
    pub cors: CorsConfig,

    /// Frontend configuration
    pub frontend: FrontendConfig,
}
impl DashboardConfig {
    /// Load configuration from environment variables
    pub fn from_env() -> Self {
        let mut config = Self::default();

        // Server configuration
        if let Ok(host) = std::env::var("LOOM_DASHBOARD_HOST") {
            config.server.host = host;
        }
        if let Ok(port) = std::env::var("LOOM_DASHBOARD_PORT") {
            if let Ok(port) = port.parse() {
                config.server.port = port;
            }
        }

        // WebSocket configuration
        if let Ok(max_conn) = std::env::var("LOOM_DASHBOARD_WS_MAX_CONNECTIONS") {
            if let Ok(max_conn) = max_conn.parse() {
                config.websocket.max_connections = max_conn;
            }
        }

        // CORS configuration
        if let Ok(enabled) = std::env::var("LOOM_DASHBOARD_CORS_ENABLED") {
            config.cors.enabled = enabled == "true" || enabled == "1";
        }
        if let Ok(origins) = std::env::var("LOOM_DASHBOARD_CORS_ORIGINS") {
            config.cors.allowed_origins =
                origins.split(',').map(|s| s.trim().to_string()).collect();
        }

        config
    }

    /// Get the socket address to bind to
    pub fn bind_address(&self) -> SocketAddr {
        format!("{}:{}", self.server.host, self.server.port)
            .parse()
            .expect("Invalid bind address")
    }

    /// Check if development mode is enabled
    pub fn is_dev_mode(&self) -> bool {
        std::env::var("LOOM_DASHBOARD_DEV")
            .ok()
            .map(|v| v == "true" || v == "1")
            .unwrap_or(false)
    }
}

/// Server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Bind host
    pub host: String,

    /// Bind port
    pub port: u16,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 3030,
        }
    }
}

/// WebSocket configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketConfig {
    /// Maximum number of concurrent WebSocket connections
    pub max_connections: usize,

    /// Ping interval in seconds
    pub ping_interval_secs: u64,

    /// Connection timeout in seconds
    pub timeout_secs: u64,

    /// Maximum message size in bytes
    pub max_message_size: usize,
}

impl Default for WebSocketConfig {
    fn default() -> Self {
        Self {
            max_connections: 1000,
            ping_interval_secs: 30,
            timeout_secs: 300,                  // 5 minutes
            max_message_size: 10 * 1024 * 1024, // 10MB
        }
    }
}

/// CORS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorsConfig {
    /// Whether CORS is enabled
    pub enabled: bool,

    /// Allowed origins (empty means all)
    pub allowed_origins: Vec<String>,

    /// Whether credentials are allowed
    pub allow_credentials: bool,
}

impl Default for CorsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            allowed_origins: vec![],
            allow_credentials: false,
        }
    }
}

/// Frontend configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrontendConfig {
    /// Path to frontend static files (if not embedded)
    pub static_path: Option<PathBuf>,

    /// Whether to serve frontend
    pub serve_frontend: bool,
}

impl Default for FrontendConfig {
    fn default() -> Self {
        Self {
            static_path: None,
            serve_frontend: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn test_from_env() {
        std::env::set_var("LOOM_DASHBOARD_HOST", "0.0.0.0");
        std::env::set_var("LOOM_DASHBOARD_PORT", "8080");
        std::env::set_var("LOOM_DASHBOARD_WS_MAX_CONNECTIONS", "500");

        let config = DashboardConfig::from_env();
        assert_eq!(config.server.host, "0.0.0.0");
        assert_eq!(config.server.port, 8080);
        assert_eq!(config.websocket.max_connections, 500);

        // Clean up
        std::env::remove_var("LOOM_DASHBOARD_HOST");
        std::env::remove_var("LOOM_DASHBOARD_PORT");
        std::env::remove_var("LOOM_DASHBOARD_WS_MAX_CONNECTIONS");
    }
}
