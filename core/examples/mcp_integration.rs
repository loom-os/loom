/// Example: Using MCP tools in Loom
///
/// This example demonstrates how to:
/// 1. Configure and connect to MCP servers
/// 2. Automatically discover tools from MCP servers
/// 3. Use MCP tools in agents via the ActionBroker
///
/// Prerequisites:
/// - Node.js installed (for running MCP servers)
/// - Optional: API keys for services (Brave Search, GitHub, etc.)
///
/// Run with:
/// ```bash
/// cargo run --example mcp_integration
/// ```
use loom_core::{mcp::types::McpServerConfig, Loom, QoSLevel};
use serde_json::json;
use std::collections::HashMap;
use tracing::{info, warn};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Set OpenTelemetry environment variables (optional - comment out to disable)
    std::env::set_var("OTEL_EXPORTER_OTLP_ENDPOINT", "http://localhost:4317");
    std::env::set_var("OTEL_SERVICE_NAME", "loom-mcp-example");

    // Initialize telemetry BEFORE creating Loom
    // This will set up both logging and OpenTelemetry tracing
    if std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT").is_ok() {
        loom_core::telemetry::init_telemetry()
            .map_err(|e| format!("Failed to initialize telemetry: {}", e))?;
    } else {
        // Fallback: just initialize basic tracing if OpenTelemetry is disabled
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::INFO)
            .with_target(true)
            .init();
    }

    info!("Starting Loom with MCP integration example");

    // Initialize Loom
    let mut loom = Loom::new().await?;

    // Configure MCP servers
    let mcp_configs = vec![
        // Example 1: Filesystem MCP server (if available)
        // Requires: npx -y @modelcontextprotocol/server-filesystem
        McpServerConfig {
            name: "filesystem".to_string(),
            command: "npx".to_string(),
            args: vec![
                "-y".to_string(),
                "@modelcontextprotocol/server-filesystem".to_string(),
                "/tmp".to_string(), // Access /tmp directory
            ],
            env: None,
            cwd: None,
            protocol_version: None, // Use latest supported version
        },
        // Example 2: Brave Search (requires API key)
        // Uncomment and add your API key to use:
        /*
        McpServerConfig {
            name: "brave-search".to_string(),
            command: "npx".to_string(),
            args: vec![
                "-y".to_string(),
                "@modelcontextprotocol/server-brave-search".to_string(),
            ],
            env: Some({
                let mut env = HashMap::new();
                env.insert("BRAVE_API_KEY".to_string(), "your-api-key-here".to_string());
                env
            }),
            cwd: None,
            protocol_version: None,
        },
        */
    ];

    // Connect to MCP servers
    info!("Connecting to MCP servers...");
    for config in mcp_configs {
        match loom.mcp_manager.add_server(config.clone()).await {
            Ok(_) => {
                info!("Successfully connected to MCP server: {}", config.name);
            }
            Err(e) => {
                warn!("Failed to connect to MCP server {}: {}", config.name, e);
                warn!("Make sure the MCP server is available. Skipping...");
            }
        }
    }

    // List connected servers
    let servers = loom.mcp_manager.list_servers().await;
    info!("Connected MCP servers: {:?}", servers);

    // List all available capabilities (including MCP tools)
    let capabilities = loom.action_broker.list_capabilities();
    info!("Available capabilities:");
    for cap in &capabilities {
        if cap.provider == loom_core::proto::ProviderKind::ProviderMcp as i32 {
            let desc = cap.metadata.get("desc").cloned().unwrap_or_default();
            info!("  [MCP] {} - {}", cap.name, desc);
        }
    }

    // Start Loom
    loom.start().await?;

    // Example: Use an MCP tool through ActionBroker
    if servers.contains(&"filesystem".to_string()) {
        info!("\nDemonstrating MCP tool invocation...");

        // Create a test file
        let test_file = "/tmp/loom_mcp_test.txt";
        std::fs::write(test_file, "Hello from Loom MCP integration!")?;
        info!("Created test file: {}", test_file);

        // Invoke the filesystem:read_file tool
        let call = loom_core::proto::ActionCall {
            id: "test-mcp-call-1".to_string(),
            capability: "filesystem:read_file".to_string(),
            version: String::new(),
            payload: serde_json::to_vec(&json!({
                "path": test_file
            }))?,
            headers: HashMap::new(),
            timeout_ms: 5000,
            correlation_id: "example-correlation".to_string(),
            qos: QoSLevel::QosRealtime as i32,
        };

        match loom.action_broker.invoke(call).await {
            Ok(result) => {
                if result.status == loom_core::proto::ActionStatus::ActionOk as i32 {
                    let output: serde_json::Value = serde_json::from_slice(&result.output)?;
                    info!("MCP tool result: {}", output);
                } else if let Some(error) = result.error {
                    warn!("MCP tool error: {} - {}", error.code, error.message);
                }
            }
            Err(e) => {
                warn!("Failed to invoke MCP tool: {}", e);
            }
        }

        // Clean up
        std::fs::remove_file(test_file)?;
    }

    // Example: Publish an event that an agent could handle
    // In a real scenario, agents would subscribe to topics and use MCP tools
    let event = loom_core::proto::Event {
        id: "event-1".to_string(),
        r#type: "agent.task".to_string(),
        timestamp_ms: chrono::Utc::now().timestamp_millis(),
        source: "mcp-example".to_string(),
        metadata: HashMap::new(),
        payload: serde_json::to_vec(&json!({
            "action": "read_file",
            "path": "/tmp/example.txt"
        }))?,
        confidence: 1.0,
        tags: vec!["example".to_string()],
        priority: 5,
    };

    loom.event_bus.publish("agent.task", event).await?;

    info!("\nMCP integration example completed successfully!");
    info!("In a real application:");
    info!("  1. Agents would subscribe to relevant topics");
    info!("  2. Agents would use ctx.tool() to invoke MCP tools");
    info!("  3. MCP tools would be seamlessly integrated with native capabilities");

    // Keep running for a bit
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Shutdown
    loom.shutdown().await?;

    Ok(())
}
