use super::client::McpClient;
use super::types::McpTool;
use crate::tools::{Tool, ToolError, ToolResult};
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;

/// Adapts an MCP tool to the unified Tool trait
pub struct McpToolAdapter {
    client: Arc<McpClient>,
    tool: McpTool,
    server_name: String,
}

impl McpToolAdapter {
    pub fn new(client: Arc<McpClient>, tool: McpTool, server_name: String) -> Self {
        Self {
            client,
            tool,
            server_name,
        }
    }
}

#[async_trait]
impl Tool for McpToolAdapter {
    fn name(&self) -> String {
        // Format: server_name:tool_name
        format!("{}:{}", self.server_name, self.tool.name)
    }

    fn description(&self) -> String {
        self.tool.description.clone().unwrap_or_default()
    }

    fn parameters(&self) -> Value {
        self.tool.input_schema.clone()
    }

    async fn call(&self, arguments: Value) -> ToolResult<Value> {
        // Convert serde_json::Value arguments to the format expected by McpClient
        // The McpClient::call_tool expects arguments as a HashMap or Value

        let result = self
            .client
            .call_tool(&self.tool.name, Some(arguments))
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("MCP call failed: {}", e)))?;

        // Convert McpToolResult to serde_json::Value
        // Usually we want to return the content list
        serde_json::to_value(result.content)
            .map_err(|e| ToolError::Internal(format!("Failed to serialize result: {}", e)))
    }
}
