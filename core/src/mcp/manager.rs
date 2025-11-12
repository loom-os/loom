/// MCP Manager
///
/// Manages multiple MCP server connections and registers their tools
/// with the ActionBroker. Handles lifecycle (connect/disconnect/reconnect).
use super::adapter::McpToolAdapter;
use super::client::McpClient;
use super::types::{McpError, McpServerConfig};
use crate::action_broker::ActionBroker;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// MCP Manager
///
/// Responsible for:
/// - Managing connections to multiple MCP servers
/// - Discovering tools from connected servers
/// - Registering tools as capabilities with ActionBroker
/// - Handling reconnection on failures
pub struct McpManager {
    /// Active MCP clients: server_name -> client
    clients: Arc<RwLock<HashMap<String, Arc<McpClient>>>>,
    /// Reference to ActionBroker for registering capabilities
    broker: Arc<ActionBroker>,
}

impl McpManager {
    /// Create a new MCP manager
    pub fn new(broker: Arc<ActionBroker>) -> Self {
        Self {
            clients: Arc::new(RwLock::new(HashMap::new())),
            broker,
        }
    }

    /// Add and connect to an MCP server
    pub async fn add_server(&self, config: McpServerConfig) -> Result<(), McpError> {
        let server_name = config.name.clone();

        info!(
            target: "mcp_manager",
            server = %server_name,
            protocol_version = %config.protocol_version(),
            "Adding MCP server"
        );

        // Validate protocol version
        if let Err(e) = config.validate_protocol_version() {
            error!(
                target: "mcp_manager",
                server = %server_name,
                error = %e,
                "Invalid protocol version"
            );
            return Err(McpError::Protocol(e));
        }

        // Check if already connected
        {
            let clients = self.clients.read().await;
            if clients.contains_key(&server_name) {
                warn!(
                    target: "mcp_manager",
                    server = %server_name,
                    "Server already connected"
                );
                return Ok(());
            }
        }

        // Create and connect client
        let client = Arc::new(McpClient::new(config));
        client.connect().await?;

        // Discover and register tools
        match self.register_tools(&client, &server_name).await {
            Ok(count) => {
                info!(
                    target: "mcp_manager",
                    server = %server_name,
                    tool_count = count,
                    "MCP server added successfully"
                );
            }
            Err(e) => {
                error!(
                    target: "mcp_manager",
                    server = %server_name,
                    error = %e,
                    "Failed to register tools"
                );
                let _ = client.disconnect().await;
                return Err(e);
            }
        }

        // Store client
        self.clients.write().await.insert(server_name, client);

        Ok(())
    }

    /// Remove and disconnect from an MCP server
    pub async fn remove_server(&self, server_name: &str) -> Result<(), McpError> {
        info!(
            target: "mcp_manager",
            server = %server_name,
            "Removing MCP server"
        );

        let client = self.clients.write().await.remove(server_name);

        if let Some(client) = client {
            client.disconnect().await?;
            info!(
                target: "mcp_manager",
                server = %server_name,
                "MCP server removed"
            );
        } else {
            warn!(
                target: "mcp_manager",
                server = %server_name,
                "Server not found"
            );
        }

        Ok(())
    }

    /// List connected servers
    pub async fn list_servers(&self) -> Vec<String> {
        self.clients.read().await.keys().cloned().collect()
    }

    /// Register tools from a client with the ActionBroker
    async fn register_tools(
        &self,
        client: &Arc<McpClient>,
        server_name: &str,
    ) -> Result<usize, McpError> {
        debug!(
            target: "mcp_manager",
            server = %server_name,
            "Discovering tools"
        );

        let tools = client.list_tools().await?;

        debug!(
            target: "mcp_manager",
            server = %server_name,
            count = tools.len(),
            "Discovered tools"
        );

        let mut registered = 0;
        for tool in tools {
            let adapter = Arc::new(McpToolAdapter::new(
                Arc::clone(client),
                tool.clone(),
                server_name.to_string(),
            ));

            self.broker.register_provider(adapter);
            registered += 1;

            debug!(
                target: "mcp_manager",
                server = %server_name,
                tool = %tool.name,
                "Registered MCP tool"
            );
        }

        Ok(registered)
    }

    /// Reconnect to a server (useful for error recovery)
    pub async fn reconnect_server(&self, server_name: &str) -> Result<(), McpError> {
        info!(
            target: "mcp_manager",
            server = %server_name,
            "Reconnecting to MCP server"
        );

        // Get current client
        let client = {
            let clients = self.clients.read().await;
            clients
                .get(server_name)
                .cloned()
                .ok_or_else(|| McpError::Transport(format!("Server not found: {}", server_name)))?
        };

        // Disconnect
        let _ = client.disconnect().await;

        // Reconnect
        client.connect().await?;

        // Re-register tools
        self.register_tools(&client, server_name).await?;

        info!(
            target: "mcp_manager",
            server = %server_name,
            "Reconnected to MCP server"
        );

        Ok(())
    }

    /// Get server info
    pub async fn server_info(&self, server_name: &str) -> Option<super::types::ServerInfo> {
        let clients = self.clients.read().await;
        if let Some(client) = clients.get(server_name) {
            client.server_info().await
        } else {
            None
        }
    }

    /// Disconnect all servers
    pub async fn shutdown(&self) {
        info!(target: "mcp_manager", "Shutting down MCP manager");

        let clients: Vec<_> = {
            let mut map = self.clients.write().await;
            map.drain().collect()
        };

        for (name, client) in clients {
            debug!(target: "mcp_manager", server = %name, "Disconnecting server");
            let _ = client.disconnect().await;
        }

        info!(target: "mcp_manager", "MCP manager shutdown complete");
    }
}

impl Drop for McpManager {
    fn drop(&mut self) {
        // Best-effort cleanup in sync context
        // Note: We can't await here, so we just clear the clients
        // Actual disconnection happens via the Drop implementation of McpClient
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action_broker::ActionBroker;

    #[tokio::test]
    async fn test_manager_creation() {
        let broker = Arc::new(ActionBroker::new());
        let manager = McpManager::new(broker);

        let servers = manager.list_servers().await;
        assert!(servers.is_empty());
    }

    #[tokio::test]
    async fn test_list_empty_servers() {
        let broker = Arc::new(ActionBroker::new());
        let manager = McpManager::new(broker);

        let servers = manager.list_servers().await;
        assert_eq!(servers.len(), 0);
    }
}
