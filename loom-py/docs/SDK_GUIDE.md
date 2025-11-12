# Loom Python SDK Documentation

Complete guide to building multi-agent systems with Loom's Python SDK.

## Table of Contents

- [Installation](#installation)
- [Quick Start](#quick-start)
- [Core Concepts](#core-concepts)
- [API Reference](#api-reference)
- [Examples](#examples)
- [Best Practices](#best-practices)
- [Troubleshooting](#troubleshooting)

## Installation

### From PyPI (Recommended)

```bash
pip install loom
```

### From Source

```bash
git clone https://github.com/loom-os/loom.git
cd loom/loom-py
pip install -e .
```

### Development Installation

```bash
pip install -e ".[dev]"
```

## Quick Start

### 1. Start the Bridge Server

Loom agents communicate through a Bridge server. Start one locally:

```bash
# From the Loom repository root
cargo run -p loom-bridge --bin loom-bridge-server
```

Or set `LOOM_BRIDGE_ADDR` to connect to a remote bridge:

```bash
export LOOM_BRIDGE_ADDR="bridge.example.com:50051"
```

### 2. Create Your First Agent

```python
from loom import Agent, capability

@capability("greet.user", version="1.0")
def greet(name: str) -> dict:
    """Greet a user by name."""
    return {"message": f"Hello, {name}!"}

async def handle_event(ctx, topic, event):
    """Handle incoming events."""
    print(f"Received event: {event.type} on {topic}")

    if event.type == "greet.request":
        name = event.payload.decode()
        result = greet(name)
        await ctx.reply(event, type="greet.response", payload=str(result).encode())

# Create and run agent
agent = Agent(
    agent_id="greeter",
    topics=["greet.topic"],
    capabilities=[greet],
    on_event=handle_event,
)

if __name__ == "__main__":
    agent.run()
```

### 3. Run Your Agent

```python
python my_agent.py
```

## Core Concepts

### Agent

An **Agent** is an autonomous entity that:

- Subscribes to topics
- Receives events
- Executes capabilities (tools)
- Emits new events

**Key Features:**

- Async event handling
- Automatic reconnection
- Heartbeat monitoring
- Graceful shutdown

### Context

The **Context** provides event primitives for agent communication:

```python
async def my_handler(ctx, topic, event):
    # Emit an event
    await ctx.emit("target.topic", type="my.event", payload=b"data")

    # Request-reply pattern
    response = await ctx.request("other.topic", type="query", payload=b"question")

    # Reply to an event
    await ctx.reply(event, type="answer", payload=b"response")

    # Invoke a tool
    result = await ctx.tool("weather.get", payload={"city": "Seattle"})
```

### Capability

A **Capability** is a function that agents can invoke:

```python
@capability("math.add", version="1.0")
def add(a: int, b: int) -> int:
    """Add two numbers."""
    return a + b
```

**Features:**

- Automatic JSON Schema generation from type hints
- Input validation via Pydantic
- Version management
- Distributed invocation

### Envelope

An **Envelope** wraps events with metadata for correlation and routing:

```python
from loom import Envelope

env = Envelope.new(
    event_id="evt-123",
    event_type="my.event",
    payload=b"data",
    thread_id="conversation-1",
    correlation_id="req-456",
)
```

**Metadata Fields:**

- `thread_id`: Groups related events (conversation, workflow)
- `correlation_id`: Links requests and responses
- `sender`: Agent that emitted the event
- `reply_to`: Topic for replies
- `ttl_ms`: Time-to-live in milliseconds

## API Reference

### Agent

```python
class Agent:
    def __init__(
        self,
        agent_id: str,
        topics: list[str],
        capabilities: list[Capability] = [],
        on_event: Optional[EventHandler] = None,
        address: Optional[str] = None,  # Defaults to LOOM_BRIDGE_ADDR or 127.0.0.1:50051
    )

    async def start(self) -> None:
        """Start the agent and connect to bridge."""

    async def stop(self) -> None:
        """Gracefully stop the agent."""

    def run(self) -> None:
        """Run agent synchronously (blocks until stopped)."""
```

### Context

```python
class Context:
    async def emit(
        self,
        topic: str,
        *,
        type: str,
        payload: bytes = b"",
        envelope: Optional[Envelope] = None,
    ) -> None:
        """Publish an event to a topic."""

    async def request(
        self,
        topic: str,
        *,
        type: str,
        payload: bytes = b"",
        timeout_ms: int = 5000,
    ) -> Envelope:
        """Send request and wait for reply."""

    async def reply(
        self,
        original: Envelope,
        *,
        type: str,
        payload: bytes = b"",
    ) -> None:
        """Reply to an event."""

    async def tool(
        self,
        name: str,
        *,
        version: str = "1.0",
        payload: Any = None,
        timeout_ms: int = 5000,
    ) -> bytes:
        """Invoke a capability and return result."""

    async def join_thread(self, thread_id: str) -> None:
        """Join a conversation thread (MVP: placeholder)."""
```

### Capability Decorator

```python
@capability(name: str, version: str = "1.0")
def my_function(param1: Type1, param2: Type2) -> ReturnType:
    """Function docstring becomes capability description."""
    pass
```

**Type Support:**

- Primitive types: `str`, `int`, `float`, `bool`
- Complex types: `dict`, `list`
- Pydantic models for structured validation

### Envelope

```python
@dataclass
class Envelope:
    id: str
    type: str
    payload: bytes
    timestamp_ms: int
    source: str
    thread_id: Optional[str]
    correlation_id: Optional[str]
    sender: Optional[str]
    reply_to: Optional[str]
    ttl_ms: Optional[int]
    metadata: Dict[str, str]
    tags: list[str]

    @classmethod
    def new(...) -> Envelope:
        """Create a new envelope."""

    @classmethod
    def from_proto(ev) -> Envelope:
        """Create from protobuf Event."""

    def to_proto(self, pb_event_cls) -> Any:
        """Convert to protobuf Event."""
```

## Examples

### Multi-Agent Collaboration

See `examples/trio.py` for a complete example:

**Planner Agent** → sends research requests
**Researcher Agent** → performs search, sends to writer
**Writer Agent** → generates final output

```python
# Planner
async def planner_handler(ctx, topic, event):
    if event.type == "user.question":
        await ctx.emit("topic.research", type="research.request", payload=event.payload)

# Researcher
@capability("research.search", version="1.0")
def search(query: str) -> dict:
    return {"results": ["https://example.com/doc1"]}

async def researcher_handler(ctx, topic, event):
    if event.type == "research.request":
        results = search(query=event.payload.decode())
        await ctx.emit("topic.writer", type="writer.draft", payload=str(results).encode())

# Writer
async def writer_handler(ctx, topic, event):
    if event.type == "writer.draft":
        final = event.payload + b"\nSUMMARY: Complete"
        print("Final output:", final.decode())
```

### Request-Reply Pattern

```python
async def requester_handler(ctx, topic, event):
    # Send request and wait for response
    response = await ctx.request(
        "service.topic",
        type="data.query",
        payload=b"request-data",
        timeout_ms=3000,
    )
    print(f"Got response: {response.payload}")

async def responder_handler(ctx, topic, event):
    if event.type == "data.query":
        # Process and reply
        result = process_query(event.payload)
        await ctx.reply(event, type="data.result", payload=result)
```

### Tool Invocation

```python
async def agent_handler(ctx, topic, event):
    # Call another agent's capability
    weather_data = await ctx.tool(
        "weather.get",
        version="1.0",
        payload={"city": "Seattle", "units": "metric"},
    )

    result = json.loads(weather_data)
    print(f"Temperature: {result['temp']}°C")
```

### Thread Management

```python
async def conversation_agent(ctx, topic, event):
    # Extract or create thread
    thread_id = event.thread_id or f"thread-{uuid.uuid4()}"

    # All events in this thread will share the thread_id
    await ctx.emit(
        "conversation.topic",
        type="message",
        payload=b"Hello",
        envelope=Envelope.new(
            event_id=str(uuid.uuid4()),
            event_type="message",
            payload=b"Hello",
            thread_id=thread_id,
        ),
    )
```

## Best Practices

### 1. Event Naming

Use hierarchical topic names:

```python
topics = [
    "app.service.action",  # Good
    "weather.forecast.get",
    "user.auth.login",
]
```

### 2. Error Handling

Always handle exceptions in event handlers:

```python
async def safe_handler(ctx, topic, event):
    try:
        result = await process_event(event)
        await ctx.reply(event, type="success", payload=result)
    except ValueError as e:
        await ctx.reply(event, type="error", payload=str(e).encode())
    except Exception as e:
        logger.error(f"Unexpected error: {e}")
        await ctx.reply(event, type="error", payload=b"Internal error")
```

### 3. Graceful Shutdown

Use signal handlers for clean shutdown:

```python
import signal
import asyncio

agent = Agent(...)

def shutdown_handler(signum, frame):
    asyncio.create_task(agent.stop())

signal.signal(signal.SIGINT, shutdown_handler)
signal.signal(signal.SIGTERM, shutdown_handler)

agent.run()
```

### 4. Type Hints

Always use type hints for capabilities:

```python
@capability("process.data")
def process(
    data: str,
    max_length: int = 100,
    format: str = "json",
) -> dict:
    """Type hints enable automatic schema generation."""
    return {"processed": data[:max_length], "format": format}
```

### 5. Testing

Use mocks for unit testing agents:

```python
from unittest.mock import Mock, AsyncMock

async def test_agent_handler():
    ctx = Mock()
    ctx.emit = AsyncMock()
    ctx.reply = AsyncMock()

    event = Mock()
    event.type = "test.event"
    event.payload = b"test data"

    await my_handler(ctx, "test.topic", event)

    ctx.emit.assert_called_once()
```

## Troubleshooting

### Connection Issues

**Problem:** Agent can't connect to bridge

**Solutions:**

1. Check bridge is running: `cargo run -p loom-bridge --bin loom-bridge-server`
2. Verify address: `echo $LOOM_BRIDGE_ADDR`
3. Check firewall/network settings
4. Enable debug logging: `export LOOM_LOG=debug`

### Proto Generation

**Problem:** Import errors for `loom.proto.generated`

**Solution:**

```bash
cd loom-py
python -m loom.proto.generate
```

### Type Checking

**Problem:** Mypy errors in your code

**Solutions:**

1. Add type hints to all functions
2. Use `# type: ignore` for generated proto code
3. Update mypy config in `pyproject.toml`

### Event Not Received

**Problem:** Agent doesn't receive events

**Checklist:**

1. ✅ Agent subscribed to correct topic
2. ✅ Bridge server running
3. ✅ Event emitted to correct topic
4. ✅ Topic names match exactly (case-sensitive)
5. ✅ Agent started with `await agent.start()`

### Performance

**Tips for high-throughput scenarios:**

1. Use QoS levels appropriately (Realtime vs Batched)
2. Batch operations where possible
3. Monitor backpressure in bridge logs
4. Scale horizontally with multiple agent instances

## Advanced Topics

### Custom Event Loop

```python
import asyncio

async def main():
    agent = Agent(...)
    await agent.start()

    # Custom logic
    await asyncio.sleep(10)

    await agent.stop()

asyncio.run(main())
```

### Dynamic Topic Subscription

```python
# Future feature - not in MVP
async def subscribe_dynamically(ctx):
    await ctx.subscribe("new.topic")
```

### Memory Integration

```python
# Future feature - not in MVP
from loom import Memory

memory = Memory(backend="redis", url="redis://localhost")
await memory.set("key", "value")
value = await memory.get("key")
```

## Contributing

See [CONTRIBUTING.md](../CONTRIBUTING.md) for guidelines.

## License

Apache 2.0 - See [LICENSE](LICENSE)
