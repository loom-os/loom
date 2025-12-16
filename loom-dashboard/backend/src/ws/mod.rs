//! WebSocket module
//!
//! Provides WebSocket support for the dashboard backend.

pub mod bridge;
pub mod connection;
pub mod handler;
pub mod message;
pub mod router;

pub use bridge::EventBusBridge;
pub use connection::{Connection, ConnectionManager, SendError};
pub use handler::websocket_handler;
pub use message::WsMessage;
pub use router::ChatRouter;
