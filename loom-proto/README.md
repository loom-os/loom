# loom-proto

Protocol Buffer definitions for Loom's gRPC interfaces.

## Overview

This crate provides the protobuf message types and service definitions used across Loom components for inter-process communication. It uses [tonic](https://github.com/hyperium/tonic) for gRPC code generation.

## Proto Files

| File              | Description                                                                          |
| ----------------- | ------------------------------------------------------------------------------------ |
| `event.proto`     | Core event types and QoS levels for the event bus                                    |
| `action.proto`    | Tool invocation types (`ToolCall`, `ToolResult`, `ToolDescriptor`) and `ToolService` |
| `agent.proto`     | Agent metadata and status types                                                      |
| `streaming.proto` | Streaming protocol for real-time LLM output and agent responses                      |
| `bridge.proto`    | Bridge gRPC service for external SDK agents                                          |
| `memory.proto`    | Memory/planning service for trading and execution tracking                           |
| `plugin.proto`    | Plugin lifecycle and control messages                                                |

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

  // Bidirectional event stream (supports streaming chunks)
  rpc EventStream(stream ClientEvent) returns (stream ServerEvent);

  // Forward a tool call to ToolRegistry
  rpc ForwardToolCall(ToolCall) returns (ToolResult);

  // Health check
  rpc Heartbeat(HeartbeatRequest) returns (HeartbeatResponse);
}

// Server-to-client messages include streaming support
message ServerEvent {
  oneof msg {
    Delivery delivery = 1;
    HeartbeatResponse pong = 2;
    Error err = 3;
    ToolCall tool_call = 4;
    StreamChunk stream_chunk = 5;      // Real-time content chunks
    StreamComplete stream_complete = 6; // Stream completion signal
    StreamState stream_state = 7;       // State notifications
  }
}
```

### Streaming Protocol (streaming.proto)

The streaming protocol enables real-time output from cognitive agents:

```protobuf
// A chunk of streaming content from an LLM or agent response
message StreamChunk {
  string correlation_id = 1;  // Match with original request
  string content = 2;         // The content chunk
  uint32 sequence = 3;        // Sequence number for ordering
  StreamContentType content_type = 4;  // TEXT, THINKING, TOOL_CALL, etc.
  map<string, string> metadata = 5;
  int64 timestamp_ms = 6;
}

// Content types for differentiated rendering
enum StreamContentType {
  STREAM_CONTENT_TEXT = 0;        // LLM response text
  STREAM_CONTENT_THINKING = 1;    // Reasoning/CoT content
  STREAM_CONTENT_TOOL_CALL = 2;   // Tool invocation
  STREAM_CONTENT_TOOL_RESULT = 3; // Tool response
  STREAM_CONTENT_ERROR = 4;       // Error message
  STREAM_CONTENT_STATUS = 5;      // Progress update
  STREAM_CONTENT_MARKDOWN = 6;    // Markdown formatted
  STREAM_CONTENT_CODE = 7;        // Code block
}

// Signals completion of a streaming response
message StreamComplete {
  string correlation_id = 1;
  string final_content = 2;   // Aggregated response
  uint32 total_chunks = 3;
  StreamStatus status = 4;    // OK, CANCELLED, TIMEOUT, ERROR, TRUNCATED
  StreamError error = 5;
  StreamStats stats = 6;      // Execution statistics
}

// Execution statistics
message StreamStats {
  uint32 total_tokens = 1;
  int64 duration_ms = 2;
  uint32 tool_calls = 3;
  uint32 iterations = 4;
  int64 first_chunk_latency_ms = 5;  // Time to first token
}
```

#### Streaming Flow

```
Client                    Bridge                   Backend Agent
  │                         │                           │
  │──── StreamRequest ─────►│────── Delivery ──────────►│
  │                         │                           │
  │                         │◄──── StreamChunk[0] ──────│
  │◄─── StreamChunk[0] ─────│                           │
  │                         │◄──── StreamChunk[1] ──────│
  │◄─── StreamChunk[1] ─────│                           │
  │           ...           │           ...             │
  │                         │◄──── StreamComplete ──────│
  │◄─── StreamComplete ─────│                           │
  │                         │                           │
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
        "proto/streaming.proto",
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
