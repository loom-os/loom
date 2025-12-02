# loom-proto

Protocol Buffer definitions for Loom's gRPC interfaces.

## Overview

This crate provides the protobuf message types and service definitions used across Loom components for inter-process communication. It uses [tonic](https://github.com/hyperium/tonic) for gRPC code generation.

## Proto Files

| File           | Description                                                                          |
| -------------- | ------------------------------------------------------------------------------------ |
| `event.proto`  | Core event types and QoS levels for the event bus                                    |
| `action.proto` | Tool invocation types (`ToolCall`, `ToolResult`, `ToolDescriptor`) and `ToolService` |
| `agent.proto`  | Agent metadata and status types                                                      |
| `bridge.proto` | Bridge gRPC service for external SDK agents                                          |
| `memory.proto` | Memory/planning service for trading and execution tracking                           |
| `plugin.proto` | Plugin lifecycle and control messages                                                |

## Key Types

### Tool System (action.proto)

```protobuf
// Tool descriptor (matches core Tool trait)
message ToolDescriptor {
  string name = 1;                 // e.g., "filesystem:read_file", "mcp:brave_search"
  string description = 2;          // Human-readable description
  string parameters_schema = 3;    // JSON Schema for arguments
  ProviderKind provider = 4;       // Native, WASM, gRPC, or MCP
}

// Tool invocation request
message ToolCall {
  string id = 1;                   // Unique call id
  string name = 2;                 // Tool name to invoke
  string arguments = 3;            // JSON-encoded arguments
  map<string, string> headers = 4; // Trace context, auth, etc.
  int64 timeout_ms = 5;            // Hard timeout (0 = default)
}

// Tool invocation result
message ToolResult {
  string id = 1;                   // Matches ToolCall.id
  ToolStatus status = 2;           // OK, ERROR, TIMEOUT, NOT_FOUND, INVALID_ARGUMENTS
  string output = 3;               // JSON-encoded result
  ToolError error = 4;             // Error details if any
}
```

### Bridge Service (bridge.proto)

```protobuf
service Bridge {
  // Register an agent with subscriptions and tools
  rpc RegisterAgent(AgentRegisterRequest) returns (AgentRegisterResponse);

  // Bidirectional event stream
  rpc EventStream(stream ClientEvent) returns (stream ServerEvent);

  // Forward a tool call to ToolRegistry
  rpc ForwardToolCall(ToolCall) returns (ToolResult);

  // Health check
  rpc Heartbeat(HeartbeatRequest) returns (HeartbeatResponse);
}
```

## Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
loom-proto = { path = "../loom-proto" }
```

Use in Rust code:

```rust
use loom_proto::{
    ToolCall, ToolResult, ToolStatus, ToolDescriptor, ProviderKind,
    Event, QoSLevel,
};

// Create a tool call
let call = ToolCall {
    id: "call_123".to_string(),
    name: "web.search".to_string(),
    arguments: r#"{"query": "rust programming"}"#.to_string(),
    headers: Default::default(),
    timeout_ms: 30_000,
    correlation_id: String::new(),
    qos: QoSLevel::QosBatched as i32,
};

// Create a tool result
let result = ToolResult {
    id: call.id.clone(),
    status: ToolStatus::ToolOk as i32,
    output: r#"{"results": [...]}"#.to_string(),
    error: None,
};
```

## Building

The proto files are compiled during `cargo build` via `build.rs`:

```rust
tonic_build::configure()
    .build_server(true)
    .build_client(true)
    .compile(&[
        "proto/event.proto",
        "proto/action.proto",
        "proto/agent.proto",
        "proto/bridge.proto",
        "proto/memory.proto",
        "proto/plugin.proto",
    ], &["proto"])?;
```

## Migration from ActionBroker

The previous `ActionBroker` gRPC service has been replaced with `ToolService`:

| Old (deprecated)            | New                      |
| --------------------------- | ------------------------ |
| `ActionCall`                | `ToolCall`               |
| `ActionResult`              | `ToolResult`             |
| `ActionStatus`              | `ToolStatus`             |
| `ActionError`               | `ToolError`              |
| `CapabilityDescriptor`      | `ToolDescriptor`         |
| `ActionBroker.InvokeAction` | `ToolService.InvokeTool` |
| `Bridge.ForwardAction`      | `Bridge.ForwardToolCall` |

The new API uses JSON-encoded arguments (`string`) instead of raw bytes for better interoperability.
