# Loom — Event-Driven AI OS

_Weaving intelligence into the fabric of reality_

[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://www.rust-lang.org)

Loom is a runtime that enables AI agents to continuously sense, reason, and act in the real world. It's built around events instead of requests: events in, actions out, state in the middle. QoS and backpressure keep things real-time; the router chooses local vs cloud intelligently.

**Why Loom:**

- **Native multi-agent collaboration**: request/reply, fanout/fanin, contract-net, barrier — powered by Envelope (thread/correlation semantics)
- **Event-driven from the ground up**: QoS levels, backpressure handling, topic routing with wildcard support
- **Polyglot ecosystem**: Write agents in Python (loom-py), JavaScript (loom-js, coming), or Rust; Bridge service spans processes and networks
- **MCP integration**: Connect to Model Context Protocol servers; access filesystems, databases, APIs as native capabilities
- **Production-ready**: Comprehensive error handling, timeout management, observability hooks, integration tests

## What's in this repo

- **loom-proto** — Shared protobuf definitions. We vendor `protoc` via `protoc-bin-vendored` in build.rs, so you don't need a system install.
- **core** (loom-core) — Runtime: Event Bus, Agent Runtime, Router, ActionBroker, Tool Orchestrator, MCP Client, Collaboration primitives, Directories. Depends only on `loom-proto`.
- **loom-audio** — Optional audio stack: mic, VAD, STT (whisper.cpp), wake detection, TTS (Piper/espeak-ng). Depends on `loom-proto` and `core`.
- **bridge** — gRPC service for cross-process event/action streaming. Supports RegisterAgent, bidirectional EventStream, ForwardAction, Heartbeat. Enables Python/JS agents to participate in the Loom ecosystem.
- **loom-py** — Python SDK with Agent/Context API, @capability decorator, Envelope support. Includes trio example (Planner/Researcher/Writer). PyPI-ready (`0.1.0a1`).
- **demo/voice_agent** — First complete E2E demo app wiring audio stack through the core runtime.

Dependency directions: `loom-proto` → `core` → (optionally) app; `loom-audio` depends on both `loom-proto` and `core`. `core` does not depend on `loom-audio` to keep the runtime slim and portable.

## 🏗️ Architecture (high level)

```
Event Sources (Camera, Audio, Sensors, UI, Network, Python/JS Agents)
            ↓
   Event Bus (Pub/Sub with QoS & Backpressure)
            ↓
 Agents (Stateful, Actor-based with Mailboxes)
      ↓           ↓
  Router      ActionBroker (Capability Registry)
      ↓           ↓
 Local/Cloud   Tools/APIs (Native, MCP, Plugins)
      ↓
  Actions & Results (with correlation)
```

See full component breakdown and contracts in `docs/ARCHITECTURE.md`.

## 🚀 Quick Start

### Option A: Python Multi-Agent Example (5-minute quickstart)

1. Start the Bridge server

```bash
cargo run -p loom-bridge --bin loom-bridge-server
```

2. Write your agents (see `loom-py/examples/trio.py`):

```python
from loom import Agent, capability

@capability("research.search", version="1.0")
def search(query: str) -> dict:
    return {"results": ["https://example.com/doc1"]}

async def planner_handler(ctx, topic, event):
    if event.type == "user.question":
        # Use Envelope thread_id for correlation
        thread = ctx.thread(event)
        results = await ctx.request(thread, "topic.research",
                                   payload=event.payload,
                                   first_k=1, timeout_ms=2000)
        await ctx.reply(thread, {"done": True})

async def researcher_handler(ctx, topic, event):
    if event.type == "research.request":
        results = search(query=event.payload.decode())
        await ctx.reply(ctx.thread(event), {"results": results})

# Create and run agents
planner = Agent("planner", topics=["topic.plan"], on_event=planner_handler)
researcher = Agent("researcher", topics=["topic.research"],
                  capabilities=[search], on_event=researcher_handler)

await planner.start()
await researcher.start()
```

3. Explore more examples in `loom-py/examples/`.

### Option B: Voice Agent Demo (fastest way to see Loom in action)

1. Build the workspace

```bash
cargo build --workspace
```

2. Prepare STT/TTS models (optional helper script)

```bash
bash demo/voice_agent/scripts/setup_models.sh
```

3. Run the demo

```bash
cargo run -p voice_agent
```

For advanced setup (local vLLM, Piper voices, environment-only config), see `demo/voice_agent/README.md`.

### Option C: Minimal Rust Example

```rust
use loom_core::{EventBus, Event};
use std::sync::Arc;

#[tokio::main]
async fn main() -> loom_core::Result<()> {
    let bus = Arc::new(EventBus::new().await?);
    bus.start().await?;

    // Subscribe and publish
    let mut rx = bus.subscribe("topic.hello").await?;
    bus.publish("topic.hello", Event::new("greeting")).await?;

    let evt = rx.recv().await.unwrap();
    println!("Received event: {}", evt.event_type);

    Ok(())
}
```

See `docs/QUICKSTART.md` for more Rust examples.

## 📦 Project Structure

```
loom/
├── Cargo.toml
├── core/              # Runtime: EventBus, AgentRuntime, Router, ActionBroker, MCP
├── loom-proto/        # Protobuf definitions and generated code (vendored protoc)
├── loom-audio/        # Optional audio stack (mic, VAD, STT, wake, TTS)
├── bridge/            # gRPC bridge for cross-process agents and actions
├── loom-py/           # Python SDK (Agent, Context, @capability)
│   ├── src/loom/      # Core Python API
│   └── examples/      # trio.py and more
├── demo/
│   └── voice_agent/   # First E2E demo app
├── docs/              # Architecture, guides, component docs
└── infra/             # Docker, Prometheus, etc.
```

## 🔑 Core Components

### Event Bus

Asynchronous pub/sub with QoS levels (`Realtime`, `Batched`, `Background`), backpressure handling (sampling, dropping, aggregation), and topic routing. Supports thread-based broadcast (`thread.{id}.broadcast`) and reply topics (`thread.{id}.reply`).

### Agent Runtime

Actor-based stateful agents with lifecycle management (create/start/stop/delete), dynamic topic subscriptions, persistent state (RocksDB), ephemeral context (in-memory), and mailbox-based event distribution.

### Envelope

Unified metadata envelope for events and actions with reserved keys: `thread_id`, `correlation_id`, `sender`, `reply_to`, `ttl`, `hop`, `ts`. Enables multi-agent collaboration with automatic TTL/hop management.

### Collaboration Primitives

Built on top of Envelope:

- **request/reply**: Correlated request-response with timeout
- **fanout/fanin**: Broadcast with strategies (any, first_k, majority, timeout)
- **barrier**: Wait for N agents to check in
- **contract-net**: Call for proposals, bid collection, award, execution

### ActionBroker & Tool Orchestrator

Unified capability registry and invocation layer. Supports native Rust providers, MCP tools, WASM plugins, and remote capabilities (via Bridge). Standardized error codes: `ACTION_OK`, `ACTION_ERROR`, `ACTION_TIMEOUT`, `INVALID_PARAMS`, `CAPABILITY_ERROR`, `PROVIDER_UNAVAILABLE`.

### MCP Client (✅ Complete)

Connect to Model Context Protocol servers and use their tools as native capabilities:

- JSON-RPC 2.0 over stdio transport
- Auto-discovery and registration of tools
- Qualified naming (server:tool) to avoid conflicts
- Configurable protocol version with validation
- Comprehensive error handling and timeout management

Future enhancements (P1): SSE transport, Resources/Prompts/Sampling APIs, Notifications.

### Model Router

Policy-based routing (Local/Cloud/Hybrid) driven by:

- Privacy policy (`public`, `sensitive`, `private`, `local-only`)
- Latency budget (ms)
- Cost cap (per-call limit)
- Quality threshold (confidence score)

Logs every routing decision with reason, confidence, estimated latency/cost.

### Bridge (gRPC)

Cross-process event and action forwarding:

- **RegisterAgent**: External agents (Python/JS) register with topics and capabilities
- **EventStream**: Bidirectional streaming (publish/receive events)
- **ForwardAction**: Client-initiated capability invocation
- **ActionCall**: Server-initiated action push (internal correlation map)
- **Heartbeat**: Connection health monitoring

Enables polyglot multi-agent systems with Python/JS agents collaborating with Rust agents.

### Directories

- **AgentDirectory**: Discover agents by id/topics/capabilities; auto-registers on creation
- **CapabilityDirectory**: Snapshot of registered providers from ActionBroker; query by name or type

### Storage & Telemetry

- **Storage**: RocksDB for agent state; optional Vector DB for long-term memory
- **Telemetry**: Structured logs, OpenTelemetry tracing, Prometheus metrics (events/sec, latency P50/P99, routing decisions, tool calls)

The audio pipeline (mic/VAD/STT/wake/TTS) lives in `loom-audio` and is intentionally optional.

## 🧩 Plugins & Integrations

- **Native Rust** providers: Built-in capabilities (WeatherProvider, WebSearchProvider, LlmGenerateProvider)
- **MCP tools**: Connect to any MCP server (filesystems, databases, APIs)
- **WASM sandbox** or **out-of-process (gRPC)** plugins for custom capabilities
- **Integrations**: vLLM/OpenAI-compatible LLMs, workflow tools (n8n), and more

See `docs/INTEGRATIONS.md` and `docs/MCP.md` for details.

## 🗺️ Roadmap

**P0 (MVS — Minimal Viable System)**: ✅ Mostly complete

- ✅ Bridge (gRPC) with full lifecycle
- ✅ Python SDK (loom-py) with trio example
- ✅ Collaboration primitives (request/reply, fanout/fanin, contract-net, barrier)
- ✅ MCP Client (stdio transport, auto-discovery, qualified naming)
- ✅ Directories (Agent & Capability)
- 🚧 Dashboard MVP (topology, metrics, swimlanes) — in progress
- 🚧 CLI basics (new/dev/list/bench) — in progress
- 🚧 JS SDK (loom-js) — in progress

**P1 (Observable Iteration)**:

- Dashboard enhancements (histograms, error heatmaps, backpressure gauges)
- CLI templates (voice-assistant, home-automation, etc.)
- Streaming APIs and parallelism (SSE, semaphore, circuit breaker)
- Error taxonomy and unified error_event
- SDK ergonomics (memory plugins, type hints)
- MCP enhancements (SSE transport, Resources/Prompts APIs)

**P2 (Ecosystem & Policy)**:

- MCP server mode (expose Loom capabilities externally)
- Learning-based routing (bandit/RL algorithms)
- Security & multi-tenancy (namespaces, ACLs, audit logs)
- Event persistence & replay (WAL, snapshots, time-travel debugging)
- WASI tool isolation

**P3 (Performance & Mobile)**:

- Mobile/edge packaging (iOS/Android xcframework/AAR)
- Deep performance optimization (lock-free, zero-copy, GPU/NPU)
- Production hardening (graceful degradation, circuit breakers)

See `docs/ROADMAP.md` for detailed milestones and acceptance criteria.

### Core documentation

Component-level documentation in `docs/core/`:

- `docs/core/overview.md` — dataflow and system overview
- `docs/core/event_bus.md` — Event Bus (QoS, backpressure, topic routing)
- `docs/core/agent_runtime.md` — Agent Runtime (lifecycle, mailboxes, subscriptions)
- `docs/core/router.md` — Router (policy-based Local/Cloud/Hybrid selection)
- `docs/core/action_broker.md` — ActionBroker (capability registry and invocation)
- `docs/core/llm.md` — LLM Client (streaming, retries, provider adapters)
- `docs/core/plugin_system.md` — Plugin System (WASM, out-of-process)
- `docs/core/storage.md` — Storage (RocksDB, Vector DB)
- `docs/core/telemetry.md` — Telemetry (metrics, tracing, structured logs)
- `docs/core/envelope.md` — Envelope (thread/correlation metadata)
- `docs/core/collaboration.md` — Collaboration primitives (request/reply, fanout/fanin, contract-net)
- `docs/core/directory.md` — Directories (agent & capability discovery)

Additional documentation:

- `docs/MCP.md` — MCP Client guide and configuration
- `docs/BRIDGE.md` — Bridge protocol and usage
- `docs/ROADMAP.md` — development roadmap and milestones
- `docs/BACKPRESSURE.md` — EventBus QoS policies
- `docs/EXTENSIBILITY.md`, `docs/INTEGRATIONS.md`, `docs/MOBILE.md`

## 🤝 Contributing

Contributions welcome! See [CONTRIBUTING.md](CONTRIBUTING.md). We're especially excited about:

- New capability providers (native or MCP integrations)
- SDK ergonomics & examples (loom-py/loom-js)
- Dashboard and observability tools
- Collaboration strategies and patterns
- Documentation improvements

## 📄 License

Apache License 2.0 — see [LICENSE](LICENSE)

---

_Loom — Weaving Intelligence into the Fabric of Reality_
