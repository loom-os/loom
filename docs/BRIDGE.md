# Loom Bridge (gRPC) — Protocol and Usage

The Bridge connects external SDK agents (Python/JS) to Loom Core via gRPC. It provides:

- Agent registration (subscriptions and tool descriptors)
- Bidirectional event streaming (client → publish; server → deliveries)
- Tool forwarding to ToolRegistry
- Optional heartbeat
- Reconnection-friendly behavior

## Services

```protobuf
service Bridge {
  rpc RegisterAgent(AgentRegisterRequest) returns (AgentRegisterResponse);
  rpc EventStream(stream ClientEvent) returns (stream ServerEvent);
  rpc ForwardToolCall(ToolCall) returns (ToolResult);
  rpc Heartbeat(HeartbeatRequest) returns (HeartbeatResponse);
}
```

## Key Types

### ToolCall / ToolResult

Tool invocations use JSON-encoded arguments:

```protobuf
message ToolCall {
  string id = 1;                   // Unique call id
  string name = 2;                 // Tool name (e.g., "web.search")
  string arguments = 3;            // JSON-encoded arguments
  map<string, string> headers = 4; // Trace context, auth, etc.
  int64 timeout_ms = 5;            // Hard timeout
}

message ToolResult {
  string id = 1;                   // Matches ToolCall.id
  ToolStatus status = 2;           // OK, ERROR, TIMEOUT, NOT_FOUND, INVALID_ARGUMENTS
  string output = 3;               // JSON-encoded result
  ToolError error = 4;             // Error details if any
}
```

## Stream Handshake

- The server expects the first stream message to be an Ack carrying `agent_id`.
- Clients must enqueue this Ack into the outbound stream BEFORE awaiting the RPC result, otherwise both sides can deadlock.

Client outline (tonic):

```rust
// Create channel and wrap with ReceiverStream as outbound
let (tx, rx) = mpsc::channel(32);
let outbound = ReceiverStream::new(rx);

// Send first Ack with agent_id BEFORE calling event_stream
tx.send(ClientEvent {
    msg: Some(client_event::Msg::Ack(Ack {
        message_id: agent_id.clone(),
    })),
}).await?;

// Now call event_stream
let response = client.event_stream(outbound).await?;
let mut inbound = response.into_inner();
```

## Event Publish/Receive

- After registering with `subscribed_topics`, any publish to those topics is delivered on the server→client stream as `ServerEvent::Delivery`.
- QoS mapping: default uses `QoS_Batched` with bounded channel sizes.

## Tool Forwarding

### Client-Initiated

Call `ForwardToolCall(ToolCall)` to invoke tools registered in the `ToolRegistry`:

```rust
let call = ToolCall {
    id: "call_123".into(),
    name: "web.search".into(),
    arguments: r#"{"query": "rust async"}"#.into(),
    ..Default::default()
};

let result = client.forward_tool_call(call).await?;
```

### Server-Initiated

The protocol supports pushing tool calls to agents via the stream:

- Server sends `ServerEvent::tool_call`
- Agent executes and replies with `ClientEvent::tool_result`

## Heartbeat

Optional unary endpoint `Heartbeat` or inline stream ping/pong.

## Reconnection

Bridge is stateless. On stream end, the server cleans up; clients can re-register with the same agent_id.

## Architecture

```
┌─────────────────┐         gRPC          ┌─────────────────┐
│  External SDK   │◄─────────────────────►│  Loom Bridge    │
│  (Python/JS)    │                       │                 │
└─────────────────┘                       └────────┬────────┘
                                                   │
                                                   ▼
                                          ┌─────────────────┐
                                          │  ToolRegistry   │
                                          │  (loom-core)    │
                                          └────────┬────────┘
                                                   │
                              ┌────────────────────┼────────────────────┐
                              ▼                    ▼                    ▼
                      ┌───────────────┐    ┌───────────────┐    ┌───────────────┐
                      │  web.search   │    │  weather.get  │    │  mcp:*        │
                      └───────────────┘    └───────────────┘    └───────────────┘
```

## Testing Notes

- Integration tests: `bridge/tests/integration/`
- Unit tests: `bridge/tests/`
- Always send Ack before awaiting `event_stream` to avoid deadlocks.

## Next Steps

- Server-initiated tool calls via admin endpoint
- Prometheus metrics for tool latency
- Token-based auth and namespaces
- WebSocket transport option
