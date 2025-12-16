//! # Loom Dashboard
//!
//! Web-based UI for Loom agent runtime, providing:
//! - **Real-time observability**: Event streams, agent topology, metrics
//! - **Interactive chat**: Conversational interface with agents
//! - **Agent management**: Monitor and control running agents
//!
//! ## Architecture
//!
//! The Dashboard is an independent Rust crate that connects to Loom Core:
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                    Browser (React Frontend)                      │
//! └────────────────────────────┬────────────────────────────────────┘
//!                              │ WebSocket + REST
//!                              ▼
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                   loom-dashboard (Rust)                          │
//! │  - WebSocket handler                                             │
//! │  - REST API                                                      │
//! │  - Chat message routing                                          │
//! └────────────────────────────┬────────────────────────────────────┘
//!                              │ Direct access
//!                              ▼
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                      loom-core (Rust)                            │
//! │  - EventBus                                                      │
//! │  - AgentDirectory                                                │
//! │  - FlowTracker                                                   │
//! └─────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Usage
//!
//! ### Basic Server
//!
//! ```no_run
//! use loom_dashboard::{DashboardConfig, DashboardServer};
//! use loom_core::messaging::event_bus::EventBus;
//! use loom_core::agent::directory::AgentDirectory;
//! use std::sync::Arc;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     // Load configuration
//!     let config = DashboardConfig::from_env();
//!
//!     // Initialize core components
//!     let event_bus = Arc::new(EventBus::new().await?);
//!     let agent_directory = Arc::new(AgentDirectory::new());
//!
//!     // Create and start server
//!     let server = DashboardServer::new(config, event_bus, agent_directory);
//!     server.serve().await?;
//!
//!     Ok(())
//! }
//! ```
//!
//! ### Configuration
//!
//! Configure via environment variables:
//!
//! ```bash
//! # Server settings
//! export LOOM_DASHBOARD_HOST=0.0.0.0
//! export LOOM_DASHBOARD_PORT=3030
//!
//! # WebSocket settings
//! export LOOM_DASHBOARD_WS_MAX_CONNECTIONS=1000
//!
//! # CORS settings
//! export LOOM_DASHBOARD_CORS_ENABLED=true
//! export LOOM_DASHBOARD_CORS_ORIGINS=http://localhost:5173
//!
//! # Development mode
//! export LOOM_DASHBOARD_DEV=true
//! ```
//!
//! ## Features
//!
//! - `dev`: Enable development mode (relaxed CORS, debug endpoints)
//! - `embed-frontend`: Embed static frontend assets into binary
//!
//! ## Modules
//!
//! - [`config`]: Configuration management
//! - [`server`]: HTTP/WebSocket server implementation
//!
//! ## Development
//!
//! ### Frontend Development
//!
//! The frontend is a separate Vite + React project in `../frontend/`.
//!
//! For development with hot reload:
//!
//! ```bash
//! # Terminal 1: Start Vite dev server
//! cd frontend && npm run dev  # Runs on http://localhost:5173
//!
//! # Terminal 2: Start backend with CORS enabled
//! export LOOM_DASHBOARD_DEV=true
//! cargo run
//! ```
//!
//! The frontend will proxy API/WebSocket requests to the backend.
//!
//! ### Production Build
//!
//! ```bash
//! # Build frontend
//! cd frontend && npm run build
//!
//! # Build backend with embedded frontend
//! cargo build --release --features embed-frontend
//! ```
//!
//! ## WebSocket Protocol
//!
//! The WebSocket endpoint (`/ws`) uses JSON messages:
//!
//! ```json
//! // Client → Server: Chat request
//! {
//!   "type": "chat.request",
//!   "thread_id": "thread-123",
//!   "content": "Hello, agent!"
//! }
//!
//! // Server → Client: Chat chunk (streaming)
//! {
//!   "type": "chat.chunk",
//!   "thread_id": "thread-123",
//!   "content": "Hello! ",
//!   "content_type": "text"
//! }
//!
//! // Server → Client: Event update (observability)
//! {
//!   "type": "event.stream",
//!   "event_id": "evt-456",
//!   "topic": "chat.input",
//!   "sender": "user",
//!   "payload": "..."
//! }
//! ```
//!
//! See [`server`] module for full protocol details.

#![warn(missing_docs)]
#![warn(rustdoc::missing_crate_level_docs)]

pub mod config;
pub mod server;
pub mod ws;

// Re-exports for convenience
pub use config::DashboardConfig;
pub use server::DashboardServer;
pub use ws::{websocket_handler, ConnectionManager, EventBusBridge, WsMessage};

/// Current crate version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
    }
}
