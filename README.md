# Loom — Event‑Driven Multi‑Agent OS

Weaving intelligence into the fabric of reality

[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://www.rust-lang.org)

Loom is an event‑driven runtime for building multi‑agent systems. The Rust core gives you a high‑performance event bus, a stateful agent runtime, collaboration primitives, and a unified action system. SDKs like loom‑py let you write agents in Python in minutes and collaborate with Rust agents through a shared envelope and protocol.

Why Loom now:

- Native multi‑agent collaboration: request/reply, fanout/fanin (first‑k/timeout), contract‑net, all built on a consistent Envelope.
- Event‑driven from the ground up: QoS and backpressure keep real‑time loops healthy; everything is an event, actions are outcomes.
- Polyglot ecosystem via loom‑py (Python today; JS next) and the Bridge service to span processes and networks.

Use Loom when you want agents that sense, reason, and act continuously—coordinating tools, models, and other agents with low latency and strong observability.

---

## What you get

- Event Bus — Async pub/sub with QoS levels and backpressure
- Agent Runtime — Actor‑style stateful agents with mailboxes and lifecycle hooks
- Collaboration — Request/reply, fanout/fanin, contract‑net powered by Envelope
- Envelope — Thread/correlation metadata with TTL/hop and reply topics
- Action System — ActionBroker + Tool Orchestrator; idempotency, timeouts, result correlation; MCP‑friendly
- Model Router — Local/Cloud/Hybrid routing by privacy/latency/cost/quality policy
- Observability — Structured logs, tracing, and metrics (designed for dashboards)
- Bridge — Optional gRPC/WebSocket bridge for cross‑process/event streaming and remote action invocation
- SDKs — loom‑py for Python (shipping), loom‑js for JS (in progress)

See details in the docs under `docs/core/*`.

---

## Quick start

Choose your path: Python (loom‑py) for fastest iteration, or Rust for full runtime control.

### A. Python (loom‑py) — 5‑minute multi‑agent

1. Install and run a sample (coming from loom‑py):

```python
from loom import Agent, capability

@capability("web.search")
def web_search(query: str) -> str:
    return f"results for {query}"

planner = Agent(id="planner", topics=["topic.plan"], capabilities=[])
researcher = Agent(id="researcher", topics=["topic.research"], capabilities=[web_search])

@planner.on_event
async def plan(ctx, evt):
    thread = ctx.thread(evt)  # uses Envelope.thread_id
    await ctx.request(thread, "topic.research", {"q": "best LLM papers"}, first_k=1, timeout_ms=2000)
    await ctx.reply(thread, {"done": True})

@researcher.on_event
async def work(ctx, evt):
    q = evt.payload.get("q")
    results = web_search(q)
    await ctx.reply(ctx.thread(evt), {"results": results})

if __name__ == "__main__":
    # Connect via Bridge; registers agents and starts streaming
    Agent.run_all([planner, researcher])
```

2. Explore more examples in `loom-py/examples`.

### B. Rust — minimal event flow

```rust
use loom_core::{EventBus, Event};
use std::sync::Arc;

#[tokio::main]
async fn main() -> loom_core::Result<()> {
    let bus = Arc::new(EventBus::new().await?);
    bus.start().await?;
    let mut rx = bus.subscribe("topic.hello").await?;
    bus.publish("topic.hello", Event::new("hi")) .await?;
    let _evt = rx.recv().await;
    Ok(())
}
```

### C. Run the Voice Agent demo

```bash
cargo build --workspace
bash demo/voice_agent/scripts/setup_models.sh   # optional STT/TTS models
cargo run -p voice_agent
```

---

## Design in 60 seconds

Everything flows through events. Each event carries an Envelope with:

- thread_id, correlation_id — conversation + per‑message correlation
- sender, reply_to — identity and reply topic (`thread.{id}.reply`)
- ttl, hop — propagation control across agents
- ts — timestamp

Collaboration primitives use the same topics (`thread.{id}.broadcast/reply`) and Envelope to coordinate multi‑agent work. Actions are invoked via the ActionBroker; the Tool Orchestrator parses tool calls (incl. MCP tools), runs them with timeouts/idempotency, and emits results with correlation.

See:

- docs/core/envelope.md — Envelope semantics and helpers
- docs/core/collaboration.md — request/reply, fanout/fanin, contract‑net
- docs/core/directory.md — agent/capability directories
- docs/ARCHITECTURE.md — full component breakdown

---

## Project structure

```
loom/
├── core/           # Runtime: event bus, agent runtime, router, action broker, tool orchestrator
├── loom-proto/     # Protobuf definitions and generated code (vendored protoc)
├── loom-audio/     # Optional audio stack (mic, VAD, STT, wake, TTS)
├── bridge/         # Optional process/network bridge (gRPC/WS) for agents & actions
├── loom-py/        # Python SDK (agents, capabilities, client), examples
├── demo/           # Demos (voice_agent first E2E demo)
├── docs/           # Documentation
└── infra/          # Docker, Prometheus, etc.
```

---

## Positioning & roadmap

We aim to let developers spin up a long‑running, observable, extensible multi‑agent system in under 10 minutes—even if they write agents in Python or JS. The Rust core keeps it fast and robust; the Bridge + SDKs make it polyglot.

Selected roadmap highlights (see `docs/ROADMAP.md`):

- P0 (MVS): 3‑agent Planner/Researcher/Writer flow in Python/JS, basic dashboard, CLI quickstart
- P1: Collaboration expansion (contract‑net, parallelism), better metrics, error taxonomy
- P2: MCP server mode, learning‑based routing, security & namespaces
- P3: Mobile/edge packaging and deep performance work

---

## Contributing

Contributions welcome! Start with `CONTRIBUTING.md`. We’re especially excited about:

- New capability providers (native or MCP)
- SDK ergonomics & examples (loom‑py/loom‑js)
- Dashboard and observability
- Collaboration strategies

---

## License

Apache License 2.0 — see [LICENSE](LICENSE)

---

Loom — Weaving Intelligence into the Fabric of Reality

# Loom — Event-Driven AI OS

_Weaving intelligence into the fabric of reality_

[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://www.rust-lang.org)

Loom is a runtime that enables AI agents to continuously sense, reason, and act in the real world. It’s built around events instead of requests: events in, actions out, state in the middle. QoS and backpressure keep things real-time; the router chooses local vs cloud intelligently.

## What’s in this repo

- `loom-proto` — Shared protobuf definitions. We vendor `protoc` via `protoc-bin-vendored` in build.rs, so you don’t need a system install.
- `core` (loom-core) — Runtime: Event Bus, Agent Runtime, Router, LLM client, ActionBroker, Plugin manager. Depends only on `loom-proto`.
- `loom-audio` — Optional audio stack: mic, VAD, STT (whisper.cpp), wake, TTS (Piper/espeak-ng). Depends on `loom-proto` and `core`.
- `demo/voice_agent` — The first complete end-to-end demo app wiring the audio stack through the core runtime.
- `bridge` — Optional process/network bridge for forwarding events & actions to external runtimes (e.g. mobile, web workers) using shared proto contracts.
- `loom-py` — Python bindings & examples for interacting with Loom (publish/subscribe events, invoke actions) from Python workflows.

Dependency directions: `loom-proto` → `core` → (optionally) app; `loom-audio` depends on both `loom-proto` and `core`. `core` does not depend on `loom-audio` to keep the runtime slim and portable.

## 🏗️ Architecture (high level)

```
Event Sources (Camera, Audio, Sensors, UI, Network)
            ↓
      Event Bus (Pub/Sub with QoS & Backpressure)
            ↓
    Agents (Stateful, Actor-based)
            ↓
      Model Router (Local / Cloud / Hybrid)
            ↓
    Plugins & Actions (TTS, UI, Tools/APIs)
            ↓
 Collaboration & Directories (multi-agent workflows & discovery)
```

See details and component contracts in `docs/ARCHITECTURE.md`.

### Core documentation

Component pages in `docs/core/`:

- `docs/core/overview.md` — overview and dataflow
- `docs/core/event_bus.md` — Event Bus
- `docs/core/agent_runtime.md` — Agent Runtime
- `docs/core/router.md` — Router
- `docs/core/action_broker.md` — ActionBroker
- `docs/core/llm.md` — LLM Client
- `docs/core/plugin_system.md` — Plugin System
- `docs/core/storage.md` — Storage
- `docs/core/telemetry.md` — Telemetry
- `docs/core/envelope.md` — Thread/correlation envelope semantics
- `docs/core/collaboration.md` — Request/reply, fanout/fanin, contract-net primitives
- `docs/core/directory.md` — Agent & Capability directories

## 🚀 Quick Start

The fastest way to see Loom in action is to run the Voice Agent demo.

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

Alternatively, if you want a minimal code sample using just `loom-core`, see `docs/QUICKSTART.md` for a tiny pub/sub example.

### Configure routing policy (per agent)

Set policy via `AgentConfig.parameters` (string map):

```
"routing.privacy" = "sensitive"
"routing.latency_budget_ms" = "300"
"routing.cost_cap" = "0.02"
"routing.quality_threshold" = "0.9"
```

These influence Local/Cloud/Hybrid selection; Hybrid runs a local quick pass and an optional cloud refine pass.

## 📦 Project Structure

```
loom/
├── Cargo.toml
├── core/              # Runtime: event bus, agents, router, plugins, LLM client
├── loom-audio/        # Optional audio stack (mic, VAD, STT, wake, TTS)
├── loom-proto/        # Protobuf definitions and generated code (vendored protoc)
├── demo/
│   └── voice_agent/   # First E2E demo app
├── infra/             # Docker, Prometheus, etc.
└── docs/              # Documentation
```

## 🔑 Core Components

- Event Bus — Async pub/sub with QoS, backpressure, and topic routing
- Agent Runtime — Stateful actors with persistent state and ephemeral context
- Model Router — Local/Cloud/Hybrid selection driven by policy (privacy/latency/cost/quality)
- Plugin System — Extensible architecture with isolation options (WASM/out-of-process)
- Storage — RocksDB for state; Vector DB for long-term memory (optional)
- Telemetry — Metrics, tracing, and structured logs
- Envelope — Shared thread/correlation metadata across events & actions (TTL/hop, reply topics)
- Collaboration — High-level multi-agent patterns (request/reply, fanout/fanin, contract-net)
- Directories — Discovery for agents (topics/capabilities) & capabilities (provider snapshot)

The audio pipeline (mic/VAD/STT/wake/TTS) lives in `loom-audio` and is intentionally optional.

## 🧩 Plugins & Integrations

- Native Rust, WASM sandbox, or out‑of‑process (gRPC) providers
- Shared plugin protocol defined in `loom-proto/proto/plugin.proto`
- Integrations: vLLM/OpenAI-compatible LLMs, workflow tools (e.g., n8n), and more — see `docs/INTEGRATIONS.md`

## 📚 More docs

- `docs/ARCHITECTURE.md` — system design and component contracts
- `docs/EXAMPLES.md` — demos and example locations
- `docs/ROADMAP.md` — near‑term milestones (centered on Voice Agent E2E)
- `docs/BACKPRESSURE.md` — EventBus QoS and policies
- `docs/EXTENSIBILITY.md`, `docs/INTEGRATIONS.md`, `docs/MOBILE.md`
- `docs/core/envelope.md`, `docs/core/collaboration.md`, `docs/core/directory.md`

## 🤝 Contributing

Contributions welcome! See [CONTRIBUTING.md](CONTRIBUTING.md).

## 📄 License

Apache License 2.0 — see [LICENSE](LICENSE)

---

Loom — Weaving Intelligence into the Fabric of Reality
