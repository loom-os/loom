# Tools Module

Tool system for Loom Core runtime. Provides a unified interface for native tools and MCP integrations.

## Architecture

```
tools/
├── mod.rs          # Module exports
├── traits.rs       # Tool trait definition
├── registry.rs     # Tool registration and lookup
├── error.rs        # Error types
├── native/         # Built-in native tools
│   ├── filesystem.rs   # fs:read_file, fs:write_file, fs:list_dir, fs:delete
│   ├── shell.rs        # system:shell
│   ├── weather.rs      # weather:get
│   └── web_search.rs   # web:search
└── mcp/            # Model Context Protocol integration
    ├── client.rs       # MCP client implementation
    ├── manager.rs      # Server lifecycle management
    ├── adapter.rs      # Tool adapter (MCP → native trait)
    └── types.rs        # MCP protocol types
```

## Tool Trait

All tools implement the `Tool` trait:

```rust
#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters_schema(&self) -> serde_json::Value;
    async fn execute(&self, params: serde_json::Value) -> ToolResult<serde_json::Value>;
}
```

## Native Tools

| Tool            | Description              | Env Vars                       |
| --------------- | ------------------------ | ------------------------------ |
| `fs:read_file`  | Read file from workspace | -                              |
| `fs:write_file` | Write file to workspace  | -                              |
| `fs:list_dir`   | List directory contents  | -                              |
| `fs:delete`     | Delete file/directory    | -                              |
| `system:shell`  | Execute shell command    | -                              |
| `weather:get`   | Get weather data         | -                              |
| `web:search`    | Search the web           | `BRAVE_API_KEY`, `HTTPS_PROXY` |

See [docs/native_tools/](../../../docs/native_tools/) for detailed usage documentation.

## Tool Registry

The `ToolRegistry` manages tool registration and lookup:

```rust
let mut registry = ToolRegistry::new();

// Register native tools
registry.register(Box::new(ReadFileTool::new(workspace_root)));
registry.register(Box::new(WebSearchTool::new()));

// Execute a tool
let result = registry.execute("fs:read_file", json!({"path": "data.txt"})).await?;
```

## MCP Integration

MCP servers are configured in `mcp-config.toml`:

```toml
[servers.filesystem]
command = "npx"
args = ["-y", "@modelcontextprotocol/server-filesystem", "/path/to/dir"]
```

The `McpManager` handles server lifecycle, and `McpToolAdapter` wraps MCP tools as native `Tool` instances.

## Adding a New Native Tool

1. Create a new file in `native/` (e.g., `my_tool.rs`)
2. Implement the `Tool` trait
3. Export in `native/mod.rs`
4. Register in the tool registry initialization

```rust
pub struct MyTool;

#[async_trait]
impl Tool for MyTool {
    fn name(&self) -> &str { "my:tool" }
    fn description(&self) -> &str { "Does something useful" }
    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "input": { "type": "string", "description": "Input value" }
            },
            "required": ["input"]
        })
    }
    async fn execute(&self, params: serde_json::Value) -> ToolResult<serde_json::Value> {
        let input = params["input"].as_str().ok_or(ToolError::InvalidInput)?;
        Ok(json!({ "result": format!("Processed: {}", input) }))
    }
}
```

## Security

- **Workspace isolation**: File tools restricted to workspace root
- **Shell allowlist**: Only pre-approved commands auto-execute
- **Human-in-the-loop**: Destructive operations require approval (handled by Python SDK)
