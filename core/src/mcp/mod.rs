/// Model Context Protocol (MCP) integration
///
/// This module provides MCP client functionality to connect to MCP servers,
/// discover tools, and invoke them through the ActionBroker.
///
/// MCP Protocol Spec: https://spec.modelcontextprotocol.io/specification/
///
/// Architecture:
/// - `client`: Low-level MCP client (stdio/SSE transport)
/// - `adapter`: Adapts MCP tools to CapabilityProvider trait
/// - `manager`: Manages multiple MCP server connections
/// - `types`: MCP protocol types (JSON-RPC 2.0 based)
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
