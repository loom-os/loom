# Model Context Protocol (MCP) Integration

Loom provides first-class support for the Model Context Protocol (MCP), enabling agents to access a rich ecosystem of tools and capabilities.

## Overview

MCP is an open protocol that standardizes how applications provide context to LLMs. With MCP, you can:

- **Access tools** - File systems, databases, web search, APIs, etc.
- **Retrieve context** - Documentation, code, data from various sources
- **Perform actions** - Execute commands, update systems, automate workflows

Loom's MCP integration automatically:

1. Connects to configured MCP servers
2. Discovers available tools
3. Registers them as capabilities
4. Makes them available to all agents

## Quick Start

### 1. Configure MCP Servers

Create a `mcp-config.toml` file:

```toml
[[servers]]
name = "filesystem"
command = "npx"
args = ["-y", "@modelcontextprotocol/server-filesystem", "/path/to/docs"]

[[servers]]
name = "brave-search"
command = "npx"
args = ["-y", "@modelcontextprotocol/server-brave-search"]
env = { "BRAVE_API_KEY" = "your-api-key" }

# Optional: Specify protocol version (defaults to latest: 2024-11-05)
# protocol_version = "2024-11-05"
```

### 2. Load Configuration in Your Application

```rust
use loom_core::{Loom, mcp::types::McpServerConfig};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize Loom
    let mut loom = Loom::new().await?;

    // Load MCP configuration
    let config: Vec<McpServerConfig> = load_mcp_config("mcp-config.toml")?;

    // Connect to MCP servers
    for server_config in config {
        loom.mcp_manager.add_server(server_config).await?;
    }

    // Start the system
    loom.start().await?;

    // Now agents can use MCP tools like "filesystem:read_file" or "brave-search:search"

    Ok(())
}

fn load_mcp_config(path: &str) -> Result<Vec<McpServerConfig>, Box<dyn std::error::Error>> {
    #[derive(serde::Deserialize)]
    struct Config {
        servers: Vec<McpServerConfig>,
    }

    let content = std::fs::read_to_string(path)?;
    let config: Config = toml::from_str(&content)?;
    Ok(config.servers)
}
```

### 3. Use MCP Tools in Agents

Once configured, agents can use MCP tools through the standard tool orchestration:

```python
# In Python SDK (loom-py)
from loom import Agent, Context

class MyAgent(Agent):
    async def on_event(self, ctx: Context, event):
        # Use MCP filesystem tool
        result = await ctx.tool("filesystem:read_file", {
            "path": "/docs/README.md"
        })

        # Use MCP search tool
        search_result = await ctx.tool("brave-search:search", {
            "query": "rust async programming"
        })

        await ctx.emit("task.result", {
            "file_content": result,
            "search_results": search_result
        })
```

## Available MCP Servers

The MCP ecosystem includes many pre-built servers:

### Official Servers (by Anthropic)

- **@modelcontextprotocol/server-filesystem** - Read/write local files
- **@modelcontextprotocol/server-github** - GitHub API access
- **@modelcontextprotocol/server-gitlab** - GitLab API access
- **@modelcontextprotocol/server-google-drive** - Google Drive access
- **@modelcontextprotocol/server-slack** - Slack integration
- **@modelcontextprotocol/server-postgres** - PostgreSQL database access
- **@modelcontextprotocol/server-brave-search** - Web search via Brave
- **@modelcontextprotocol/server-puppeteer** - Browser automation

### Community Servers

- **@smithery/mcp-server-youtube-transcript** - Get YouTube transcripts
- **@executeautomation/mcp-playwright** - Browser automation with Playwright
- Search the [MCP Servers Repository](https://github.com/modelcontextprotocol/servers)

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                         Loom Core                            │
│                                                               │
│  ┌──────────────┐         ┌──────────────┐                 │
│  │ ActionBroker │◄────────┤  MCP Manager │                 │
│  └──────┬───────┘         └──────┬───────┘                 │
│         │                         │                          │
│         │ register                │ connect                  │
│         │                         │                          │
│  ┌──────▼─────────────────────────▼──────┐                 │
│  │         McpToolAdapters                │                 │
│  │  ┌──────────┐  ┌──────────┐           │                 │
│  │  │Tool1     │  │Tool2     │  ...      │                 │
│  │  └────┬─────┘  └────┬─────┘           │                 │
│  └───────┼─────────────┼──────────────────┘                 │
│          │             │                                     │
└──────────┼─────────────┼─────────────────────────────────────┘
           │             │
           ▼             ▼
    ┌──────────┐   ┌──────────┐
    │MCP Server│   │MCP Server│
    │  (stdio) │   │  (stdio) │
    └──────────┘   └──────────┘
```

### Components

1. **McpManager** - Manages multiple MCP server connections
2. **McpClient** - Low-level JSON-RPC 2.0 client (stdio transport)
3. **McpToolAdapter** - Adapts MCP tools to `CapabilityProvider` trait
4. **ActionBroker** - Invokes tools with timeout/idempotency/correlation

### Tool Naming Convention

MCP tools are registered with a qualified name:

```
{server_name}:{tool_name}
```

Example:

- `filesystem:read_file`
- `brave-search:search`
- `postgres:query`

This prevents naming conflicts between servers.

## Advanced Features

### Reconnection

If an MCP server crashes or disconnects, you can reconnect:

```rust
loom.mcp_manager.reconnect_server("filesystem").await?;
```

### Dynamic Server Management

Add/remove servers at runtime:

```rust
// Add a new server
let config = McpServerConfig {
    name: "new-server".to_string(),
    command: "npx".to_string(),
    args: vec!["-y", "@modelcontextprotocol/server-example"],
    env: None,
    cwd: None,
    protocol_version: None, // Uses default (2024-11-05)
};
loom.mcp_manager.add_server(config).await?;

// Or specify a custom protocol version
let config_custom = McpServerConfig {
    protocol_version: Some("2024-11-05".to_string()),
    ..config
};

// Remove a server
loom.mcp_manager.remove_server("old-server").await?;

// List active servers
let servers = loom.mcp_manager.list_servers().await;
```

### Tool Discovery

List available tools from the ActionBroker:

```rust
let capabilities = loom.action_broker.list_capabilities();
for cap in capabilities {
    if cap.provider == ProviderKind::ProviderMcp as i32 {
        println!("MCP Tool: {} - {}",
            cap.name,
            cap.metadata.get("desc").unwrap_or(&"".to_string())
        );
    }
}
```

## Error Handling

MCP tool invocations can fail for various reasons:

- **TRANSPORT_ERROR** - Connection issues with MCP server
- **TIMEOUT** - Tool took too long to respond
- **TOOL_NOT_FOUND** - Tool doesn't exist on the server
- **INVALID_PARAMS** - Invalid arguments provided
- **TOOL_ERROR** - Tool execution failed

Errors are automatically converted to `ActionResult` with appropriate status codes.

## Security Considerations

1. **Trust MCP Servers** - Only connect to trusted MCP servers
2. **Validate Inputs** - MCP servers receive agent-provided arguments
3. **Resource Limits** - MCP tools can access filesystem/network/databases
4. **API Keys** - Store sensitive credentials securely (not in code)

## Performance

- **Connection Overhead** - MCP servers are persistent processes
- **Tool Latency** - stdio communication + tool execution time
- **Concurrency** - Multiple tools can run in parallel
- **Caching** - ActionBroker provides idempotency caching

## Troubleshooting

### Server Won't Start

Check that the command is in PATH:

```bash
which npx
npx -y @modelcontextprotocol/server-filesystem --version
```

### Tools Not Appearing

Enable debug logging:

```rust
tracing_subscriber::fmt()
    .with_target(true)
    .with_level(true)
    .with_env_filter("mcp_client=debug,mcp_manager=debug")
    .init();
```

### Timeout Issues

Increase timeout in `ActionCall`:

```rust
call.timeout_ms = 60_000; // 60 seconds
```

## Building Your Own MCP Server

See the [MCP specification](https://spec.modelcontextprotocol.io/) and [SDK documentation](https://github.com/modelcontextprotocol) for building custom servers in:

- TypeScript/JavaScript
- Python
- Rust (coming soon)

## Roadmap

- [ ] SSE transport support (in addition to stdio)
- [ ] MCP server mode (expose Loom capabilities as MCP tools)
- [ ] Auto-reconnection with exponential backoff
- [ ] Tool execution metrics and circuit breakers
- [ ] Resource/prompt support (beyond tools)
- [ ] Sampling support for multi-turn tool use

## References

- [MCP Specification](https://spec.modelcontextprotocol.io/)
- [MCP Servers Repository](https://github.com/modelcontextprotocol/servers)
- [MCP Documentation](https://modelcontextprotocol.io/)
- [Loom Core Architecture](./ARCHITECTURE.md)
- [Action Broker Documentation](./core/action_broker.md)
