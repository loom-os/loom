# Agent Module

Core agent functionality for connecting Python agents to Loom Runtime via Bridge.

## Overview

This module provides the foundational classes for building event-driven agents in Python:

- **Agent**: Main class for agent lifecycle (connect, register, handle events)
- **EventContext**: Agent's interface to Rust Core Event Bus (publish, subscribe, tool calls)
- **Envelope**: Message wrapper for event communication with tracing support

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                     Python Agent                        │
│  ┌────────────┐        ┌──────────────┐                │
│  │   Agent    │───────▶│ EventContext │                │
│  │  (base.py) │        │  (event.py)  │                │
│  └────────────┘        └──────────────┘                │
│        │                       │                        │
│        │                       │ emit(), request()      │
│        │                       │ tool(), save_plan()    │
└────────┼───────────────────────┼────────────────────────┘
         │                       │
         │ gRPC Stream           │ gRPC Calls
         ▼                       ▼
┌─────────────────────────────────────────────────────────┐
│              Rust Core (via Bridge)                     │
│  ┌─────────────┐  ┌─────────────┐  ┌──────────────┐   │
│  │  Event Bus  │  │ Tool System │  │ Memory Store │   │
│  └─────────────┘  └─────────────┘  └──────────────┘   │
└─────────────────────────────────────────────────────────┘
```

## Key Components

### Agent (`base.py`)

The main agent class that manages:
- **Connection**: Establishes gRPC stream to Bridge
- **Registration**: Registers with Core (agent_id, topics, tools)
- **Event Loop**: Processes incoming events and deliveries
- **Tool Declaration**: Exposes Python functions as callable tools
- **Heartbeat**: Maintains connection with periodic heartbeats

**Example:**
```python
from loom import Agent, tool

@tool("hello.greet", description="Greet someone")
def greet(name: str) -> dict:
    return {"message": f"Hello, {name}!"}

async def on_event(ctx, topic, envelope):
    print(f"Received event: {envelope.type} on {topic}")
    await ctx.emit(topic, type="ack", payload=b"ok")

agent = Agent(
    agent_id="greeter",
    topics=["greetings"],
    tools=[greet],
    on_event=on_event,
)

await agent.start()  # Connect and run
```

### EventContext (`event.py`)

Agent's interface to Rust Core's Event Bus. Provides:

- **Event Publishing**: `emit()` - Fire-and-forget event publishing
- **Request/Reply**: `request()` - Send request and wait for reply
- **Tool Invocation**: `tool()` - Call Rust tools via Bridge
- **Memory Operations**: `save_plan()`, `get_recent_plans()`, etc.

**Event Communication:**
```python
# Publish event
await ctx.emit("research.tasks", type="task.created", payload=b"...")

# Request-reply pattern
reply = await ctx.request(
    "workers.pool",
    type="job.submit",
    payload=b"...",
    timeout_ms=5000
)

# Invoke tool
result = await ctx.tool(
    "web:search",
    payload={"query": "Loom framework"},
    timeout_ms=10000
)
```

**Memory Operations:**
```python
# Save trading plan (deduplication)
plan_hash = await ctx.save_plan(
    symbol="BTC",
    action="BUY",
    confidence=0.85,
    reasoning="Strong uptrend detected",
)

# Check for duplicates
is_dup, dup_plan = await ctx.check_duplicate_plan(
    symbol="BTC",
    action="BUY",
    reasoning="Strong uptrend detected",
    time_window_sec=300,  # 5 minutes
)

# Get recent plans
plans = await ctx.get_recent_plans(symbol="BTC", limit=10)
```

### Envelope (`envelope.py`)

Message wrapper that provides:
- **Unique IDs**: Every event has a unique identifier
- **Thread Correlation**: Link related events via `thread_id` and `correlation_id`
- **Trace Context**: Inject/extract OpenTelemetry trace context for distributed tracing
- **Serialization**: Convert to/from protobuf for gRPC

**Trace Propagation:**
```python
# Automatically inject current span context
env = Envelope.new(type="task.start", payload=b"work")
env.inject_trace_context()  # Adds traceparent/tracestate

await ctx.emit("tasks", envelope=env)

# On receiver side, extract and continue trace
env = Envelope.from_proto(proto_event)
span_context = env.extract_trace_context()
# Use span_context to link distributed traces
```

## Design Principles

### 1. **Event-Driven Communication**

All inter-agent and agent-core communication happens via events on the Event Bus:

```python
# Producer
await ctx.emit("market.signals", type="signal.detected", payload=data)

# Consumer (in on_event handler)
async def on_event(ctx, topic, envelope):
    if envelope.type == "signal.detected":
        # Process signal
        await ctx.emit("trades.orders", type="order.placed", ...)
```

### 2. **Separation of Concerns**

- **Agent** (`base.py`): Lifecycle, connection management, event routing
- **EventContext** (`event.py`): Communication primitives, tool invocation
- **Envelope** (`envelope.py`): Message format, tracing

### 3. **Bridge Abstraction**

Python agents never directly interact with Rust Core. All communication goes through:
1. **EventContext** → **BridgeClient** (gRPC)
2. **Bridge** (Rust service) → **Core** (event bus, tools, memory)

This allows:
- **Language Independence**: Agents in any language can use the same Bridge
- **Network Transparency**: Bridge can run locally or remotely
- **Clean Boundaries**: Python doesn't need to understand Core internals

## Integration with Other Modules

### Cognitive Module

`CognitiveAgent` wraps `EventContext` for LLM-powered reasoning:

```python
from loom import Agent
from loom.cognitive import CognitiveAgent, CognitiveConfig
from loom.llm import LLMProvider

agent = Agent(agent_id="researcher", topics=["research.tasks"])
await agent.start()

cognitive = CognitiveAgent(
    ctx=agent.ctx,  # ← EventContext from Agent
    llm=LLMProvider.from_name(agent.ctx, "deepseek"),
    config=CognitiveConfig(max_iterations=5),
)

result = await cognitive.run("Research AI frameworks")
```

### LLM Module

LLM providers use `EventContext` for tool invocation during reasoning:

```python
llm = LLMProvider(ctx=agent.ctx, config=LLMConfig(...))
# LLM can now call tools via ctx.tool() during generation
```

## Testing

```bash
# Run agent tests
pytest tests/unit/test_context.py -v

# Run integration tests
pytest tests/integration/test_agent.py -v
```

## Related Documentation

- **[ARCHITECTURE.md](../../../ARCHITECTURE.md)**: Overall system design
- **[Bridge Documentation](../bridge/README.md)**: gRPC communication layer
- **[Event Bus](../../../docs/core/event_bus.md)**: Core event system
- **[Cognitive Module](../cognitive/README.md)**: LLM-powered agents

---

**Key Insight**: The agent module is the "Hand" in Loom's Brain/Hand separation - it handles communication and tool execution, while the Brain (cognitive module) handles reasoning and decision-making.
