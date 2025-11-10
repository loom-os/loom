# Loom — Event-Driven AI OS

_Weaving intelligence into the fabric of reality_

[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://www.rust-lang.org)

Loom is a runtime that enables AI agents to continuously sense, reason, and act in the real world. It’s built around events instead of requests: events in, actions out, state in the middle. QoS and backpressure keep things real-time; the router chooses local vs cloud intelligently.

简述（中文）：本仓库当前以 Voice Agent E2E Demo 为首个完整闭环（Mic → VAD → STT → Wake → LLM → TTS）。代码按「并列三仓」组织：`loom-proto`（最基础，仅定义协议）、`core`（运行时，依赖 proto，不依赖 audio）、`loom-audio`（可选音频能力，依赖前两者）。应用可按需选择 audio；后续会有依赖 `loom-vision` 的应用与打磨。

## What’s in this repo

- `loom-proto` — Shared protobuf definitions. We vendor `protoc` via `protoc-bin-vendored` in build.rs, so you don’t need a system install.
- `core` (loom-core) — Runtime: Event Bus, Agent Runtime, Router, LLM client, ActionBroker, Plugin manager. Depends only on `loom-proto`.
- `loom-audio` — Optional audio stack: mic, VAD, STT (whisper.cpp), wake, TTS (Piper/espeak-ng). Depends on `loom-proto` and `core`.
- `demo/voice_agent` — The first complete end-to-end demo app wiring the audio stack through the core runtime.

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

## 🤝 Contributing

Contributions welcome! See [CONTRIBUTING.md](CONTRIBUTING.md).

## 📄 License

Apache License 2.0 — see [LICENSE](LICENSE)

---

Loom — Weaving Intelligence into the Fabric of Reality
