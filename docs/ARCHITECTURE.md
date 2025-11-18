# Loom Architecture

This document reflects the current repository shape, centered on the first end‑to‑end Voice Agent demo and a modular, parallel crate layout.

## Crate relationships (parallel layout)

```
loom-proto   ──▶   core (loom-core)
  │                  │
  └──────────────────┼──▶ loom-audio (optional)
  │                  │
  └──────────────────┴──▶ bridge (optional)
  │
  └──────────────────────▶ loom-py (Python SDK)

apps / demos (e.g., demo/voice_agent) ──▶ depend on core and optionally loom-audio
```

- `loom-proto` contains only protobuf definitions and generated Rust. `protoc` is vendored; no system install is required.
- `core` depends on `loom-proto` and implements the runtime (Event Bus, Agent Runtime, Router, LLM client, ActionBroker, Plugin Manager, MCP Client, Collaboration, Directories, Envelope). It intentionally does not depend on `loom-audio`.
- `loom-audio` is a capability provider set with mic/VAD/STT/wake/TTS and depends on both `loom-proto` and `core`. Applications can opt‑in to audio features.
- `bridge` is a gRPC service for forwarding events and actions across process or network boundaries (e.g., to Python/JS agents, mobile clients, or web workers) using the shared proto contracts. Supports RegisterAgent, bidirectional EventStream, ForwardAction, and Heartbeat.
- `loom-py` provides Python bindings and Agent/Context API for writing agents in Python. Includes @capability decorator, Envelope support, and examples (trio.py). Communicates with Loom core via the bridge service.

## Overview

Loom is an event-driven AI operating system that models intelligent agents as **stateful event-responsive entities**.

## Why “OS” (five core traits):

- Event‑driven resource scheduling (Event Bus + Router)
- Stateful runtime (Agent Runtime)
- Device/model abstraction (Plugins + Model Router)
- Observability & policy management
- General action interface (Action System)

## Core Design Principles

1. **Event-First**: All inputs modeled as events, not synchronous calls
2. **Async-First**: Fully asynchronous using Tokio runtime
3. **Stateful**: Agents maintain persistent state and ephemeral context
4. **Composable**: Extensible through plugins and tools
5. **Observable**: Built-in tracing, metrics, and logging

## System Layers

```
┌─────────────────────────────────────┐
│     Application Layer            │  examples/, demo/
│  Demo Apps, Custom Agents           │
├─────────────────────────────────────┤
│     Plugin Layer                    │  plugins/
│  Feature Extractors, Models, Tools  │
├─────────────────────────────────────┤
│     Runtime Layer                   │  core/
│  Event Bus, Agent Runtime, Router, Collaboration, Directories, Envelope │
├─────────────────────────────────────┤
│     Infrastructure Layer            │  infra/, bridge/
│  Storage, Network, Telemetry, Bridge│
└─────────────────────────────────────┘
```

### Case Study: The Market Analyst Demo

The `demo/market-analyst` application serves as a comprehensive showcase of the Loom architecture in action, demonstrating a sophisticated multi-agent system for real-time market analysis and automated trading. It consists of five distinct Python agents collaborating to:

1.  **Data Agent**: Ingests real-time market data from the OKX WebSocket API and publishes it onto the event bus (e.g., `market.price.BTC-USDT-SWAP`).
2.  **Analysis Agents (Risk, Trend, Sentiment)**: These agents subscribe to price events using wildcard topics (`market.price.*`). They perform parallel analysis:
    - **Sentiment Agent**: Uses an MCP `web-search` tool to gather news and determine market sentiment.
    - **Trend Agent**: Analyzes price history to identify short-term trends.
    - **Risk Agent**: Calculates risk metrics based on volatility.
3.  **Planner Agent**: Subscribes to all `analysis.*` topics. It aggregates the inputs from the analysis agents and uses an LLM (via the `llm.generate` capability) to formulate a trading plan (e.g., "BUY BTC at market price"). Uses **memory system** to:
    - Query recent plans for context (avoid similar decisions)
    - Check for duplicate plans within 5-minute time window
    - Save new plans with hash-based deduplication
4.  **Executor Agent**: Subscribes to `plan.ready` events. It parses the plan and executes trades by calling the `okx.place_order` capability, which is a native Rust capability for interacting with the exchange's private API. Uses **memory system** to:
    - Check execution idempotency (prevent double-execution on retry)
    - Track execution status (success/error) with order details
    - Calculate and display win rate statistics

This demo validates several core architectural principles:

- **Event-First Collaboration**: Agents communicate exclusively through the event bus, ensuring loose coupling.
- **Polyglot Agents**: A Rust core orchestrates Python agents via the gRPC Bridge.
- **Capability Abstraction**: LLM, web search (MCP), and trading APIs are all exposed as uniform "tools" or "capabilities" through the ActionBroker.
- **Memory-Aware Decision Making**: Planner and Executor use shared memory to prevent duplicate decisions and double-execution, ensuring idempotent trading operations.
- **Observability**: The entire workflow, from data ingestion to trade execution, is captured in a single distributed trace, providing deep visibility into the system's behavior.

## Core Components

### Event Bus

Asynchronous pub/sub message system with:

- **QoS Levels**:

  - `Realtime`: Low latency, best-effort delivery
  - `Batched`: Batch processing, guaranteed delivery
  - `Background`: Background tasks, delay-tolerant

- **Topic Routing**:

  ```
  camera.front.face      → Front camera face detection
  mic.primary.speech     → Primary microphone speech
  sensor.imu.motion      → IMU motion events
  agent.{id}.intent      → Agent intent events
  thread.{id}.broadcast  → Thread broadcast (collaboration)
  thread.{id}.reply      → Thread replies (correlation)
  ```

- **Backpressure Handling**: Sampling, dropping old events, aggregation

- **Event Structure**:
  ```rust
  Event {
      id: String,
      type: String,
      timestamp_ms: i64,
      source: String,
      metadata: Map<String>,
      payload: Bytes,
      confidence: f32,
      tags: Vec<String>,
      priority: i32,
  }
  ```

**Envelope (thread/correlation)**: see `docs/core/envelope.md` for the reserved metadata keys (`thread_id`, `correlation_id`, `sender`, `reply_to`, `ttl`, `hop`, `ts`) and topic conventions (`thread.{id}.broadcast/reply`). Agents automatically maintain TTL/hop and drop expired events. The Envelope provides the foundation for multi-agent collaboration patterns.

### Collaboration & Directories

On top of the Envelope, Loom provides collaboration primitives (request/reply, fanout/fanin, contract-net) for multi-agent workflows, and directories to discover agents and capabilities. See `docs/core/collaboration.md` and `docs/core/directory.md`.

**Collaboration Primitives (✅ IMPLEMENTED)**:

- **request/reply**: Send request with correlation_id, wait for reply on thread reply topic
- **fanout/fanin**: Broadcast to multiple agents, collect replies with strategies:
  - `any`: Return first reply
  - `first_k(n)`: Return first n replies
  - `majority`: Wait for majority consensus
  - `timeout(ms)`: Collect replies within timeout
- **barrier**: Wait for N agents to check in before proceeding
- **contract-net**: Call for proposals, collect bids, award contract, execute task

**Directories (✅ IMPLEMENTED)**:

- **AgentDirectory**: Discover agents by id, topics, or capabilities; auto-registers on agent creation
- **CapabilityDirectory**: Snapshot of all registered capability providers from ActionBroker; query by name or provider type

### MCP (Model Context Protocol) Client

**✅ FULLY IMPLEMENTED** — Loom can now connect to MCP servers and use their tools as native capabilities:

- **McpClient**: Low-level JSON-RPC 2.0 client over stdio transport
  - Initialize handshake with protocol version (configurable, default: 2024-11-05)
  - list_tools with pagination support
  - call_tool with timeout and comprehensive error handling
  - Background reader task for response correlation
- **McpToolAdapter**: Implements `CapabilityProvider` trait
  - Adapts MCP tools to ActionBroker interface
  - Qualified naming (server:tool) to avoid conflicts
  - Automatic JSON schema extraction for tool metadata
  - Error code mapping (INVALID_PARAMS, TIMEOUT, TOOL_ERROR, etc.)
- **McpManager**: Lifecycle management for multiple MCP servers
  - add_server/remove_server with validation
  - Auto-discovery and registration of tools
  - Graceful shutdown with connection cleanup
  - Protocol version validation

**Configuration**: See `docs/MCP.md` for configuration examples and `core/src/mcp/README.md` for developer documentation.

**Future Enhancements** (P1):

- SSE transport (HTTP-based) in addition to stdio
- Resources API (read/write/list resources)
- Prompts API (list/get prompts with arguments)
- Sampling support for multi-turn tool use
- Notifications support

### Agent Runtime

Actor-based stateful agents with:

- **Lifecycle Management**: Create, start, stop, delete agents
- **Dynamic Subscription**: Subscribe/unsubscribe from topics at runtime
- **State Persistence**: RocksDB for long-term state
- **Event Distribution**: Mailbox pattern for event delivery
- **Behavior Execution**: Async event handlers

**Agent Model**:

```rust
Agent {
    config: AgentConfig,
    state: {
        persistent_state: Bytes,   // Persisted in RocksDB
        ephemeral_context: Bytes,  // In-memory sliding window
        last_update_ms: i64,
    },
    behavior: AgentBehavior,
    mailbox: mpsc::Receiver,
}
```

**Dynamic Subscription API** (enables multi-agent collaboration):

```rust
// Agent joins thread mid-conversation
runtime.subscribe_agent("agent-1", "thread.task-123.broadcast").await?;

// Agent leaves when done
runtime.unsubscribe_agent("agent-1", "thread.task-123.broadcast").await?;

// Check active subscriptions
let subs = runtime.get_agent_subscriptions("agent-1")?;
```

### Memory & Context System

**✅ IMPLEMENTED** — Dual-layer memory system supporting both general-purpose episodic memory and specialized agent decision tracking.

**Architecture**:

```rust
InMemoryMemory {
    // General-purpose episodic memory
    events: DashMap<String, Vec<String>>,       // session_id → events
    summaries: DashMap<String, String>,         // session_id → summary

    // Agent decision memory
    plans: DashMap<String, Vec<PlanRecord>>,    // session_id → plans (max 100)
    executed_plans: DashMap<String, ExecutionRecord>, // plan_hash → execution
}
```

**Core Traits (General-purpose)**:

- `MemoryWriter`: append_event(), summarize_episode()
- `MemoryReader`: retrieve(query, k, filters)
- Used by `ContextBuilder` to assemble LLM-ready `PromptBundle` with recent summaries and retrieved context

**Agent Decision API (Extended)**:

```rust
// Plan storage with deduplication
save_plan(session_id, symbol, action, confidence, reasoning, method) -> plan_hash

// Query recent decisions
get_recent_plans(session_id, symbol, limit) -> Vec<PlanRecord>

// Duplicate detection (MD5 hash + time window)
check_duplicate(session_id, symbol, action, reasoning, time_window_sec)
    -> (bool, Option<PlanRecord>)

// Execution tracking (idempotency)
mark_executed(session_id, plan_hash, ...) -> Result<()>
check_executed(session_id, plan_hash) -> (bool, Option<ExecutionRecord>)

// Statistics
get_execution_stats(session_id, symbol) -> ExecutionStats {
    total_executions, successful_executions, failed_executions,
    win_rate, recent_executions
}
```

**Key Features**:

- **Deduplication**: MD5 hash (symbol|action|reasoning) with configurable time windows (default: 5 min)
- **Idempotency**: Execution tracking prevents duplicate order submissions (e.g., on retry)
- **Session Isolation**: All data partitioned by session_id for multi-agent safety
- **Performance**: DashMap lock-free concurrent access; O(1) execution lookups
- **Plan Limit**: FIFO eviction at 100 plans per session

**gRPC Bridge Integration**:

Exposed via `MemoryService` with 9 RPC methods:

- SavePlan, GetRecentPlans, CheckDuplicate
- MarkExecuted, CheckExecuted, GetExecutionStats
- Store, Retrieve, Summarize (legacy general-purpose)

**Use Case: Market Analyst Demo**:

- **Planner Agent**:
  - Queries recent_plans for context
  - Checks duplicates before saving new decisions
  - Prevents repetitive trading signals within time windows
- **Executor Agent**:
  - Checks execution idempotency before placing orders
  - Tracks execution status (success/error)
  - Reports win rate and statistics

**Testing**: 25 comprehensive tests (8 Core Rust + 5 Bridge gRPC + 12 Python SDK) — all passing ✅

See `docs/core/memory.md` for detailed implementation and `core/src/context/README.md` for API usage.

### Action System (ActionBroker + Tool Orchestrator)

**ActionBroker (✅ IMPLEMENTED)**: Unified capability registry and invocation layer

- **Capability Registration**: Providers implement `CapabilityProvider` trait
  - Native Rust providers (WeatherProvider, WebSearchProvider, LlmGenerateProvider)
  - MCP tools via McpToolAdapter
  - Custom plugins via Plugin system
- **Invocation API**:
  - `invoke(call: ActionCall) -> ActionResult`
  - Timeout handling, idempotency keys
  - Result correlation via correlation_id
- **Provider Types**:
  - `ProviderNative`: Built-in Rust capabilities
  - `ProviderMcp`: MCP server tools
  - `ProviderPlugin`: WASM/external plugins
  - `ProviderRemote`: Bridge-connected remote agents

**Tool Orchestrator (✅ IMPLEMENTED)**: Unified tool call parsing and execution

- Parse tool calls from LLM responses (function_call or structured format)
- Route to ActionBroker with appropriate provider
- Aggregate results for multi-tool scenarios
- Emit tool execution metrics (latency, success/failure)

**Error Codes** (standardized across all providers):

- `ACTION_OK`: Success
- `ACTION_ERROR`: Generic error
- `ACTION_TIMEOUT`: Execution timeout
- `INVALID_PARAMS`: Invalid tool arguments
- `CAPABILITY_ERROR`: Provider-specific error
- `PROVIDER_UNAVAILABLE`: Provider not found or offline

### Bridge (gRPC)

**✅ FULLY IMPLEMENTED** — Cross-process/network event and action forwarding:

- **RegisterAgent**: Register external agents (Python/JS) with topics and capabilities
- **EventStream**: Bidirectional streaming
  - Inbound: External agents publish events to Loom EventBus
  - Outbound: Loom pushes matching events to external agents
  - Ack-first handshake for connection confirmation
- **ForwardAction**: Client-initiated action invocation
  - External agents invoke Loom capabilities via ActionBroker
  - Result correlation and timeout handling
- **ActionCall** (server-initiated): Internal push API
  - Loom pushes action invocations to external agent capabilities
  - Result correlation map for async responses
- **Heartbeat**: Keep-alive mechanism for connection health
- **Stateless Reconnection**: Agents can reconnect and resume subscriptions

**Integration Tests** (all passing):

- registration_test.rs: Agent registration flow
- heartbeat_test.rs: Heartbeat and timeout handling
- forward_action_test.rs: Action invocation and results
- e2e integration tests: Full event/action roundtrip

**Future Enhancements** (P2):

- External admin RPC for server-initiated push
- Metrics/backpressure export via gRPC
- Authentication and namespaces
- WebSocket transport alternative

### Python SDK (loom-py)

**✅ FULLY IMPLEMENTED** — Write agents in Python, communicate via Bridge:

**Core API**:

```python
from loom import Agent, Context, capability

# Define capability
@capability("research.search", version="1.0")
def search(query: str) -> dict:
    return {"results": [...]}

# Create agent
async def on_event(ctx: Context, topic: str, envelope: Envelope):
    # Envelope provides thread/correlation metadata
    thread = envelope.thread_id
    correlation = envelope.correlation_id

    # Emit events
    await ctx.emit("target.topic", type="msg", payload=b"data")

    # Request-reply with timeout
    results = await ctx.request(thread, "topic", payload, first_k=1, timeout_ms=2000)

    # Reply in thread
    await ctx.reply(thread, {"done": True})

agent = Agent("my-agent", topics=["topic.in"], capabilities=[search], on_event=on_event)
await agent.start()  # Connects to bridge, registers, starts streaming
```

**Features**:

- **Agent/Context abstraction**: High-level API for event handling and capability invocation
- **Envelope integration**: Automatic extraction from proto Event, exposes thread/correlation metadata
- **@capability decorator**: Pydantic schema auto-generation for input validation
- **BridgeClient**: gRPC connection with automatic reconnection and heartbeat
- **Correlation tracking**: Request/reply correlation via correlation_id in headers
- **Stream resilience**: Auto-reconnect with exponential backoff (0.5s → 10s)

**Architecture**:

```
Python Agent
  └─ Agent.start()
      ├─ BridgeClient.connect() (gRPC channel)
      ├─ BridgeClient.register_agent(id, topics, capabilities)
      ├─ BridgeClient.event_stream() (bidirectional streaming)
      │   ├─ Outbound: ctx.emit() → ClientEvent.Publish
      │   └─ Inbound: ServerEvent.Delivery → on_event(ctx, topic, envelope)
      └─ Heartbeat loop (15s interval)
```

**Example**: `loom-py/examples/trio.py` — Planner/Researcher/Writer collaboration with fanout/fanin

**Packaging**: PyPI-ready with `pyproject.toml` (package name: `loom`)

**Orchestration**: CLI provides `loom run` to start core + agents in a project:

- Auto-discovers `agents/*.py` and `main.py`/`run.py`
- Manages core/bridge lifecycle
- Streams logs to `logs/` directory (optional)

**Observability**: Full OpenTelemetry integration for traces and metrics. Events sent and received by Python agents automatically participate in the distributed trace.

### Collaboration & Directories

On top of the Envelope, Loom provides collaboration primitives (request/reply, fanout/fanin, contract-net) for multi-agent workflows, and directories to discover agents and capabilities. See `docs/core/collaboration.md` and `docs/core/directory.md`.

### Model Router

Intelligent routing engine that decides where to run inference:

**Decision Algorithm**:

```
Input: Event, AgentContext, RoutingPolicy
Output: Route, Confidence, Reason

1. Check privacy policy
   if privacy == "local-only" → Local

2. Check local capability
   if not local_supported → Cloud

3. Run local quick inference
   local_confidence = local_model(event)

4. Apply threshold rules
   if local_confidence >= threshold → Local

5. Check latency budget
   if latency_budget < 100ms → Local

6. Check cost constraints
   if cloud_cost > cap → LocalFallback

7. Hybrid strategy
   if 0.5 < local_confidence < threshold → Hybrid

8. Default → Cloud
```

**Routing Strategies**:

- **Rule-Based** (current): Fast but rigid if-else rules
- **ML-Based** (future): Learned classifier using event features and performance history

**Decision Logging & Events**:

- Each routing decision is logged with: route, reason, confidence, estimated latency/cost, and the individual policy fields (privacy, latency_budget_ms, cost_cap, quality_threshold)
- An observability event `routing_decision` is published on the agent topic with the same fields for dashboards

**Policy Configuration (per agent)**:

Configure via `AgentConfig.parameters` (string map):

- `routing.privacy` = `public | sensitive | private | local-only`
- `routing.latency_budget_ms` = integer (u64)
- `routing.cost_cap` = float (f32)
- `routing.quality_threshold` = float (f32)

Hybrid two-phase execution metadata for behaviors:

- `routing_target` = `local` (quick) or `cloud` (refine)
- `phase` = `quick` or `refine`
- `refine` = `true` on refine pass

### Plugin System

Extensible plugin architecture:

**Plugin Types**:

1. **Feature Extractor**: Face detection, pose estimation, audio features
2. **Model Backend**: TFLite, ONNX, TorchScript wrappers
3. **Tool/API**: Calendar, search, database integration
4. **Actuator**: TTS, UI rendering, robot control

**Interface**:

```protobuf
service Plugin {
  rpc Init(PluginMeta) returns (Status);
  rpc HandleEvent(Event) returns (PluginResponse);
  rpc Health() returns (HealthStatus);
  rpc Shutdown() returns (Status);
}
```

**Security Isolation**:

- WASM sandboxing (recommended)
- Separate process + RPC
- Resource limits (CPU/memory/network)
- Capability declaration and authorization

### Storage Layer

**Storage Types**:

1. **KV Store (RocksDB)**: Agent state, metadata, event logs
2. **Vector DB**: Long-term memory embeddings (Milvus/FAISS/Weaviate)
3. **Object Store** (optional): Large files (video/audio) via S3/MinIO

**Data Lifecycle**:

```
Hot (memory) → 5 min → Warm (RocksDB) → 1 day → Cold (Vector DB) → 30 days → Archive/Delete
```

### Telemetry

Built-in observability with **OpenTelemetry integration** (✅ IMPLEMENTED):

**Metrics** (exported to Prometheus):

- `loom.event_bus.published_total`: Total events published (by topic, event_type)
- `loom.event_bus.delivered_total`: Total events delivered to subscribers
- `loom.event_bus.dropped_total`: Dropped events (by reason: backpressure, queue_full)
- `loom.event_bus.backlog_size`: Current backlog per topic
- `loom.event_bus.active_subscriptions`: Active subscription count
- `loom.event_bus.publish_latency_ms`: P50/P99/Max publish latency

**Tracing** (exported to Jaeger):

Core components instrumented with `#[tracing::instrument]`:

```
Span: event_bus.publish (topic, event_id, event_type)
  ├─ Span: dashboard.broadcast (event delivered to Dashboard SSE)
  └─ Span: subscription.forward (event forwarded to subscriber)

Span: agent_runtime.dispatch_event (agent_id, event_id)
  └─ Span: agent_instance.handle_event (agent_id, event_id)

Span: action_broker.invoke (capability, version, call_id)
  └─ Span: provider.execute (provider_type, timeout_ms)

Span: router.route (event_id, event_type, route, confidence)
```

**Trace Propagation**: Trace context is now fully propagated across the gRPC Bridge and through the Python SDK, enabling end-to-end distributed tracing. The `Envelope` carries W3C Trace Context headers (`traceparent`), which are automatically injected and extracted at each process boundary.

**Logging**: Structured logs via `tracing` crate (DEBUG/INFO/WARN/ERROR) with:

- Event/agent/capability metadata
- Performance metrics (latency, throughput)
- Error details with context
- No sensitive data in default log level

### Dashboard (React UI)

**✅ IMPLEMENTED** — Real-time event visualization and system observability:

**Architecture**:

- Vite-built React + shadcn/ui frontend embedded into `loom-core` via `include_dir`
- Backend: Axum HTTP server serving static assets + REST/SSE APIs
- Build artifacts: `core/src/dashboard/static/` (auto-generated via `npm run build`)

**API Endpoints**:

- `GET /api/events/stream` (SSE): Real-time event stream broadcast
  - Events: EventPublished, EventDelivered, AgentRegistered, AgentUnregistered
  - Filters: by agent_id, topic, event_type
- `GET /api/flow`: Event flow graph (nodes + edges, 60s retention)
- `GET /api/topology`: Agent topology (agents + subscriptions + capabilities)
- `GET /api/metrics`: System metrics (throughput, latency, backlog, errors)
- `GET /api/agents`: Registered agents with heartbeat status

**UI Components**:

- **Event Timeline**: Chronological event feed with filtering and search
- **Agent Network Graph**: Animated D3.js force-directed graph showing agent communication
- **Metrics Cards**: Real-time throughput, latency, active agents, error rate
- **Agent Details**: Per-agent view with subscriptions, capabilities, and health

**FlowTracker** (core component):

- Records event flows: (source, target, topic) → count + last_event_ms
- Tracks nodes: agents, EventBus, Router, LLM, Tool, Storage
- Cleanup: Flows older than 60s purged, nodes older than 120s removed
- **Gap**: The dashboard UI does not yet visualize the `trace_id`. While the backend `FlowTracker` and `DashboardEvent` now include the `trace_id`, the frontend needs to be updated to display it and link to a tracing backend like Jaeger.

**Integration Points**:

- EventBus broadcasts to Dashboard via `EventBroadcaster` (tokio broadcast channel)
- Bridge registers/unregisters agents → Dashboard updates topology
- FlowTracker records publish/deliver/subscription flows

**Development Workflow**:

```bash
cd core/src/dashboard/frontend
npm install
npm run dev        # Hot reload dev server
npm run build      # Production build → ../static/
```

**Limitations**:

- No historical playback (events retained for 60s only)
- No trace timeline view (requires trace_id in Envelope)
- No per-event drill-down (need trace_id → span correlation)
- Metrics not yet connected to Prometheus/Grafana

### Distributed Trace Timeline

The Dashboard now includes an experimental **Trace Timeline** view (see `docs/dashboard/TIMELINE.md`) that renders spans from Rust core components, the Bridge, and Python agents in synchronized swimlanes.

Data Path:

```
OpenTelemetry Spans (Rust + Python)
  │
  ├─ SpanCollector (in‑process ring buffer, 10k spans)
  │
  ├─ /api/spans/recent  (initial load)
  ├─ /api/spans/stream  (SSE incremental updates)
  └─ /api/traces/{trace_id} (focused trace drill‑down)
    │
    └─ Timeline.tsx (React, selective render, pause/resume)
```

Key Concepts:

- **SpanCollector** implements an OpenTelemetry `SpanProcessor`, normalizing spans into a light `SpanData` struct.
- **Trace Context Propagation**: `Envelope` carries W3C traceparent across EventBus, Bridge, and Python SDK boundaries.
- **Live Mode**: SSE pushes batches (`event: spans`) and the UI appends while trimming history (`maxSpans` client-side).
- **Filtering**: Client can target a single `trace_id` without over-fetching other spans.

Current Constraints:

- No hierarchical flamegraph yet (flat swimlane only)
- Error spans use basic coloring; severity heatmaps are planned
- Limited attribute projection (agent_id, topic, correlation_id); needs extension for tool calls

If Timeline UI fails to update after a frontend build, recompile `loom-core` to embed new assets (static bundling via `include_dir!`). See Troubleshooting section in `docs/dashboard/TESTING_GUIDE.md`.

## Data Flow Examples

### Example 1: Real-time Face Emotion Recognition

```
Camera → VideoFrame Event
  → EventBus.publish("camera.front")
    → FaceAgent receives event
      → Router: Privacy OK, LocalModel confidence=0.92 → Local
        → Plugin: face-detector → {expression: "happy"}
          → Agent generates Action: ui_update {emoji: "😊"}
            → UI updates
```

### Example 2: Hybrid Voice Assistant (current demo path)

```
Mic → AudioChunk Event
  → EventBus.publish("mic.primary.speech")
    → VoiceAgent receives
      → Router: Hybrid strategy
        ├─ Local: whisper-tiny → "what's the weather" (0.7 conf)
        │   → UI shows immediate feedback
        └─ Cloud: GPT-4 → refined intent
            → Tool calls via ActionBroker (e.g., Weather API)
              → TTS provider (Piper preferred, falls back to espeak-ng)
              → Optional cross-process forwarding via bridge
```

## Component Interaction

**Interaction Matrix**:

| Component     | Event Bus | Agent Runtime | Router | Plugin | Storage |
| ------------- | --------- | ------------- | ------ | ------ | ------- |
| Event Bus     | -         | Send events   | -      | -      | Log     |
| Agent Runtime | Subscribe | -             | Query  | Call   | R/W     |
| Router        | -         | Return route  | -      | -      | Perf    |
| Plugin        | Publish   | -             | -      | -      | -       |
| Storage       | -         | -             | -      | -      | -       |

**Key Collaboration Patterns**:

1. **Event-Driven Pipeline**: `Event Source → Event Bus → Agent → Plugin → Action`
2. **Stateful Processing**: Agent reads/updates state from Storage on each event
3. **Routing Optimization**: Agent queries Router for Local/Cloud/Hybrid decision
4. **Plugin Composition**: Agent calls multiple plugins and fuses results
5. **Cross-Process Bridging**: Optional bridge forwards events/actions to external runtimes; Python clients (loom-py) can subscribe/publish and invoke capabilities via the same proto contracts.

## Performance Targets

**Latency**:

- Event Bus: < 1ms (in-memory routing)
- Agent Dispatch: < 5ms
- Local Model: 10-100ms
- Cloud Model: 200-2000ms

**Throughput**:

- Event Bus: 10k events/sec (single node)
- Agent Runtime: 100 concurrent agents
- Storage: 5k writes/sec

**Resources**:

- Memory: < 2GB (edge devices)
- GPU: Shared inference engine
- Network: Optimized payload size

## Security & Privacy

**Privacy Protection**:

- Tiered policies (Public/Sensitive/Private/LocalOnly)
- Optional payload encryption
- Minimal data upload (embeddings > raw data)

**Access Control**:

- Plugin capability declaration
- Runtime permission checks
- Audit logging

**Compliance**:

- GDPR: User data deletion
- Transparency: Explainable decisions
- Consent management

---

For detailed API documentation, see the source code.

## Core documentation

Detailed component-level documentation is available under `docs/core/`:

- `docs/core/overview.md` — high-level overview and dataflow
- `docs/core/event_bus.md` — Event Bus responsibilities and tuning
- `docs/core/agent_runtime.md` — Agent lifecycle and mailboxing
- `docs/core/router.md` — Routing policies and decision logging
- `docs/core/action_broker.md` — Capability registration and invocation
- `docs/core/llm.md` — LLM adapters, streaming, retries
- `docs/core/memory.md` — Memory & Context system (episodic + agent decision tracking)
- `docs/core/plugin_system.md` — Plugin lifecycle and interfaces
- `docs/core/storage.md` — Storage modes and configuration
- `docs/core/telemetry.md` — Recommended metrics and spans
- `docs/core/envelope.md` — Thread/correlation semantics and helpers
- `docs/core/collaboration.md` — Collaboration primitives
- `docs/core/directory.md` — Agent & Capability directories
- `docs/core/cognitive_runtime.md` — Cognitive agent pattern (perceive-think-act)

These pages provide implementation pointers, common error modes, and test guidance for each core component.
