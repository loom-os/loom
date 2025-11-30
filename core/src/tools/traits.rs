use super::error::ToolResult;
use async_trait::async_trait;
use serde_json::Value;

/// The core trait for all tools (Native & MCP)
#[async_trait]
pub trait Tool: Send + Sync {
    /// The unique name of the tool (e.g., "filesystem:read_file")
    fn name(&self) -> String;

    /// A human-readable description of what the tool does
    fn description(&self) -> String;

    /// The JSON Schema for the tool's arguments
    fn parameters(&self) -> Value;

    /// Execute the tool with the given arguments
    async fn call(&self, arguments: Value) -> ToolResult<Value>;
}
