# MCP (Model Context Protocol) Integration

This module provides complete MCP client support for Loom, enabling agents to access tools from any MCP-compatible server.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      Loom Core                              │
│                                                             │
│  ┌────────────────┐                                         │
│  │ ActionBroker   │                                         │
│  └────────┬───────┘                                         │
│           │                                                 │
│           │ registers tools                                 │
│           │                                                 │
│  ┌────────▼───────────────────────────────────┐             │
│  │              McpManager                    │             │
│  │  - Manages multiple MCP server connections │             │
│  │  - Auto-discovers and registers tools      │             │
│  │  - Handles reconnection                    │             │
│  └────────┬───────────────────────────────────┘             │
│           │                                                 │
│           │ uses                                            │
│           │                                                 │
│  ┌────────▼───────────────────────────────────┐             │
│  │         McpClient (per server)             │             │
│  │  - JSON-RPC 2.0 over stdio                 │             │
│  │  - Tools discovery (tools/list)            │             │
│  │  - Tool invocation (tools/call)            │             │
│  └────────┬───────────────────────────────────┘             │
│           │                                                 │
│           │ wraps as                                        │
│           │                                                 │
│  ┌────────▼───────────────────────────────────┐             │
│  │      McpToolAdapter (per tool)             │             │
│  │  - Implements CapabilityProvider           │             │
│  │  - Converts ActionCall → MCP format        │             │
│  │  - Handles errors and timeouts             │             │
│  └────────────────────────────────────────────┘             │
│                                                             │
└─────────────────┬───────────────────────────────────────────┘
                  │ stdio
                  │
        ┌─────────▼──────────┐
        │   MCP Server       │
        │   (Node/Python)    │
        └────────────────────┘
```

## Components

### 1. `McpClient` (`client.rs`)

Low-level MCP protocol client:

- Spawns and manages child process (MCP server)
- JSON-RPC 2.0 communication over stdin/stdout
- Request/response correlation with timeouts
- Implements MCP protocol: `initialize`, `tools/list`, `tools/call`

**Key Methods:**

- `connect()` - Start server and initialize connection
- `list_tools()` - Discover available tools
- `call_tool(name, args)` - Invoke a tool
- `disconnect()` - Clean shutdown

### 2. `McpToolAdapter` (`adapter.rs`)

Adapts MCP tools to Loom's `CapabilityProvider` interface:

- Converts `ActionCall` → MCP `tools/call` request
- Parses MCP response → `ActionResult`
- Error mapping (TIMEOUT, TOOL_ERROR, INVALID_PARAMS, etc.)
- Adds server prefix to tool names (`server:tool`)

### 3. `McpManager` (`manager.rs`)

High-level manager for multiple MCP servers:

- Connects to configured servers
- Auto-discovers and registers all tools
- Lifecycle management (add/remove/reconnect servers)
- Graceful shutdown

**Key Methods:**

- `add_server(config)` - Connect and register server
- `remove_server(name)` - Disconnect server
- `reconnect_server(name)` - Reconnect after failure
- `list_servers()` - Get connected servers
- `shutdown()` - Clean shutdown of all servers

### 4. `types.rs`

MCP protocol types:

- JSON-RPC 2.0 structures
- MCP-specific types (InitializeParams, ToolSchema, etc.)
- Server configuration
- Error types

## Usage

### Basic Setup

````rust
use loom_core::{Loom, mcp::types::McpServerConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize Loom
    let mut loom = Loom::new().await?;

    // Configure MCP server
    let config = McpServerConfig {
        name: "filesystem".to_string(),
        command: "npx".to_string(),
        args: vec![
            "-y".to_string(),
            "@modelcontextprotocol/server-filesystem".to_string(),
            "/home/user/docs".to_string()
        ],
        env: None,
        cwd: None,
        protocol_version: None, // Uses default (2024-11-05)
    };

    // Connect to MCP server (auto-discovers and registers tools)
    loom.mcp_manager.add_server(config).await?;

    // Start Loom
    loom.start().await?;

    // Tools are now available as "filesystem:read_file", etc.

    Ok(())
}
```### Load from Configuration File

```rust
// Load from TOML
#[derive(serde::Deserialize)]
struct Config {
    servers: Vec<McpServerConfig>,
}

let content = std::fs::read_to_string("mcp-config.toml")?;
let config: Config = toml::from_str(&content)?;

for server in config.servers {
    loom.mcp_manager.add_server(server).await?;
}
````

### Invoke MCP Tools

```rust
use loom_core::proto::{ActionCall, ActionStatus};
use serde_json::json;

// Create tool call
let call = ActionCall {
    id: "call-1".to_string(),
    capability: "filesystem:read_file".to_string(),
    payload: serde_json::to_vec(&json!({
        "path": "/docs/README.md"
    }))?,
    timeout_ms: 5000,
    ..Default::default()
};

// Invoke through ActionBroker
let result = loom.action_broker.invoke(call).await?;

if result.status == ActionStatus::ActionOk as i32 {
    let output: serde_json::Value = serde_json::from_slice(&result.output)?;
    println!("Result: {}", output);
}
```

### From Python SDK

```python
from loom import Agent, Context

class FileAgent(Agent):
    async def on_event(self, ctx: Context, event):
        # Use MCP tool
        result = await ctx.tool("filesystem:read_file", {
            "path": "/docs/README.md"
        })

        print(f"File content: {result}")
```

## MCP Protocol Support

Currently implemented:

- ✅ Stdio transport (most common)
- ✅ Initialize handshake
- ✅ Tools discovery (`tools/list` with pagination)
- ✅ Tool invocation (`tools/call`)
- ✅ Error handling (protocol errors, timeouts, tool errors)
- ✅ Text and image content types
- ✅ Resource references
- ✅ Configurable protocol version (defaults to 2024-11-05)

**Supported Protocol Versions:**

- `2024-11-05` (default, latest stable)

To use a specific protocol version, set it in `McpServerConfig`:

```rust
let config = McpServerConfig {
    protocol_version: Some("2024-11-05".to_string()),
    ..config
};
```

If not specified, the latest supported version is used automatically.

Future additions:

- [ ] SSE transport (HTTP-based)
- [ ] Resources API (`resources/list`, `resources/read`)
- [ ] Prompts API (`prompts/list`, `prompts/get`)
- [ ] Sampling support (multi-turn tool use)
- [ ] Notifications (`notifications/tools/list_changed`)
- [ ] Support for future protocol versions
- [ ] Notifications (`notifications/tools/list_changed`)

## Error Handling

MCP errors are mapped to ActionBroker error codes:

| MCP Error            | ActionBroker Code | Description                      |
| -------------------- | ----------------- | -------------------------------- |
| Connection failure   | TRANSPORT_ERROR   | Can't connect to MCP server      |
| Invalid JSON-RPC     | PROTOCOL_ERROR    | Malformed protocol messages      |
| Tool not found       | TOOL_NOT_FOUND    | Tool doesn't exist on server     |
| Invalid params       | INVALID_PARAMS    | Arguments don't match schema     |
| Tool execution error | TOOL_ERROR        | Tool ran but returned error      |
| Timeout              | TIMEOUT           | Tool took too long (default 30s) |

## Testing

```bash
# Run MCP tests
cargo test -p loom-core --test mcp_test

# Run MCP example (requires MCP server)
cargo run --example mcp_integration
```

## Performance Considerations

- **Startup overhead**: ~100-500ms per MCP server (process spawn + init)
- **Tool latency**: stdio roundtrip + tool execution time (typically 10-100ms)
- **Memory**: Each server runs as separate process (~20-50MB per server)
- **Concurrency**: Multiple tools can run in parallel (no blocking)

## Security

⚠️ **Important Security Notes:**

1. **Trust MCP servers** - They run as child processes with full system access
2. **Validate inputs** - MCP tools receive agent-provided arguments
3. **Resource limits** - MCP servers can access filesystem, network, databases
4. **API keys** - Store credentials securely (use env vars, not code)
5. **Sandboxing** - Consider running MCP servers in containers for production

## Troubleshooting

### Server won't start

Check command is in PATH:

```bash
which npx
npx -y @modelcontextprotocol/server-filesystem --version
```

### Tools not appearing

Enable debug logging:

```rust
tracing_subscriber::fmt()
    .with_env_filter("mcp_client=debug,mcp_manager=debug,mcp_adapter=debug")
    .init();
```

### Timeout issues

Increase timeout:

```rust
call.timeout_ms = 60_000; // 60 seconds
```

### Process cleanup issues

McpClient automatically kills child processes on drop, but for clean shutdown:

```rust
loom.mcp_manager.shutdown().await;
```

## Examples

See:

- `/core/examples/mcp_integration.rs` - Complete example
- `/mcp-config.toml.example` - Configuration examples
- `/docs/MCP.md` - Full documentation

## References

- [MCP Specification](https://spec.modelcontextprotocol.io/)
- [MCP Servers](https://github.com/modelcontextprotocol/servers)
- [Loom Action Broker](../action_broker.rs)
