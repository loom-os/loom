// Dashboard HTTP API server
//
// Provides REST endpoints and SSE streaming for the Dashboard UI

use crate::dashboard::event_stream::EventBroadcaster;
use crate::dashboard::topology::TopologyBuilder;
use crate::dashboard::DashboardConfig;
use crate::directory::AgentDirectory;
use axum::{
    extract::State,
    http::StatusCode,
    response::{
        sse::{Event, KeepAlive},
        Html, IntoResponse, Sse,
    },
    routing::get,
    Router,
};
use std::convert::Infallible;
use std::sync::Arc;
use tokio_stream::{wrappers::BroadcastStream, StreamExt};
use tower_http::cors::{Any, CorsLayer};
use tracing::{info, warn};

/// Dashboard server state
#[derive(Clone)]
struct DashboardState {
    broadcaster: EventBroadcaster,
    topology_builder: Arc<TopologyBuilder>,
}

/// Dashboard HTTP server
pub struct DashboardServer {
    config: DashboardConfig,
    broadcaster: EventBroadcaster,
    agent_directory: Arc<AgentDirectory>,
}

impl DashboardServer {
    pub fn new(
        config: DashboardConfig,
        broadcaster: EventBroadcaster,
        agent_directory: Arc<AgentDirectory>,
    ) -> Self {
        Self {
            config,
            broadcaster,
            agent_directory,
        }
    }

    /// Start the Dashboard server
    pub async fn serve(self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let addr = format!("{}:{}", self.config.host, self.config.port);
        info!(
            target: "dashboard",
            addr = %addr,
            "Starting Dashboard server"
        );

        let state = DashboardState {
            broadcaster: self.broadcaster,
            topology_builder: Arc::new(TopologyBuilder::new(self.agent_directory)),
        };

        // Build router
        let app = Router::new()
            .route("/", get(index_handler))
            .route("/api/events/stream", get(event_stream_handler))
            .route("/api/topology", get(topology_handler))
            .route("/api/metrics", get(metrics_handler))
            .layer(
                CorsLayer::new()
                    .allow_origin(Any)
                    .allow_methods(Any)
                    .allow_headers(Any),
            )
            .with_state(state);

        // Start server
        let listener = tokio::net::TcpListener::bind(&addr).await?;
        info!(
            target: "dashboard",
            url = %format!("http://{}", addr),
            "Dashboard server ready"
        );

        axum::serve(listener, app).await?;

        Ok(())
    }
}

/// Serve the main HTML page
async fn index_handler() -> Html<&'static str> {
    Html(include_str!("static/index.html"))
}

/// SSE endpoint for real-time events
async fn event_stream_handler(
    State(state): State<DashboardState>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>> {
    info!(target: "dashboard", "New SSE client connected");

    let rx = state.broadcaster.subscribe();
    let stream = BroadcastStream::new(rx).filter_map(|result| match result {
        Ok(event) => {
            // Convert DashboardEvent to SSE Event
            match serde_json::to_string(&event) {
                Ok(json) => Some(Ok(Event::default().data(json))),
                Err(e) => {
                    warn!(target: "dashboard", error = %e, "Failed to serialize event");
                    None
                }
            }
        }
        Err(e) => {
            warn!(target: "dashboard", error = %e, "Broadcast error");
            None
        }
    });

    Sse::new(stream).keep_alive(KeepAlive::default())
}

/// Get current topology snapshot
async fn topology_handler(
    State(state): State<DashboardState>,
) -> Result<impl IntoResponse, StatusCode> {
    let snapshot = state.topology_builder.build_snapshot().await;
    match serde_json::to_string(&snapshot) {
        Ok(json) => Ok((StatusCode::OK, json)),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// Get current metrics snapshot
async fn metrics_handler() -> Result<impl IntoResponse, StatusCode> {
    // TODO: Integrate with OpenTelemetry metrics
    let metrics = serde_json::json!({
        "events_per_sec": 0,
        "active_agents": 0,
        "active_subscriptions": 0,
        "tool_invocations_per_sec": 0,
    });

    Ok((StatusCode::OK, metrics.to_string()))
}
