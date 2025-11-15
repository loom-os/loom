// Dashboard HTTP API server
//
// Provides REST endpoints and SSE streaming for the Dashboard UI

use crate::dashboard::event_stream::EventBroadcaster;
use crate::dashboard::flow_tracker::FlowTracker;
use crate::dashboard::topology::TopologyBuilder;
use crate::dashboard::DashboardConfig;
use crate::directory::AgentDirectory;
use crate::telemetry::SpanCollector;
use axum::{
    extract::{Path, Query, State},
    http::{header, HeaderMap, StatusCode},
    response::{
        sse::{Event, KeepAlive},
        Html, IntoResponse, Sse,
    },
    routing::get,
    Router,
};
use serde::Deserialize;
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
    flow_tracker: Arc<FlowTracker>,
    span_collector: SpanCollector,
}

/// Dashboard HTTP server
pub struct DashboardServer {
    config: DashboardConfig,
    broadcaster: EventBroadcaster,
    agent_directory: Arc<AgentDirectory>,
    flow_tracker: Arc<FlowTracker>,
    span_collector: SpanCollector,
}

impl DashboardServer {
    pub fn new(
        config: DashboardConfig,
        broadcaster: EventBroadcaster,
        agent_directory: Arc<AgentDirectory>,
        span_collector: SpanCollector,
    ) -> Self {
        let flow_tracker = Arc::new(FlowTracker::new());
        Self {
            config,
            broadcaster,
            agent_directory,
            flow_tracker,
            span_collector,
        }
    }

    pub fn with_flow_tracker(mut self, flow_tracker: Arc<FlowTracker>) -> Self {
        self.flow_tracker = flow_tracker;
        self
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
            flow_tracker: self.flow_tracker.clone(),
            span_collector: self.span_collector.clone(),
        };

        // Start cleanup task for flow tracker
        let flow_tracker_clone = state.flow_tracker.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
            loop {
                interval.tick().await;
                flow_tracker_clone.cleanup().await;
            }
        });

        // Build router
        let app = Router::new()
            .route("/", get(index_handler))
            .route("/static/*asset", get(static_asset_handler))
            .route("/api/events/stream", get(event_stream_handler))
            .route("/api/topology", get(topology_handler))
            .route("/api/flow", get(flow_handler))
            .route("/api/metrics", get(metrics_handler))
            .route("/api/spans/recent", get(spans_recent_handler))
            .route("/api/traces/:trace_id", get(trace_handler))
            .route("/api/spans/stream", get(spans_stream_handler))
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
const FALLBACK_INDEX: &str = r#"<!DOCTYPE html><html><head><meta charset="utf-8"><title>Loom Dashboard</title></head><body><h1>Loom Dashboard assets not found</h1><p>Please run <code>npm run build</code> inside <code>core/src/dashboard/frontend</code> to generate the static assets.</p></body></html>"#;

async fn index_handler() -> Html<&'static str> {
    let html = crate::dashboard::static_assets::get_text("index.html").unwrap_or(FALLBACK_INDEX);
    Html(html)
}

async fn static_asset_handler(Path(asset): Path<String>) -> impl IntoResponse {
    match crate::dashboard::static_assets::get(asset.as_str()) {
        Some(asset) => {
            let mut headers = HeaderMap::new();
            if let Ok(value) = header::HeaderValue::from_str(asset.content_type.as_ref()) {
                headers.insert(header::CONTENT_TYPE, value);
            }
            (StatusCode::OK, headers, asset.body).into_response()
        }
        None => {
            let headers = HeaderMap::new();
            (StatusCode::NOT_FOUND, headers, b"Not found".as_slice()).into_response()
        }
    }
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

/// Get current flow graph snapshot
async fn flow_handler(
    State(state): State<DashboardState>,
) -> Result<impl IntoResponse, StatusCode> {
    let graph = state.flow_tracker.get_graph().await;
    match serde_json::to_string(&graph) {
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

/// Query parameters for spans/recent endpoint
#[derive(Deserialize)]
struct SpansRecentQuery {
    #[serde(default = "default_limit")]
    limit: usize,
}

fn default_limit() -> usize {
    100
}

/// Get recent spans for Timeline view
/// Query params: ?limit=100 (default: 100, max: 1000)
async fn spans_recent_handler(
    State(state): State<DashboardState>,
    Query(query): Query<SpansRecentQuery>,
) -> Result<impl IntoResponse, StatusCode> {
    let limit = query.limit.min(1000); // Cap at 1000
    let spans = state.span_collector.get_recent(limit).await;

    match serde_json::to_string(&spans) {
        Ok(json) => Ok((StatusCode::OK, json)),
        Err(e) => {
            warn!(target: "dashboard", error = %e, "Failed to serialize spans");
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Get all spans for a specific trace
async fn trace_handler(
    State(state): State<DashboardState>,
    Path(trace_id): Path<String>,
) -> Result<impl IntoResponse, StatusCode> {
    let spans = state.span_collector.get_trace(&trace_id).await;

    match serde_json::to_string(&spans) {
        Ok(json) => Ok((StatusCode::OK, json)),
        Err(e) => {
            warn!(target: "dashboard", error = %e, "Failed to serialize trace spans");
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// SSE endpoint for real-time span updates
/// This allows the Timeline view to update as new spans arrive
async fn spans_stream_handler(
    State(state): State<DashboardState>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>> {
    info!(target: "dashboard", "New spans SSE client connected");

    // Create a channel to send span updates
    let (tx, rx) = tokio::sync::mpsc::channel(100);
    let span_collector = state.span_collector.clone();

    // Spawn a task to poll for new spans
    tokio::spawn(async move {
        let mut last_count = 0usize;
        let mut interval = tokio::time::interval(std::time::Duration::from_millis(500));

        loop {
            interval.tick().await;

            let current_count = span_collector.count().await;
            if current_count > last_count {
                // Get only the new spans
                let new_span_count = current_count - last_count;
                let recent = span_collector.get_recent(new_span_count).await;

                // Send as a batch
                match serde_json::to_string(&recent) {
                    Ok(json) => {
                        if tx.send(json).await.is_err() {
                            // Client disconnected
                            break;
                        }
                    }
                    Err(e) => {
                        warn!(target: "dashboard", error = %e, "Failed to serialize spans");
                    }
                }

                last_count = current_count;
            }
        }
    });

    // Convert channel receiver to stream
    let stream = tokio_stream::wrappers::ReceiverStream::new(rx)
        .map(|json| Ok(Event::default().event("spans").data(json)));

    Sse::new(stream).keep_alive(KeepAlive::default())
}
