## Loom Core — Built-in Tools

This document describes the built-in tools in the `core` crate and how to create custom tools.

### Overview

Tools implement the `Tool` trait and are registered with the `ToolRegistry`. They expose structured interfaces (OpenAI-compatible JSON schemas) that LLMs can discover and invoke.

### Architecture

```
┌─────────────┐         ┌─────────────────┐         ┌─────────────────┐
│    LLM      │◄───────►│ ToolOrchestrator│◄───────►│  ToolRegistry   │
└─────────────┘         └─────────────────┘         └────────┬────────┘
                                                             │
                              ┌──────────────────────────────┼──────────────────────────────┐
                              ▼                              ▼                              ▼
                      ┌───────────────┐              ┌───────────────┐              ┌───────────────┐
                      │ WebSearchTool │              │  WeatherTool  │              │   MCP Tools   │
                      └───────────────┘              └───────────────┘              └───────────────┘
```

### Tool Trait

All tools implement the `Tool` trait:

```rust
#[async_trait]
pub trait Tool: Send + Sync {
    /// Unique tool name (e.g., "web.search")
    fn name(&self) -> String;

    /// Human-readable description
    fn description(&self) -> String;

    /// JSON Schema for parameters
    fn parameters(&self) -> serde_json::Value;

    /// Execute the tool
    async fn call(&self, arguments: serde_json::Value) -> ToolResult<serde_json::Value>;
}
```

### Built-in Tools

#### Web Search Tool

**Name**: `web.search`
**API**: DuckDuckGo Instant Answer API (free, no auth)

```rust
use loom_core::WebSearchTool;

let tool = WebSearchTool::new();
let result = tool.call(serde_json::json!({
    "query": "rust programming"
})).await?;
```

**Parameters**:
- `query` (required, string): Search query

**Features**:
- Instant answers and summaries
- Related topics extraction
- Automatic URL encoding

#### Weather Tool

**Name**: `weather.get`
**API**: Open-Meteo API (free, no auth, auto-geocoding)

```rust
use loom_core::WeatherTool;

let tool = WeatherTool::new();
let result = tool.call(serde_json::json!({
    "location": "San Francisco",
    "units": "fahrenheit"
})).await?;
```

**Parameters**:
- `location` (required, string): City name or coordinates
- `units` (optional, string): "celsius" (default) or "fahrenheit"

**Features**:
- Automatic geocoding
- Current conditions: temperature, humidity, wind, pressure
- WMO weather codes → human-readable descriptions

#### Filesystem Tool

**Name**: `filesystem.read_file`

```rust
use loom_core::ReadFileTool;

let tool = ReadFileTool::new();
let result = tool.call(serde_json::json!({
    "path": "/path/to/file.txt"
})).await?;
```

#### Shell Tool

**Name**: `shell.exec`

```rust
use loom_core::ShellTool;

let tool = ShellTool::new();
let result = tool.call(serde_json::json!({
    "command": "ls -la"
})).await?;
```

### Registration with ToolRegistry

```rust
use loom_core::{ToolRegistry, WebSearchTool, WeatherTool};
use std::sync::Arc;

let registry = Arc::new(ToolRegistry::new());

// Register tools
registry.register(Arc::new(WebSearchTool::new())).await;
registry.register(Arc::new(WeatherTool::new())).await;

// List registered tools
let tools = registry.list_tools();
for tool in &tools {
    println!("{}: {}", tool.name(), tool.description());
}

// Invoke a tool
let result = registry.call("web.search", serde_json::json!({
    "query": "rust async"
})).await?;
```

### LLM Integration

The `ToolOrchestrator` bridges LLM tool calls to the registry:

```rust
use loom_core::cognitive::llm::orchestrator::ToolOrchestrator;

let orchestrator = ToolOrchestrator::new(llm_client, registry);

let answer = orchestrator.run(
    &prompt_bundle,
    Some(budget),
    options,
    Some("correlation_id".to_string()),
).await?;
```

### Creating Custom Tools

```rust
use loom_core::tools::{Tool, ToolResult, ToolError};
use async_trait::async_trait;
use serde_json::{json, Value};

pub struct MyCustomTool;

#[async_trait]
impl Tool for MyCustomTool {
    fn name(&self) -> String {
        "custom.action".to_string()
    }

    fn description(&self) -> String {
        "Performs a custom action".to_string()
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "input": {
                    "type": "string",
                    "description": "Input data"
                }
            },
            "required": ["input"]
        })
    }

    async fn call(&self, args: Value) -> ToolResult<Value> {
        let input = args.get("input")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArguments(
                "Missing required parameter: input".into()
            ))?;

        // Execute tool logic
        Ok(json!({
            "status": "success",
            "result": format!("Processed: {}", input)
        }))
    }
}

// Register
registry.register(Arc::new(MyCustomTool)).await;
```

### Error Handling

```rust
pub enum ToolError {
    NotFound(String),           // Tool not in registry
    InvalidArguments(String),   // Argument validation failed
    ExecutionFailed(String),    // Runtime error
    Timeout,                    // Execution exceeded timeout
    Internal(String),           // Unexpected error
}
```

### MCP Tools

MCP (Model Context Protocol) tools are dynamically loaded from external servers:

```rust
use loom_core::{McpManager, McpClient};

// Connect to MCP server
let client = McpClient::connect("http://localhost:8080").await?;

// Tools are automatically registered with "mcp:" prefix
// e.g., "mcp:brave_search", "mcp:memory_store"
```

See [MCP.md](../MCP.md) for MCP integration details.
