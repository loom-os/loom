/// MCP Client implementation
///
/// Provides low-level communication with MCP servers via stdio transport.
/// Supports JSON-RPC 2.0 protocol with proper request/response correlation.
use super::types::*;
use crate::LoomError;
use serde_json::json;
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::sync::{mpsc, oneshot, Mutex};
use tokio::time::{timeout, Duration};
use tracing::{debug, error, info, warn};

/// MCP transport type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McpTransport {
    /// Standard input/output (most common)
    Stdio,
    /// Server-Sent Events (HTTP-based)
    Sse,
}

/// MCP client for communicating with a single MCP server
pub struct McpClient {
    /// Server configuration
    config: McpServerConfig,
    /// Child process handle
    process: Arc<Mutex<Option<Child>>>,
    /// Stdin writer
    stdin: Arc<Mutex<Option<ChildStdin>>>,
    /// Request ID counter
    request_id: Arc<AtomicU64>,
    /// Pending requests: request_id -> response channel
    pending: Arc<Mutex<HashMap<u64, oneshot::Sender<JsonRpcResponse>>>>,
    /// Server info after initialization
    server_info: Arc<Mutex<Option<ServerInfo>>>,
    /// Server capabilities
    capabilities: Arc<Mutex<Option<ServerCapabilities>>>,
}

impl McpClient {
    /// Create a new MCP client with configuration
    pub fn new(config: McpServerConfig) -> Self {
        Self {
            config,
            process: Arc::new(Mutex::new(None)),
            stdin: Arc::new(Mutex::new(None)),
            request_id: Arc::new(AtomicU64::new(1)),
            pending: Arc::new(Mutex::new(HashMap::new())),
            server_info: Arc::new(Mutex::new(None)),
            capabilities: Arc::new(Mutex::new(None)),
        }
    }

    /// Start the MCP server process and initialize connection
    pub async fn connect(&self) -> Result<(), McpError> {
        info!(
            target: "mcp_client",
            server = %self.config.name,
            command = %self.config.command,
            "Connecting to MCP server"
        );

        // Spawn process
        let mut cmd = Command::new(&self.config.command);
        cmd.args(&self.config.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        if let Some(ref env) = self.config.env {
            for (key, val) in env {
                cmd.env(key, val);
            }
        }

        if let Some(ref cwd) = self.config.cwd {
            cmd.current_dir(cwd);
        }

        let mut child = cmd.spawn().map_err(|e| {
            error!(target: "mcp_client", error = %e, "Failed to spawn MCP server process");
            McpError::Transport(format!("Failed to spawn process: {}", e))
        })?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| McpError::Transport("Failed to capture stdin".to_string()))?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| McpError::Transport("Failed to capture stdout".to_string()))?;

        *self.stdin.lock().await = Some(stdin);
        *self.process.lock().await = Some(child);

        // Start stdout reader task
        self.spawn_reader(stdout);

        // Send initialize request
        let init_result = self.initialize().await?;

        *self.server_info.lock().await = Some(init_result.server_info.clone());
        *self.capabilities.lock().await = Some(init_result.capabilities.clone());

        info!(
            target: "mcp_client",
            server = %self.config.name,
            server_name = %init_result.server_info.name,
            server_version = %init_result.server_info.version,
            "MCP server connected and initialized"
        );

        Ok(())
    }

    /// Disconnect from the MCP server
    pub async fn disconnect(&self) -> Result<(), McpError> {
        info!(target: "mcp_client", server = %self.config.name, "Disconnecting from MCP server");

        // Close stdin to signal shutdown
        if let Some(mut stdin) = self.stdin.lock().await.take() {
            let _ = stdin.shutdown().await;
        }

        // Kill process if still running
        if let Some(mut child) = self.process.lock().await.take() {
            let _ = child.kill().await;
            let _ = child.wait().await;
        }

        Ok(())
    }

    /// Send initialize request
    async fn initialize(&self) -> Result<InitializeResult, McpError> {
        let params = InitializeParams {
            protocol_version: self.config.protocol_version().to_string(),
            capabilities: ClientCapabilities {
                roots: None,
                sampling: None,
                experimental: None,
            },
            client_info: ClientInfo {
                name: "loom".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
        };

        let result = self.send_request("initialize", Some(json!(params))).await?;

        serde_json::from_value(result)
            .map_err(|e| McpError::Protocol(format!("Invalid initialize result: {}", e)))
    }

    /// List available tools
    pub async fn list_tools(&self) -> Result<Vec<McpTool>, McpError> {
        debug!(target: "mcp_client", server = %self.config.name, "Listing tools");

        let mut all_tools = Vec::new();
        let mut cursor: Option<String> = None;

        loop {
            let params = ListToolsParams { cursor };
            let result = self.send_request("tools/list", Some(json!(params))).await?;

            let list_result: ListToolsResult = serde_json::from_value(result)
                .map_err(|e| McpError::Protocol(format!("Invalid tools/list result: {}", e)))?;

            all_tools.extend(list_result.tools);

            if list_result.next_cursor.is_none() {
                break;
            }
            cursor = list_result.next_cursor;
        }

        debug!(
            target: "mcp_client",
            server = %self.config.name,
            count = all_tools.len(),
            "Listed tools"
        );

        Ok(all_tools)
    }

    /// Call a tool
    pub async fn call_tool(
        &self,
        name: &str,
        arguments: Option<serde_json::Value>,
    ) -> Result<McpToolResult, McpError> {
        debug!(target: "mcp_client", server = %self.config.name, tool = %name, "Calling tool");

        let params = CallToolParams {
            name: name.to_string(),
            arguments,
        };

        let result = self.send_request("tools/call", Some(json!(params))).await?;

        let call_result: McpToolCall = serde_json::from_value(result)
            .map_err(|e| McpError::Protocol(format!("Invalid tools/call result: {}", e)))?;

        // Combine all text content
        let mut content_parts = Vec::new();
        for item in &call_result.content {
            match item {
                ToolContent::Text { text } => content_parts.push(text.clone()),
                ToolContent::Image { .. } => {
                    content_parts.push("[image content]".to_string());
                }
                ToolContent::Resource { resource } => {
                    if let Some(ref text) = resource.text {
                        content_parts.push(text.clone());
                    } else {
                        content_parts.push(format!("[resource: {}]", resource.uri));
                    }
                }
            }
        }

        let content = content_parts.join("\n");
        let is_error = call_result.is_error.unwrap_or(false);

        if is_error {
            debug!(
                target: "mcp_client",
                server = %self.config.name,
                tool = %name,
                "Tool returned error"
            );
        }

        Ok(McpToolResult { content, is_error })
    }

    /// Send a JSON-RPC request and wait for response
    async fn send_request(
        &self,
        method: &str,
        params: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, McpError> {
        let id = self.request_id.fetch_add(1, Ordering::SeqCst);
        let (tx, rx) = oneshot::channel();

        // Register pending request
        self.pending.lock().await.insert(id, tx);

        // Build request
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: json!(id),
            method: method.to_string(),
            params,
        };

        // Serialize and send
        let mut line = serde_json::to_string(&request)?;
        line.push('\n');

        let mut stdin_guard = self.stdin.lock().await;
        let stdin = stdin_guard
            .as_mut()
            .ok_or_else(|| McpError::Transport("stdin not available".to_string()))?;

        stdin.write_all(line.as_bytes()).await.map_err(|e| {
            error!(target: "mcp_client", error = %e, "Failed to write request");
            McpError::Io(e)
        })?;

        stdin.flush().await.map_err(|e| {
            error!(target: "mcp_client", error = %e, "Failed to flush stdin");
            McpError::Io(e)
        })?;

        drop(stdin_guard);

        // Wait for response with timeout
        let response = timeout(Duration::from_secs(30), rx)
            .await
            .map_err(|_| {
                warn!(target: "mcp_client", method = %method, "Request timeout");
                McpError::Timeout
            })?
            .map_err(|_| McpError::Transport("Response channel closed".to_string()))?;

        // Check for error
        if let Some(error) = response.error {
            return Err(McpError::ServerError(format!(
                "{} (code: {})",
                error.message, error.code
            )));
        }

        response
            .result
            .ok_or_else(|| McpError::Protocol("Missing result in response".to_string()))
    }

    /// Spawn stdout reader task
    fn spawn_reader(&self, stdout: ChildStdout) {
        let pending = Arc::clone(&self.pending);
        let server_name = self.config.name.clone();

        tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();

            while let Ok(Some(line)) = lines.next_line().await {
                if line.trim().is_empty() {
                    continue;
                }

                match serde_json::from_str::<JsonRpcResponse>(&line) {
                    Ok(response) => {
                        if let Some(id) = response.id.as_u64() {
                            if let Some(tx) = pending.lock().await.remove(&id) {
                                let _ = tx.send(response);
                            } else {
                                warn!(
                                    target: "mcp_client",
                                    server = %server_name,
                                    id = id,
                                    "Received response for unknown request"
                                );
                            }
                        }
                    }
                    Err(e) => {
                        warn!(
                            target: "mcp_client",
                            server = %server_name,
                            error = %e,
                            line = %line,
                            "Failed to parse JSON-RPC response"
                        );
                    }
                }
            }

            debug!(target: "mcp_client", server = %server_name, "Stdout reader exited");
        });
    }

    /// Get server info
    pub async fn server_info(&self) -> Option<ServerInfo> {
        self.server_info.lock().await.clone()
    }

    /// Get server capabilities
    pub async fn capabilities(&self) -> Option<ServerCapabilities> {
        self.capabilities.lock().await.clone()
    }
}

impl Drop for McpClient {
    fn drop(&mut self) {
        // Best-effort cleanup
        // Note: We can't await here in sync context
        // The child process will be killed automatically when dropped
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_rpc_request_serialization() {
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: json!(1),
            method: "test".to_string(),
            params: Some(json!({"foo": "bar"})),
        };

        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"jsonrpc\":\"2.0\""));
        assert!(json.contains("\"method\":\"test\""));
    }

    #[test]
    fn test_mcp_error_codes() {
        assert_eq!(McpError::Timeout.code(), "TIMEOUT");
        assert_eq!(
            McpError::ToolNotFound("test".to_string()).code(),
            "TOOL_NOT_FOUND"
        );
        assert_eq!(
            McpError::InvalidParams("test".to_string()).code(),
            "INVALID_PARAMS"
        );
    }
}
