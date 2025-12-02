## Tool Registry

The Tool Registry is the central hub for registering and invoking tools (capabilities) in Loom Core. It replaces the legacy ActionBroker with a simpler, more unified API.

### Responsibility

- Register tools that implement the `Tool` trait
- Discover available tools and their schemas
- Invoke tools by name with JSON arguments
- Provide a consistent interface for LLM tool orchestration

### Key Files

- `core/src/tools/mod.rs` — `Tool` trait, `ToolRegistry`, `ToolError`, `ToolResult`
- `core/src/tools/mcp.rs` — MCP (Model Context Protocol) integration

### Tool Trait

All tools implement the `Tool` trait:

```rust
#[async_trait]
pub trait Tool: Send + Sync {
    /// Unique tool name (e.g., "web.search", "weather.get")
    fn name(&self) -> String;

    /// Human-readable description for LLM discovery
    fn description(&self) -> String;

    /// JSON Schema for parameters (OpenAI function-calling compatible)
    fn parameters(&self) -> serde_json::Value;

    /// Execute the tool with JSON arguments
    async fn call(&self, arguments: serde_json::Value) -> ToolResult<serde_json::Value>;
}
```

### ToolRegistry API

```rust
use loom_core::tools::{ToolRegistry, Tool};
use std::sync::Arc;

// Create registry
let registry = Arc::new(ToolRegistry::new());

// Register a tool
registry.register(Arc::new(MyTool::new())).await;

// List all tools
let tools = registry.list_tools().await;

// Invoke a tool by name
let result = registry.call("my.tool", serde_json::json!({
    "param1": "value1"
})).await?;
```

### Error Handling

Tool errors are structured:

```rust
pub enum ToolError {
    NotFound(String),           // Tool not registered
    InvalidArguments(String),   // Argument validation failed
    ExecutionFailed(String),    // Runtime error during execution
    Timeout,                    // Execution exceeded timeout
    Internal(String),           // Unexpected internal error
}

pub type ToolResult<T> = Result<T, ToolError>;
```

### Built-in Tools

| Tool           | Description                |
| -------------- | -------------------------- |
| `web.search`   | DuckDuckGo instant search  |
| `weather.get`  | Open-Meteo weather API     |
| `llm.generate` | LLM text generation        |
| `tts.speak`    | Text-to-speech synthesis   |
| `mcp:*`        | MCP server tools (dynamic) |

### Integration with LLM

The `ToolOrchestrator` uses the registry to:

1. Get tool schemas for LLM function-calling
2. Execute tool calls from LLM responses
3. Format results back to the LLM

```rust
use loom_core::llm::ToolOrchestrator;

let orchestrator = ToolOrchestrator::new(llm_client, registry);

// Tools are automatically discovered from registry
let answer = orchestrator.run(&prompt_bundle, budget, options, correlation_id).await?;
```

### Creating Custom Tools

```rust
use loom_core::tools::{Tool, ToolResult};
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
            .ok_or_else(|| loom_core::ToolError::InvalidArguments(
                "Missing required parameter: input".into()
            ))?;

        // Do something with input
        Ok(json!({
            "status": "success",
            "result": format!("Processed: {}", input)
        }))
    }
}

// Register
registry.register(Arc::new(MyCustomTool)).await;
```

### Migration from ActionBroker

| Old (ActionBroker)                      | New (ToolRegistry)                |
| --------------------------------------- | --------------------------------- |
| `CapabilityProvider` trait              | `Tool` trait                      |
| `broker.register_provider(provider)`    | `registry.register(tool).await`   |
| `broker.invoke(ActionCall)`             | `registry.call(name, args).await` |
| `broker.list_capabilities()`            | `registry.list_tools().await`     |
| `ActionCall { payload: Vec<u8>, ... }`  | `call(name, Value)`               |
| `ActionResult { output: Vec<u8>, ... }` | `ToolResult<Value>`               |

Key changes:

- JSON (`serde_json::Value`) instead of bytes
- Simpler API surface
- Async registration
- Structured error types
