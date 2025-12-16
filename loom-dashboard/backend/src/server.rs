//! Dashboard Server Module
//!
//! Provides the main HTTP/WebSocket server for the Loom Dashboard.
//!
//! The server exposes:
//! - REST API endpoints for configuration and data
//! - WebSocket endpoint for real-time bidirectional communication
//! - Static file serving for the frontend (optional)
//!
//! # Architecture
//!
//! ```text
//! Browser ←→ WebSocket ←→ Dashboard Server ←→ EventBus/Bridge
//!                              (axum)
//! ```
//!
//! # Example
//!
//! ```no_run
//! use loom_dashboard::{DashboardConfig, DashboardServer};
//! use loom_core::messaging::event_bus::EventBus;
//! use loom_core::agent::directory::AgentDirectory;
//! use std::sync::Arc;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let config = DashboardConfig::from_env();
//!     let event_bus = Arc::new(EventBus::new().await?);
//!     let agent_directory = Arc::new(AgentDirectory::new());
//!
//!     let server = DashboardServer::new(config, event_bus, agent_directory);
//!     server.serve().await?;
//!
//!     Ok(())
//! }
//! ```

use crate::config::DashboardConfig;
use crate::ws::{websocket_handler, ChatRouter, ConnectionManager, EventBusBridge};
use axum::{extract::State, response::{Html, IntoResponse}, routing::get, Router};
use loom_core::agent::directory::AgentDirectory;
use loom_core::messaging::event_bus::EventBus;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;
use tracing::{info, warn};

/// Dashboard server state shared across handlers
pub struct DashboardState {
    /// Configuration
    pub config: DashboardConfig,

    /// Event bus for agent communication
    pub event_bus: Arc<EventBus>,

    /// Agent directory for topology information
    pub agent_directory: Arc<AgentDirectory>,

    /// WebSocket connection manager
    pub connection_manager: Arc<ConnectionManager>,

    /// Chat router for handling chat requests
    pub chat_router: Arc<ChatRouter>,
}

/// Main Dashboard server
pub struct DashboardServer {
    config: DashboardConfig,
    event_bus: Arc<EventBus>,
    agent_directory: Arc<AgentDirectory>,
}

impl DashboardServer {
    /// Create a new Dashboard server
    ///
    /// # Arguments
    ///
    /// * `config` - Server configuration
    /// * `event_bus` - Event bus for agent communication
    /// * `agent_directory` - Agent directory for topology
    pub fn new(
        config: DashboardConfig,
        event_bus: Arc<EventBus>,
        agent_directory: Arc<AgentDirectory>,
    ) -> Self {
        Self {
            config,
            event_bus,
            agent_directory,
        }
    }

    /// Start the Dashboard server
    ///
    /// This will bind to the configured address and start serving requests.
    /// The function will block until the server is stopped.
    ///
    /// # Errors
    ///
    /// Returns an error if the server fails to bind or start.
    pub async fn serve(self) -> anyhow::Result<()> {
        let addr = self.config.bind_address();
        info!(
            target: "loom_dashboard",
            addr = %addr,
            "Starting Dashboard server"
        );

        let connection_manager = Arc::new(ConnectionManager::new());
        let chat_router = Arc::new(ChatRouter::new(
            self.event_bus.clone(),
            self.agent_directory.clone(),
            connection_manager.clone(),
        ));

        let state = Arc::new(DashboardState {
            config: self.config.clone(),
            event_bus: self.event_bus.clone(),
            agent_directory: self.agent_directory.clone(),
            connection_manager: connection_manager.clone(),
            chat_router,
        });

        // Start EventBus bridge
        let bridge = Arc::new(EventBusBridge::new(
            self.event_bus.clone(),
            connection_manager,
        ));
        bridge.clone().start().await?;
        info!(target: "loom_dashboard", "EventBus bridge started");

        // Build router
        let app = self.build_router(state);

        // Start server
        let listener = tokio::net::TcpListener::bind(&addr).await?;
        info!(
            target: "loom_dashboard",
            url = %format!("http://{}", addr),
            "Dashboard server ready"
        );

        axum::serve(listener, app).await?;

        Ok(())
    }

    /// Build the application router
    fn build_router(&self, state: Arc<DashboardState>) -> Router {
        let mut app = Router::new()
            // Health check endpoint
            .route("/health", get(health_handler))
            // WebSocket endpoint for real-time bidirectional communication
            .route("/ws", get(websocket_handler))
            // API routes
            .route("/api/config", get(config_handler))
            .route("/api/agents", get(agents_handler))
            .with_state(state.clone());

        // Serve frontend static files if enabled
        if state.config.frontend.serve_frontend {
            let frontend_path = state.config.frontend.static_path
                .clone()
                .unwrap_or_else(|| {
                    // Default to ../frontend/dist relative to cargo workspace
                    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                        .parent()
                        .unwrap()
                        .join("frontend")
                        .join("dist")
                });

            if frontend_path.exists() {
                info!(
                    target: "loom_dashboard",
                    path = ?frontend_path,
                    "Serving frontend from path"
                );
                app = app.nest_service("/", ServeDir::new(frontend_path));
            } else {
                warn!(
                    target: "loom_dashboard",
                    path = ?frontend_path,
                    "Frontend path not found, serving fallback page"
                );
                app = app.route("/", get(fallback_handler));
            }
        } else {
            app = app.route("/", get(fallback_handler));
        }

        // Add CORS layer if enabled
        if self.config.cors.enabled {
            let cors = if self.config.cors.allowed_origins.is_empty() {
                // Allow all origins in dev mode
                CorsLayer::new()
                    .allow_origin(Any)
                    .allow_methods(Any)
                    .allow_headers(Any)
            } else {
                // Specific origins in production
                let origins: Vec<_> = self
                    .config
                    .cors
                    .allowed_origins
                    .iter()
                    .filter_map(|origin| origin.parse().ok())
                    .collect();
                CorsLayer::new()
                    .allow_origin(origins)
                    .allow_methods(Any)
                    .allow_headers(Any)
            };
            app = app.layer(cors);
        }

        app
    }
}

/// Fallback page handler when frontend is not available
async fn fallback_handler() -> impl IntoResponse {
    Html(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Loom Dashboard</title>
    <style>
        * { margin: 0; padding: 0; box-sizing: border-box; }
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            min-height: 100vh;
            display: flex;
            align-items: center;
            justify-content: center;
            color: #333;
        }
        .container {
            background: white;
            padding: 3rem;
            border-radius: 20px;
            box-shadow: 0 20px 60px rgba(0,0,0,0.3);
            max-width: 600px;
            text-align: center;
        }
        h1 {
            font-size: 2.5rem;
            margin-bottom: 1rem;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            -webkit-background-clip: text;
            -webkit-text-fill-color: transparent;
            background-clip: text;
        }
        .subtitle {
            color: #666;
            font-size: 1.1rem;
            margin-bottom: 2rem;
        }
        .status {
            display: inline-block;
            padding: 0.5rem 1rem;
            background: #10b981;
            color: white;
            border-radius: 20px;
            font-weight: 600;
            margin-bottom: 2rem;
        }
        .endpoints {
            text-align: left;
            background: #f9fafb;
            padding: 1.5rem;
            border-radius: 10px;
            margin-top: 2rem;
        }
        .endpoints h3 {
            margin-bottom: 1rem;
            color: #667eea;
        }
        .endpoint {
            display: flex;
            margin: 0.75rem 0;
            font-family: 'Courier New', monospace;
            font-size: 0.9rem;
        }
        .method {
            background: #667eea;
            color: white;
            padding: 0.25rem 0.5rem;
            border-radius: 4px;
            margin-right: 0.5rem;
            font-weight: 600;
            min-width: 50px;
            text-align: center;
        }
        .path {
            color: #333;
        }
        .info {
            margin-top: 2rem;
            padding-top: 2rem;
            border-top: 1px solid #e5e7eb;
            color: #666;
            font-size: 0.9rem;
        }
        a {
            color: #667eea;
            text-decoration: none;
        }
        a:hover {
            text-decoration: underline;
        }
    </style>
</head>
<body>
    <div class="container">
        <h1>🧠 Loom Dashboard</h1>
        <div class="subtitle">AI Agent Runtime Observability</div>
        <div class="status">✓ Server Running</div>

        <div class="endpoints">
            <h3>Available Endpoints:</h3>
            <div class="endpoint">
                <span class="method">GET</span>
                <span class="path">/health</span>
            </div>
            <div class="endpoint">
                <span class="method">GET</span>
                <span class="path">/api/config</span>
            </div>
            <div class="endpoint">
                <span class="method">GET</span>
                <span class="path">/ws</span>
            </div>
        </div>

        <div class="info">
            <p>Frontend: Coming soon! 🚀</p>
            <p style="margin-top: 0.5rem;">
                <a href="/health">Check Health</a> |
                <a href="/api/config">View Config</a>
            </p>
        </div>
    </div>
</body>
</html>"#,
    )
}

/// Health check handler
async fn health_handler() -> impl IntoResponse {
    serde_json::json!({
        "status": "ok",
        "service": "loom-dashboard",
        "version": env!("CARGO_PKG_VERSION"),
    })
    .to_string()
}

/// Config handler - returns current configuration
async fn config_handler(State(state): State<Arc<DashboardState>>) -> impl IntoResponse {
    serde_json::to_string(&state.config)
        .map(|json| (axum::http::StatusCode::OK, json))
        .unwrap_or_else(|e| {
            warn!(target: "loom_dashboard", error = %e, "Failed to serialize config");
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to serialize config".to_string(),
            )
        })
}

/// Available agents endpoint
///
/// Returns a list of all registered agents with their capabilities and status.
/// Clients can use this to discover which agents are available for interaction.
///
/// # Response Format
///
/// ```json
/// {
///   "agents": [
///     {
///       "agent_id": "chat-assistant",
///       "status": "active",
///       "subscribed_topics": ["chat.input"],
///       "capabilities": ["web_search", "file_write"],
///       "metadata": {"type": "cognitive"},
///       "last_heartbeat": 1734393600000
///     }
///   ],
///   "count": 1,
///   "timestamp": "2024-12-17T00:00:00Z"
/// }
/// ```
async fn agents_handler(State(state): State<Arc<DashboardState>>) -> impl IntoResponse {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct AvailableAgent {
        agent_id: String,
        status: String,
        subscribed_topics: Vec<String>,
        capabilities: Vec<String>,
        metadata: std::collections::HashMap<String, String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        last_heartbeat: Option<i64>,
    }

    #[derive(Debug, Serialize)]
    struct AgentsResponse {
        agents: Vec<AvailableAgent>,
        count: usize,
        timestamp: String,
    }

    // Get all agents from directory
    let all_agents = state.agent_directory.all();

    // Convert to response format
    let agents: Vec<AvailableAgent> = all_agents
        .into_iter()
        .map(|info| {
            let status = match info.status {
                loom_core::agent::directory::AgentStatus::Active => "active",
                loom_core::agent::directory::AgentStatus::Idle => "idle",
                loom_core::agent::directory::AgentStatus::Inactive => "inactive",
                loom_core::agent::directory::AgentStatus::Disconnected => "disconnected",
            };

            AvailableAgent {
                agent_id: info.agent_id,
                status: status.to_string(),
                subscribed_topics: info.subscribed_topics,
                capabilities: info.capabilities,
                metadata: info.metadata,
                last_heartbeat: info.last_heartbeat,
            }
        })
        .collect();

    let count = agents.len();

    let response = AgentsResponse {
        agents,
        count,
        timestamp: chrono::Utc::now().to_rfc3339(),
    };

    axum::Json(response)
}

#[cfg(test)]
mod tests {
    use super::*;
    use loom_core::messaging::event_bus::EventBus;

    #[tokio::test]
    async fn test_fallback_handler() {
        let response = fallback_handler().await.into_response();
        assert_eq!(response.status(), axum::http::StatusCode::OK);
    }

    #[tokio::test]
    async fn test_health_endpoint() {
        let response = health_handler().await.into_response();
        assert_eq!(response.status(), axum::http::StatusCode::OK);
    }

    #[tokio::test]
    async fn test_server_creation() {
        let config = DashboardConfig::default();
        let event_bus = Arc::new(EventBus::new().await.unwrap());
        let agent_directory = Arc::new(AgentDirectory::new());

        let server = DashboardServer::new(config, event_bus, agent_directory);
        // Just ensure it doesn't panic
        drop(server);
    }

    #[tokio::test]
    async fn test_agents_endpoint_empty() {
        use axum::body::Body;
        use axum::http::Request;
        use tower::ServiceExt;

        let config = DashboardConfig::default();
        let event_bus = Arc::new(EventBus::new().await.unwrap());
        let agent_directory = Arc::new(AgentDirectory::new());
        let connection_manager = Arc::new(crate::ws::ConnectionManager::new());
        let chat_router = Arc::new(crate::ws::ChatRouter::new(
            event_bus.clone(),
            agent_directory.clone(),
            connection_manager.clone(),
        ));

        let state = Arc::new(DashboardState {
            config,
            event_bus,
            agent_directory,
            connection_manager,
            chat_router,
        });

        let app = Router::new()
            .route("/api/agents", get(agents_handler))
            .with_state(state);

        let request = Request::builder()
            .uri("/api/agents")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), axum::http::StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();

        // Should return empty array
        assert!(body_str.contains("\"agents\":[]"));
        assert!(body_str.contains("\"count\":0"));
    }

    #[tokio::test]
    async fn test_agents_endpoint_with_agents() {
        use axum::body::Body;
        use axum::http::Request;
        use tower::ServiceExt;
        use loom_core::agent::directory::AgentInfo;

        let config = DashboardConfig::default();
        let event_bus = Arc::new(EventBus::new().await.unwrap());
        let agent_directory = Arc::new(AgentDirectory::new());
        let connection_manager = Arc::new(crate::ws::ConnectionManager::new());

        // Register test agents
        agent_directory.register_agent(AgentInfo {
            agent_id: "chat-assistant".to_string(),
            subscribed_topics: vec!["chat.input".to_string()],
            capabilities: vec!["web_search".to_string(), "file_write".to_string()],
            metadata: {
                let mut m = std::collections::HashMap::new();
                m.insert("type".to_string(), "cognitive".to_string());
                m
            },
            last_heartbeat: Some(chrono::Utc::now().timestamp_millis()),
            status: loom_core::agent::directory::AgentStatus::Active,
        });

        agent_directory.register_agent(AgentInfo {
            agent_id: "worker-agent".to_string(),
            subscribed_topics: vec!["tasks.background".to_string()],
            capabilities: vec!["process_data".to_string()],
            metadata: std::collections::HashMap::new(),
            last_heartbeat: Some(chrono::Utc::now().timestamp_millis()),
            status: loom_core::agent::directory::AgentStatus::Idle,
        });

        let chat_router = Arc::new(crate::ws::ChatRouter::new(
            event_bus.clone(),
            agent_directory.clone(),
            connection_manager.clone(),
        ));

        let state = Arc::new(DashboardState {
            config,
            event_bus,
            agent_directory,
            connection_manager,
            chat_router,
        });

        let app = Router::new()
            .route("/api/agents", get(agents_handler))
            .with_state(state);

        let request = Request::builder()
            .uri("/api/agents")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), axum::http::StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();

        // Should return 2 agents
        assert!(body_str.contains("\"count\":2"));
        assert!(body_str.contains("\"chat-assistant\""));
        assert!(body_str.contains("\"worker-agent\""));
        assert!(body_str.contains("\"chat.input\""));
        assert!(body_str.contains("\"web_search\""));
        assert!(body_str.contains("\"active\""));
        assert!(body_str.contains("\"idle\""));
    }
}
