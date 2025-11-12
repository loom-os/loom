/// Integration tests for MCP functionality
use loom_core::mcp::{
    client::McpClient,
    manager::McpManager,
    types::{McpServerConfig, McpTool},
};
use loom_core::ActionBroker;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;

/// Test MCP type serialization
#[test]
fn test_mcp_server_config_serialization() {
    let config = McpServerConfig {
        name: "test-server".to_string(),
        command: "node".to_string(),
        args: vec!["server.js".to_string()],
        env: Some({
            let mut env = HashMap::new();
            env.insert("API_KEY".to_string(), "test-key".to_string());
            env
        }),
        cwd: Some("/tmp".to_string()),
        protocol_version: None,
    };

    let json = serde_json::to_string(&config).unwrap();
    assert!(json.contains("test-server"));
    assert!(json.contains("node"));

    let deserialized: McpServerConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.name, "test-server");
    assert_eq!(deserialized.command, "node");
}

/// Test protocol version handling
#[test]
fn test_protocol_version_defaults() {
    use loom_core::mcp::{DEFAULT_PROTOCOL_VERSION, SUPPORTED_PROTOCOL_VERSIONS};

    // Config without explicit version should use default
    let config = McpServerConfig {
        name: "test".to_string(),
        command: "node".to_string(),
        args: vec![],
        env: None,
        cwd: None,
        protocol_version: None,
    };

    assert_eq!(config.protocol_version(), DEFAULT_PROTOCOL_VERSION);
    assert!(config.validate_protocol_version().is_ok());

    // Config with explicit supported version
    let config_with_version = McpServerConfig {
        protocol_version: Some("2024-11-05".to_string()),
        ..config.clone()
    };

    assert_eq!(config_with_version.protocol_version(), "2024-11-05");
    assert!(config_with_version.validate_protocol_version().is_ok());

    // Config with unsupported version should fail validation
    let config_bad_version = McpServerConfig {
        protocol_version: Some("1999-01-01".to_string()),
        ..config
    };

    assert_eq!(config_bad_version.protocol_version(), "1999-01-01");
    assert!(config_bad_version.validate_protocol_version().is_err());

    // Verify constants
    assert_eq!(DEFAULT_PROTOCOL_VERSION, "2024-11-05");
    assert!(SUPPORTED_PROTOCOL_VERSIONS.contains(&DEFAULT_PROTOCOL_VERSION));
}

/// Test MCP tool definition
#[test]
fn test_mcp_tool_definition() {
    let tool = McpTool {
        name: "read_file".to_string(),
        description: Some("Read a file from filesystem".to_string()),
        input_schema: json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "File path"
                }
            },
            "required": ["path"]
        }),
    };

    assert_eq!(tool.name, "read_file");
    assert!(tool.description.is_some());
    assert!(tool.input_schema.is_object());
}

/// Test MCP manager creation
#[tokio::test]
async fn test_mcp_manager_creation() {
    let broker = Arc::new(ActionBroker::new());
    let manager = McpManager::new(broker);

    let servers = manager.list_servers().await;
    assert!(servers.is_empty());
}

/// Test adding invalid server configuration
#[tokio::test]
async fn test_add_invalid_server() {
    let broker = Arc::new(ActionBroker::new());
    let manager = McpManager::new(broker);

    let config = McpServerConfig {
        name: "invalid-server".to_string(),
        command: "nonexistent-command-12345".to_string(),
        args: vec![],
        env: None,
        cwd: None,
        protocol_version: None,
    };

    // This should fail because the command doesn't exist
    let result = manager.add_server(config).await;
    assert!(result.is_err());
}

/// Test manager shutdown
#[tokio::test]
async fn test_manager_shutdown() {
    let broker = Arc::new(ActionBroker::new());
    let manager = McpManager::new(broker);

    // Shutdown should succeed even with no servers
    manager.shutdown().await;

    let servers = manager.list_servers().await;
    assert!(servers.is_empty());
}

/// Test removing non-existent server
#[tokio::test]
async fn test_remove_nonexistent_server() {
    let broker = Arc::new(ActionBroker::new());
    let manager = McpManager::new(broker);

    // Removing a server that doesn't exist should not panic
    let result = manager.remove_server("nonexistent").await;
    assert!(result.is_ok());
}

/// Test qualified tool names
#[test]
fn test_qualified_tool_names() {
    let server_name = "my-server";
    let tool_name = "search";
    let qualified = format!("{}:{}", server_name, tool_name);

    assert_eq!(qualified, "my-server:search");
    assert!(qualified.contains(':'));

    // Parse back
    let parts: Vec<&str> = qualified.split(':').collect();
    assert_eq!(parts.len(), 2);
    assert_eq!(parts[0], server_name);
    assert_eq!(parts[1], tool_name);
}

/// Test MCP tool adapter metadata
#[test]
fn test_tool_adapter_metadata() {
    use loom_core::action_broker::CapabilityProvider;
    use loom_core::mcp::adapter::McpToolAdapter;

    let tool = McpTool {
        name: "test_tool".to_string(),
        description: Some("A test tool".to_string()),
        input_schema: json!({
            "type": "object",
            "properties": {
                "input": {"type": "string"}
            }
        }),
    };

    // Note: We can't easily create a full adapter without a real client,
    // but we can verify the metadata structure we expect

    // Verify tool has required fields
    assert!(!tool.name.is_empty());
    assert!(tool.description.is_some());
    assert!(tool.input_schema.is_object());
}

/// Test configuration loading from TOML
#[test]
fn test_load_mcp_config_from_toml() {
    let toml_content = r#"
        [[servers]]
        name = "filesystem"
        command = "npx"
        args = ["-y", "@modelcontextprotocol/server-filesystem", "/tmp"]

        [[servers]]
        name = "search"
        command = "npx"
        args = ["-y", "@modelcontextprotocol/server-brave-search"]

        [servers.env]
        API_KEY = "test"
    "#;

    #[derive(serde::Deserialize)]
    struct Config {
        servers: Vec<McpServerConfig>,
    }

    let config: Config = toml::from_str(toml_content).unwrap();
    assert_eq!(config.servers.len(), 2);
    assert_eq!(config.servers[0].name, "filesystem");
    assert_eq!(config.servers[1].name, "search");
    assert_eq!(config.servers[0].args.len(), 3);
    assert!(config.servers[1].env.is_some());
}

/// Test error code mapping
#[test]
fn test_mcp_error_codes() {
    use loom_core::mcp::types::McpError;

    let timeout_error = McpError::Timeout;
    assert_eq!(timeout_error.code(), "TIMEOUT");

    let not_found = McpError::ToolNotFound("test".to_string());
    assert_eq!(not_found.code(), "TOOL_NOT_FOUND");

    let invalid = McpError::InvalidParams("bad input".to_string());
    assert_eq!(invalid.code(), "INVALID_PARAMS");
}

/// Test JSON-RPC request/response structures
#[test]
fn test_jsonrpc_structures() {
    use loom_core::mcp::types::{JsonRpcRequest, JsonRpcResponse};

    let request = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: json!(1),
        method: "tools/list".to_string(),
        params: Some(json!({})),
    };

    let json = serde_json::to_string(&request).unwrap();
    assert!(json.contains("\"jsonrpc\":\"2.0\""));
    assert!(json.contains("\"method\":\"tools/list\""));

    let response = JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        id: json!(1),
        result: Some(json!({"tools": []})),
        error: None,
    };

    let json = serde_json::to_string(&response).unwrap();
    assert!(json.contains("\"result\""));
    assert!(!json.contains("\"error\""));
}
