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

**Memory System**:

- **Episodic**: Event sequences
- **Semantic**: Knowledge graph
- **Working**: Active context

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
async def on_event(ctx: Context, topic: str, event):
    thread = ctx.thread(event)  # Extract thread_id from Envelope
    await ctx.emit("target.topic", type="msg", payload=b"data")
    results = await ctx.request(thread, "topic", payload, first_k=1, timeout_ms=2000)
    await ctx.reply(thread, {"done": True})

agent = Agent("my-agent", topics=["topic.in"], capabilities=[search], on_event=on_event)
await agent.start()  # Connects to bridge, registers, starts streaming
```

**Features**:

- Agent/Context abstraction with Envelope support
- @capability decorator with Pydantic schema auto-generation
- BridgeClient with gRPC connection management
- Automatic correlation_id handling for request/reply
- Thread-aware operations (thread(), emit(), request(), reply())

**Example**: `loom-py/examples/trio.py` — Planner/Researcher/Writer collaboration

**Packaging**: PyPI-ready with `pyproject.toml` (package name: `loom`)

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

Built-in observability:

**Metrics**:

- Throughput: events/sec
- Latency: P50/P99/Max
- Routing: local_rate, cloud_rate
- Resources: CPU/GPU/Memory
- Cost: Estimated cloud API usage

**Tracing** (OpenTelemetry):

```
Span: PublishEvent
  └─ Span: RouteDecision
      ├─ Span: LocalInference
      └─ Span: CloudRequest
```

**Logging**: Structured JSON logs (DEBUG/INFO/WARN/ERROR) with sensitive data masking

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
- `docs/core/plugin_system.md` — Plugin lifecycle and interfaces
- `docs/core/storage.md` — Storage modes and configuration
- `docs/core/telemetry.md` — Recommended metrics and spans
- `docs/core/envelope.md` — Thread/correlation semantics and helpers
- `docs/core/collaboration.md` — Collaboration primitives
- `docs/core/directory.md` — Agent & Capability directories

These pages provide implementation pointers, common error modes, and test guidance for each core component.
