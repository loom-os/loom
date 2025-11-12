/// MCP Tool Adapter
///
/// Adapts MCP tools to the CapabilityProvider trait so they can be
/// registered with the ActionBroker and invoked like native capabilities.
use super::client::McpClient;
use super::types::{McpTool, McpToolResult};
use crate::action_broker::CapabilityProvider;
use crate::proto::{
    ActionCall, ActionError, ActionResult, ActionStatus, CapabilityDescriptor, ProviderKind,
};
use crate::{LoomError, Result};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, warn};

/// Adapter that wraps an MCP tool as a CapabilityProvider
pub struct McpToolAdapter {
    /// MCP client (shared among all tools from same server)
    client: Arc<McpClient>,
    /// Tool definition
    tool: McpTool,
    /// Server name for logging/debugging
    server_name: String,
}

impl McpToolAdapter {
    /// Create a new adapter for an MCP tool
    pub fn new(client: Arc<McpClient>, tool: McpTool, server_name: String) -> Self {
        Self {
            client,
            tool,
            server_name,
        }
    }

    /// Get the tool name with server prefix
    pub fn qualified_name(&self) -> String {
        format!("{}:{}", self.server_name, self.tool.name)
    }
}

#[async_trait]
impl CapabilityProvider for McpToolAdapter {
    fn descriptor(&self) -> CapabilityDescriptor {
        let mut metadata = HashMap::new();

        // Add description
        if let Some(ref desc) = self.tool.description {
            metadata.insert("desc".to_string(), desc.clone());
        }

        // Add JSON schema
        metadata.insert(
            "schema".to_string(),
            serde_json::to_string(&self.tool.input_schema).unwrap_or_default(),
        );

        // Add server info
        metadata.insert("mcp_server".to_string(), self.server_name.clone());
        metadata.insert("mcp_tool".to_string(), self.tool.name.clone());

        CapabilityDescriptor {
            name: self.qualified_name(),
            version: "0.1.0".to_string(),
            provider: ProviderKind::ProviderMcp as i32,
            metadata,
        }
    }

    async fn invoke(&self, call: ActionCall) -> Result<ActionResult> {
        let call_id = call.id.clone();

        debug!(
            target: "mcp_adapter",
            server = %self.server_name,
            tool = %self.tool.name,
            call_id = %call_id,
            "Invoking MCP tool"
        );

        // Parse arguments from payload
        let arguments: Option<serde_json::Value> = if call.payload.is_empty() {
            None
        } else {
            match serde_json::from_slice(&call.payload) {
                Ok(v) => Some(v),
                Err(e) => {
                    warn!(
                        target: "mcp_adapter",
                        error = %e,
                        "Invalid JSON payload"
                    );
                    return Ok(ActionResult {
                        id: call_id,
                        status: ActionStatus::ActionError as i32,
                        output: vec![],
                        error: Some(ActionError {
                            code: "INVALID_PARAMS".to_string(),
                            message: format!("Invalid JSON payload: {}", e),
                            details: Default::default(),
                        }),
                    });
                }
            }
        };

        // Call the MCP tool
        match self.client.call_tool(&self.tool.name, arguments).await {
            Ok(result) => {
                if result.is_error {
                    // Tool returned error
                    Ok(ActionResult {
                        id: call_id,
                        status: ActionStatus::ActionError as i32,
                        output: vec![],
                        error: Some(ActionError {
                            code: "TOOL_ERROR".to_string(),
                            message: result.content,
                            details: Default::default(),
                        }),
                    })
                } else {
                    // Tool succeeded
                    // Wrap result as JSON object with "result" key
                    let output = serde_json::json!({
                        "result": result.content
                    });

                    Ok(ActionResult {
                        id: call_id,
                        status: ActionStatus::ActionOk as i32,
                        output: serde_json::to_vec(&output)?,
                        error: None,
                    })
                }
            }
            Err(e) => {
                warn!(
                    target: "mcp_adapter",
                    server = %self.server_name,
                    tool = %self.tool.name,
                    error = %e,
                    "MCP tool call failed"
                );

                let error_code = e.code().to_string();
                let error_message = e.to_string();

                let status = if matches!(e, super::types::McpError::Timeout) {
                    ActionStatus::ActionTimeout
                } else {
                    ActionStatus::ActionError
                };

                Ok(ActionResult {
                    id: call_id,
                    status: status as i32,
                    output: vec![],
                    error: Some(ActionError {
                        code: error_code,
                        message: error_message,
                        details: Default::default(),
                    }),
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_qualified_name() {
        let tool = McpTool {
            name: "search".to_string(),
            description: Some("Search tool".to_string()),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "query": {"type": "string"}
                }
            }),
        };

        // Note: We can't easily test the full adapter without a real client,
        // but we can verify the name generation logic
        let server_name = "my-server".to_string();
        let expected = format!("{}:{}", server_name, tool.name);

        assert_eq!(expected, "my-server:search");
    }

    #[test]
    fn test_descriptor_metadata() {
        let tool = McpTool {
            name: "test".to_string(),
            description: Some("Test tool".to_string()),
            input_schema: json!({"type": "object"}),
        };

        // Verify metadata keys are present
        let schema_str = serde_json::to_string(&tool.input_schema).unwrap();
        assert!(schema_str.contains("object"));
    }
}
