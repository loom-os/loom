# Loom Architecture

An event-driven AI agent runtime built in Rust.

## Crate Layout

```
loom/
├── loom-proto      # Protobuf definitions + generated Rust code
├── core            # Main runtime (loom-core)
├── loom-audio      # Audio capabilities (VAD, STT, TTS)
├── bridge          # gRPC service for external agents
└── loom-py         # Python SDK
```

Dependencies flow:

```
loom-proto ──▶ core ──▶ loom-audio (optional)
    │           │
    └───────────┴──▶ bridge ──▶ loom-py
```

## Core Module Structure

```
core/src/
├── agent/           # Agent definitions and lifecycle
│   ├── behavior.rs    # AgentBehavior trait
│   ├── directory.rs   # Agent/capability discovery
│   ├── instance.rs    # Running agent with mailbox
│   └── runtime.rs     # Agent lifecycle manager
│
├── cognitive/       # LLM-powered reasoning (Perceive-Think-Act)
│   ├── llm/           # LLM client, router, providers
│   ├── simple_loop.rs # Main cognitive loop implementation
│   ├── thought.rs     # Plan, ToolCall, Observation types
│   └── config.rs      # Thinking strategies
│
├── context/         # Context Engineering system
│   ├── agent_context.rs  # High-level API for agents
│   ├── memory/           # Storage backends
│   ├── retrieval/        # Retrieval strategies
│   ├── ranking/          # Context ranking
│   ├── window/           # Token budget management
│   └── pipeline/         # Context orchestration
│
├── tools/           # Unified tool system
│   ├── registry.rs    # Tool registration and invocation
│   ├── traits.rs      # Tool trait definition
│   ├── native/        # Built-in tools (shell, file, weather)
│   └── mcp/           # Model Context Protocol client
│
├── event.rs         # EventBus with QoS levels
├── envelope.rs      # Thread/correlation metadata
├── collab.rs        # Multi-agent collaboration primitives
├── dashboard/       # Real-time visualization
└── telemetry.rs     # OpenTelemetry tracing
```

## Design Principles

1. **Event-First** — All inputs modeled as events, not RPC calls
2. **Stateful Agents** — Persistent state + ephemeral context
3. **Composable Tools** — Native, MCP, and remote tools via unified registry
4. **Observable** — Built-in tracing, metrics, and logging

## Key Components

### EventBus

Async pub/sub with three QoS levels:

| QoS        | Use Case            | Behavior                 |
| ---------- | ------------------- | ------------------------ |
| Realtime   | Low latency         | Drops under pressure     |
| Batched    | Guaranteed delivery | Queues with backpressure |
| Background | Delay-tolerant      | Large buffer, no drops   |

Topic patterns:

```
agent.{id}.intent     # Agent-specific
thread.{id}.broadcast # Collaboration broadcast
thread.{id}.reply     # Correlated replies
market.price.*        # Wildcard subscription
```

### Agent System

```rust
// Simple event-driven agent
impl AgentBehavior for MyAgent {
    async fn on_event(&mut self, event: Event, state: &mut AgentState) -> Result<Vec<Action>> {
        // Handle event, return actions
    }
}

// LLM-powered cognitive agent
let loop_impl = SimpleCognitiveLoop::new(config, llm, tools)
    .with_context(AgentContext::with_defaults(session, agent_id));
let behavior = CognitiveAgent::new(loop_impl);
```

**AgentRuntime** manages lifecycle: create → start → stop → delete

**AgentDirectory** enables discovery by ID, topic, or capability

### Cognitive Loop

Perceive-Think-Act pattern with LLM integration:

```
Event ──▶ [PERCEIVE] ──▶ [THINK] ──▶ [ACT] ──▶ Actions
              │            │          │
              ▼            ▼          ▼
         AgentContext   LLM+Router  ToolRegistry
```

**ThinkingStrategy**:

- `SingleShot` — One LLM call, no tools
- `ReAct` — Iterative reasoning with tool use
- `ChainOfThought` — Multi-step reasoning

### Context Engineering

`AgentContext` provides a clean API for context management:

```rust
let ctx = AgentContext::with_defaults("session-1", "agent-1");

// Record interactions
ctx.record_message(MessageRole::User, "Hello").await?;
ctx.record_tool_call("search", json!({"q": "news"})).await?;
ctx.record_tool_result("search", true, result, call_id).await?;

// Retrieve context for LLM
let bundle = ctx.get_context(trigger).await?;
```

**Pipeline**: Retrieval → Ranking → Windowing → PromptBundle

**Storage**: InMemoryStore (dev), RocksDbStore (prod, TODO)

### Tool System

Unified interface for all tools:

```rust
#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> String;
    fn description(&self) -> String;
    fn schema(&self) -> Value;
    async fn execute(&self, args: Value) -> ToolResult;
}
```

**ToolRegistry** provides:

- `register(tool)` — Register a tool
- `invoke(name, args)` — Execute with 30s timeout
- `list_tools()` — Get all registered tools

**Built-in Tools**:

- `shell.exec` — Execute shell commands (sandboxed)
- `fs.read` — Read files from workspace
- `weather.get` — Weather information
- `web.search` — Web search

**MCP Integration**: Connect external tool servers via stdio transport

### Model Router

Routes LLM requests based on:

- Privacy policy (local-only, sensitive, public)
- Model capabilities (local vs cloud)
- Cost and latency constraints

### Collaboration

Built on Envelope metadata for multi-agent workflows:

| Pattern       | Description                         |
| ------------- | ----------------------------------- |
| Request/Reply | Correlated request-response         |
| Fanout/Fanin  | Broadcast, collect with strategies  |
| Barrier       | Wait for N agents before proceeding |
| Contract-Net  | Call for proposals, award, execute  |

### Bridge (gRPC)

Cross-process communication for external agents:

- `RegisterAgent` — Register with topics and capabilities
- `EventStream` — Bidirectional event streaming
- `ForwardAction` — Invoke tools from external agents
- `Heartbeat` — Connection health monitoring

### Python SDK

```python
from loom import Agent, Context

@capability("my.tool")
def my_tool(query: str) -> dict:
    return {"result": "..."}

async def on_event(ctx: Context, topic: str, envelope: Envelope):
    await ctx.emit("output.topic", payload=b"data")

agent = Agent("my-agent", topics=["input.*"], capabilities=[my_tool])
await agent.start()
```

## Deprecated Modules

| Module                        | Replacement                | Notes                               |
| ----------------------------- | -------------------------- | ----------------------------------- |
| `plugin.rs`                   | `tools/`                   | Use `Tool` trait and `ToolRegistry` |
| `storage.rs`                  | `context/memory/`          | Use `MemoryStore` trait             |
| `cognitive/working_memory.rs` | `context/agent_context.rs` | Use `AgentContext`                  |

## Telemetry

OpenTelemetry integration with:

- Distributed tracing across agents
- Metrics for event bus, agent runtime, tools
- Span collection for dashboard visualization

```rust
// Initialize before creating Loom
let span_collector = init_telemetry()?;
let loom = Loom::new().await?;
```

## Future Work

- [ ] Persistent context storage (RocksDB backend)
- [ ] SSE transport for MCP
- [ ] WebSocket transport for Bridge
- [ ] WASM plugin support
- [ ] Semantic retrieval for context
