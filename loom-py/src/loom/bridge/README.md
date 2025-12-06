# Bridge Module

gRPC client for communication between Python agents and Rust Core.

## Overview

The Bridge is a Rust gRPC service that acts as the communication layer between:
- **Python Agents** (loom-py)
- **Rust Core** (event bus, tools, memory)

This module provides the **Python client** (`BridgeClient`) for Bridge communication.

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                   Python Agent                          │
│  ┌──────────────┐         ┌──────────────┐             │
│  │ EventContext │────────▶│ BridgeClient │             │
│  │ (event.py)   │         │ (client.py)  │             │
│  └──────────────┘         └──────────────┘             │
└────────────────────────────────│────────────────────────┘
                                  │
                     gRPC (HTTP/2) │ Port 50051
                                  │
┌─────────────────────────────────▼────────────────────────┐
│              Bridge (Rust Service)                       │
│  ┌────────────┐  ┌────────────┐  ┌────────────┐        │
│  │   Stream   │  │  Forward   │  │   Memory   │        │
│  │  Service   │  │   Action   │  │  Service   │        │
│  └────────────┘  └────────────┘  └────────────┘        │
└────────────────────────────────┬─────────────────────────┘
                                  │
                                  ▼
┌──────────────────────────────────────────────────────────┐
│                   Rust Core                              │
│  ┌─────────────┐  ┌─────────────┐  ┌──────────────┐    │
│  │  Event Bus  │  │ Tool System │  │ Memory Store │    │
│  └─────────────┘  └─────────────┘  └──────────────┘    │
└──────────────────────────────────────────────────────────┘
```

## Key Components

### BridgeClient (`client.py`)

Manages gRPC connection and provides RPC methods:

**Connection Management:**
```python
from loom.bridge import BridgeClient

# Connect to default local bridge
client = BridgeClient()  # localhost:50051
await client.connect()

# Connect to remote bridge
client = BridgeClient(address="bridge.example.com:50051")
await client.connect()

# Disconnect
await client.disconnect()
```

**Stream Communication:**
```python
# Bidirectional event stream
async for delivery in client.stream(outbound_events):
    if delivery.event:
        # Process incoming event
        envelope = Envelope.from_proto(delivery.event)
        await handle_event(envelope)
```

**Tool Forwarding:**
```python
from loom.bridge.proto import action_pb2

call = action_pb2.ToolCall(
    id="call-123",
    name="web:search",
    arguments='{"query": "AI frameworks"}',
    timeout_ms=10000,
)

result = await client.forward_tool_call(call)
if result.status == action_pb2.TOOL_OK:
    print(result.output)
```

**Memory Operations:**
```python
from loom.bridge.proto import memory_pb2

# Save plan
req = memory_pb2.SavePlanRequest(
    session_id="agent-1",
    plan=memory_pb2.PlanRecord(
        symbol="BTC",
        action="BUY",
        confidence=0.85,
        ...
    )
)
resp = await client.save_plan(req)

# Check duplicate
req = memory_pb2.CheckDuplicateRequest(...)
resp = await client.check_duplicate(req)

# Get recent plans
req = memory_pb2.GetRecentPlansRequest(...)
resp = await client.get_recent_plans(req)
```

### Protobuf Definitions (`proto/generated/`)

All protocol buffers are auto-generated from `.proto` files:

- **`bridge_pb2.py`**: Stream protocol, client events
- **`event_pb2.py`**: Event envelope format
- **`action_pb2.py`**: Tool calls and results
- **`memory_pb2.py`**: Memory service RPCs
- **`agent_pb2.py`**: Agent registration
- **`plugin_pb2.py`**: Plugin system (future)

**Regenerate protobuf:**
```bash
cd loom-py/src/loom/bridge/proto
python generate.py
```

## Protocol Details

### Stream Protocol

Bidirectional stream for real-time communication:

```protobuf
service BridgeService {
  rpc Stream(stream ClientEvent) returns (stream Delivery);
}

message ClientEvent {
  oneof event {
    Publish publish = 1;      // Publish event to topic
    Register register = 2;    // Register agent
    Heartbeat heartbeat = 3;  // Keep-alive ping
  }
}

message Delivery {
  oneof delivery {
    Event event = 1;          // Incoming event
    Ack ack = 2;              // Registration ack
  }
}
```

**Usage Flow:**
1. Client opens stream
2. Client sends `Register` event (agent_id, topics, tools)
3. Bridge responds with `Ack`
4. Client sends `Publish` events, receives `Event` deliveries
5. Client sends `Heartbeat` every 30s to maintain connection

### Tool Forwarding Protocol

Synchronous RPC for tool invocation:

```protobuf
service ActionService {
  rpc ForwardToolCall(ToolCall) returns (ToolResult);
}

message ToolCall {
  string id = 1;
  string name = 2;
  string arguments = 3;       // JSON string
  int32 timeout_ms = 4;
}

message ToolResult {
  ToolStatus status = 1;      // OK, ERROR, TIMEOUT
  string output = 2;          // JSON string
  ToolError error = 3;
}
```

### Memory Service Protocol

Async RPCs for persistent memory:

```protobuf
service MemoryService {
  rpc SavePlan(SavePlanRequest) returns (SavePlanResponse);
  rpc CheckDuplicate(CheckDuplicateRequest) returns (CheckDuplicateResponse);
  rpc GetRecentPlans(GetRecentPlansRequest) returns (GetRecentPlansResponse);
  rpc MarkExecuted(MarkExecutedRequest) returns (MarkExecutedResponse);
  rpc CheckExecuted(CheckExecutedRequest) returns (CheckExecutedResponse);
  rpc GetExecutionStats(GetExecutionStatsRequest) returns (GetExecutionStatsResponse);
}
```

## Configuration

Bridge connection can be configured via:

**1. Environment Variable:**
```bash
export LOOM_BRIDGE_ADDR="localhost:50051"
```

**2. Constructor Argument:**
```python
client = BridgeClient(address="bridge.example.com:9999")
```

**3. Default:**
```python
client = BridgeClient()  # Uses localhost:50051
```

## Error Handling

```python
from grpc import RpcError

try:
    await client.connect()
except RpcError as e:
    print(f"Failed to connect: {e.code()} - {e.details()}")
    # Retry logic...

try:
    result = await client.forward_tool_call(call)
except RpcError as e:
    print(f"Tool call failed: {e}")
    # Fallback logic...
```

## Development

### Regenerate Protobuf

When `.proto` files change in Rust Core:

```bash
# Copy updated .proto files
cp ../../../../loom-proto/proto/*.proto proto/

# Regenerate Python code
cd proto
python generate.py
```

### Testing Bridge Connection

```bash
# Start Bridge (from Rust)
cd ../../../bridge
cargo run --release

# Test connection (from Python)
pytest tests/integration/test_bridge.py -v
```

## Performance Considerations

### 1. **Connection Pooling**

`BridgeClient` maintains a single connection per agent. For multi-agent systems, each agent gets its own client:

```python
agents = []
for i in range(10):
    client = BridgeClient()
    await client.connect()
    agents.append(Agent(agent_id=f"agent-{i}", client=client))
```

### 2. **Backpressure**

The stream has a queue size of 2048 for batched processing. If the queue fills up, publishes will block until space is available.

### 3. **Timeout Configuration**

Tool calls have configurable timeouts:

```python
# Short timeout for fast tools
await ctx.tool("cache:get", payload={"key": "x"}, timeout_ms=1000)

# Long timeout for slow tools
await ctx.tool("web:scrape", payload={"url": "..."}, timeout_ms=30000)
```

## Security

### 1. **TLS Support** (Future)

Currently using insecure channels. TLS support planned for production:

```python
# Future API
client = BridgeClient(
    address="bridge.example.com:443",
    credentials=grpc.ssl_channel_credentials(...)
)
```

### 2. **Authentication** (Future)

API key or JWT-based authentication planned:

```python
# Future API
client = BridgeClient(
    address="bridge.example.com:50051",
    metadata=[("authorization", "Bearer <token>")]
)
```

## Troubleshooting

### Connection Refused

```
Error: Failed to connect to localhost:50051
```

**Solution:**
1. Check Bridge is running: `ps aux | grep bridge`
2. Start Bridge: `loom up --mode bridge-only`
3. Verify port: `netstat -tulpn | grep 50051`

### Tool Call Timeout

```
ToolResult.status = TIMEOUT
```

**Solution:**
1. Increase timeout: `timeout_ms=60000` (60s)
2. Check Core logs for tool execution delays
3. Verify tool is registered in Core

### Memory RPC Errors

```
grpc.RpcError: StatusCode.UNIMPLEMENTED
```

**Solution:**
1. Update Bridge to latest version (memory RPCs added in v0.2.0)
2. Check Bridge logs for errors
3. Verify Core has memory service enabled

## Related Documentation

- **[Bridge Rust Docs](../../../../bridge/README.md)**: Rust Bridge implementation
- **[Protocol Buffers](../../../../loom-proto/proto/)**: Proto definitions
- **[Agent Module](../agent/README.md)**: How agents use BridgeClient
- **[Core Memory](../../../../docs/BRIDGE.md)**: Memory service design

---

**Key Insight**: The Bridge is the "nervous system" of Loom - it routes messages between the Brain (Python agents) and the Hands (Rust Core tools and event bus).
