pub mod adapter;
pub mod client;
pub mod manager;
pub mod types;

pub use adapter::McpToolAdapter;
pub use client::{McpClient, McpTransport};
pub use manager::McpManager;
pub use types::{
    McpError, McpTool, McpToolCall, McpToolResult, DEFAULT_PROTOCOL_VERSION,
    SUPPORTED_PROTOCOL_VERSIONS,
};
