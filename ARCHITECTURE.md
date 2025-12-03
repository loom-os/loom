# Loom Architecture

An event-driven AI agent runtime — the operating layer for long-lifecycle, desktop/edge AI agents.

## Design Philosophy

**Loom is not another LangChain.** It's a runtime that enables AI agents to:

- **Run continuously** as background services (not one-shot scripts)
- **Respond to events** from the system (hotkeys, file changes, clipboard, timers)
- **Collaborate** via event-driven pub/sub (not function calls)
- **Persist state** across sessions (long-term memory)
- **Integrate deeply** with the desktop/edge environment

### The Brain/Hand Separation

```
┌─────────────────────────────────────────────────────────────────────┐
│                     Python Agent (Brain 🧠)                         │
│                                                                     │
│   "Thinking" - needs rapid iteration                                │
│   ┌─────────────────────────────────────────────────────────────┐  │
│   │ • LLM Calls (direct HTTP)      - prompt engineering         │  │
│   │ • Cognitive Loop (ReAct/CoT)   - strategy experiments       │  │
│   │ • Context Engineering          - retrieval, ranking         │  │
│   │ • Memory Management            - what to remember           │  │
│   │ • Business Logic               - agent-specific behavior    │  │
│   └─────────────────────────────────────────────────────────────┘  │
│                              │                                      │
│                              │ ctx.tool("xxx", {...})               │
│                              ▼                                      │
└──────────────────────────────┼──────────────────────────────────────┘
                               │ gRPC
                               ▼
┌──────────────────────────────────────────────────────────────────────┐
│                     Rust Core Runtime (Hands 🤚)                     │
│                                                                      │
│   "Execution" - stable infrastructure                                │
│   ┌──────────────────────────────────────────────────────────────┐  │
│   │ • Event Bus           - agent communication, QoS             │  │
│   │ • Tool Registry       - tool execution, sandboxing           │  │
│   │ • Agent Lifecycle     - register, heartbeat, restart         │  │
│   │ • Persistent Store    - RocksDB for long-term memory         │  │
│   │ • System Integration  - files, hotkeys, clipboard, notify    │  │
│   │ • MCP Proxy           - external tool servers                │  │
│   │ • Telemetry           - tracing, metrics, dashboard          │  │
│   └──────────────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────────────┘
```

**Why this separation?**

| Aspect               | Brain (Python)             | Hands (Rust Core)          |
| -------------------- | -------------------------- | -------------------------- |
| Change frequency     | High (daily prompt tuning) | Low (stable APIs)          |
| Experimentation      | High (A/B test strategies) | Low (fixed behavior)       |
| Debug needs          | High (print intermediate)  | Low (logs sufficient)      |
| Performance critical | No (LLM is bottleneck)     | Yes (tool execution)       |
| Security critical    | No                         | Yes (sandbox, permissions) |

## Crate Layout

```
loom/
├── loom-proto      # Protobuf definitions + generated Rust code
├── core            # Runtime: EventBus, Tools, Agent Lifecycle, Telemetry
├── bridge          # gRPC service for Python/JS agents
├── loom-py         # Python SDK: Agent, CognitiveAgent, LLMProvider
├── loom-audio      # Audio stack (VAD, STT, TTS) for desktop agents
└── loom-dashboard  # (planned) Standalone observability UI
```

Dependency flow:

```
loom-proto ──▶ core ──▶ bridge
                │
                └──▶ loom-audio (optional, desktop only)

loom-py (standalone, connects via gRPC)
```

## Rust Core: What It Does

### Event Bus

Async pub/sub with QoS for agent communication:

| QoS        | Use Case            | Behavior                 |
| ---------- | ------------------- | ------------------------ |
| Realtime   | Low latency         | Drops under pressure     |
| Batched    | Guaranteed delivery | Queues with backpressure |
| Background | Delay-tolerant      | Large buffer             |

```rust
// Agents communicate via topics
bus.publish("market.alert", event).await?;
bus.subscribe("market.alert").await?;

// Topic patterns
"agent.{id}.replies"      // Agent-specific
"thread.{id}.broadcast"   // Collaboration
"market.price.*"          // Wildcard
```

### Tool Registry

Unified tool execution with sandboxing:

```rust
// Register tools
registry.register(WeatherTool::new());
registry.register(ShellTool::new().with_allowlist(["ls", "cat"]));
registry.register_mcp("brave-search", mcp_client);

// Execute (called from Python via gRPC)
let result = registry.invoke("weather:get", args).await?;
```

**Built-in tools**:

- `fs:read`, `fs:write`, `fs:list` — File operations
- `system:shell` — Sandboxed shell execution
- `weather:get` — Weather information

**MCP Integration**: Connect external tool servers (Brave Search, databases, etc.)

### Agent Lifecycle

Long-running agent management:

```rust
// Agents register via Bridge
runtime.register_agent(agent_id, topics, capabilities).await?;

// Lifecycle management
- Heartbeat monitoring
- Automatic restart on crash
- Graceful shutdown
- State persistence
```

### Persistent Store

RocksDB-backed storage for long-term memory:

```rust
// Store context across sessions
store.save_context(session_id, items).await?;
store.query_context(session_id, query).await?;
```

### System Integration (Desktop Agents)

For desktop/edge deployment:

- **File monitoring** — Watch directories for changes
- **Hotkeys** — Global keyboard shortcuts
- **Clipboard** — Monitor and modify clipboard
- **Notifications** — System notifications
- **System tray** — Background presence

### Telemetry

OpenTelemetry integration:

- Distributed tracing across agents
- Metrics (events/sec, latency, tool calls)
- Dashboard visualization

## Python SDK (loom-py): What It Does

### Agent Connection

```python
from loom import Agent

agent = Agent(
    agent_id="my-agent",
    topics=["input.query"],
    address="127.0.0.1:50051"
)
await agent.start()
```

### Cognitive Loop

```python
from loom import CognitiveAgent, CognitiveConfig, ThinkingStrategy

cognitive = CognitiveAgent(
    ctx=agent._ctx,
    llm=LLMProvider.from_config(ctx, "deepseek", config),
    config=CognitiveConfig(
        system_prompt="You are a helpful assistant...",
        thinking_strategy=ThinkingStrategy.REACT,
        max_iterations=10,
    ),
    available_tools=["weather:get", "fs:read"],
)

result = await cognitive.run("What's the weather in Tokyo?")
```

### LLM Provider (Direct HTTP)

```python
from loom import LLMProvider

# Direct HTTP call to LLM API (not through Rust Core)
llm = LLMProvider.from_config(ctx, "deepseek", project_config)
response = await llm.generate(
    prompt="Hello",
    system="You are helpful",
    temperature=0.7,
)
```

### Tool Invocation (via Rust Core)

```python
# Tools execute in Rust Core (sandboxed)
result = await ctx.tool("weather:get", {"location": "Tokyo"})
result = await ctx.tool("fs:read", {"path": "data.txt"})
```

### Context Engineering (Python-side)

```python
# Memory management in Python
cognitive.memory.add("user", "What's the weather?")
cognitive.memory.add("assistant", "Let me check...")

# Context assembly before LLM call
context = cognitive.memory.get_recent(10)
```

## What Rust Core Does NOT Do

The following are **intentionally NOT in Rust Core**:

| Component        | Why Not in Rust             | Where It Lives          |
| ---------------- | --------------------------- | ----------------------- |
| LLM API calls    | Need rapid prompt iteration | Python `LLMProvider`    |
| Cognitive Loop   | Strategy experimentation    | Python `CognitiveAgent` |
| Context ranking  | Algorithm tuning            | Python (planned)        |
| Prompt templates | Frequent changes            | Python/Agent code       |
| Business logic   | Domain-specific             | Python/Agent code       |

## Bridge Protocol

gRPC service connecting Python agents to Rust Core:

```protobuf
service Bridge {
  rpc RegisterAgent(RegisterRequest) returns (RegisterResponse);
  rpc EventStream(stream ClientEvent) returns (stream ServerEvent);
  rpc ForwardToolCall(ToolCall) returns (ToolResult);
  rpc Heartbeat(HeartbeatRequest) returns (HeartbeatResponse);
}
```

## Use Cases

### Server-side Agent (Market Analyst)

```
Python Agent                         Rust Core
┌────────────────────┐              ┌────────────────────────┐
│ • Cognitive Loop   │              │ • Event Bus            │
│ • Trading Logic    │ ─ gRPC ───▶  │ • Tool Execution       │
│ • LLM Calls        │              │ • Agent Lifecycle      │
│ • Analysis         │ ◀──────────  │ • Persistent Memory    │
└────────────────────┘   tool result │ • Telemetry           │
                                     └────────────────────────┘
```

### Desktop Agent (Personal Assistant)

```
Python Agent                         Rust Core
┌────────────────────┐              ┌────────────────────────┐
│ • Cognitive Loop   │              │ • Event Bus            │
│ • User Intent      │ ─ gRPC ───▶  │ • Tool Execution       │
│ • LLM Calls        │              │ • System Integration   │
│                    │ ◀──────────  │   - Hotkeys            │
└────────────────────┘   events     │   - File Watch         │
                                     │   - Clipboard          │
                                     │   - Notifications      │
                                     └────────────────────────┘
```

## Comparison with Alternatives

|                     | LangChain        | CrewAI           | Loom                             |
| ------------------- | ---------------- | ---------------- | -------------------------------- |
| Nature              | Library          | Library          | **Runtime**                      |
| Lifecycle           | Script execution | Script execution | **Long-running service**         |
| Trigger             | Code call        | Code call        | **Events (hotkey, file, timer)** |
| Agent comm          | In-process       | In-process       | **Event Bus (cross-process)**    |
| Tool safety         | None             | None             | **Sandbox**                      |
| Desktop integration | None             | None             | **Native**                       |
| Language            | Python only      | Python only      | **Polyglot**                     |

## Future Work

- [ ] `loom-dashboard` — Standalone observability UI (extract from core)
- [ ] WebSocket transport for Bridge
- [ ] Semantic retrieval for context (vector similarity)
- [ ] WASM plugin support for tools
- [ ] Mobile/edge packaging (iOS/Android)

---

_Loom — Weaving Intelligence into the Fabric of Reality_
