# Loom : Event-Driven AI OS

_Weaving intelligence into the fabric of reality_

[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://www.rust-lang.org)

> A runtime that enables AI agents to continuously sense, reason, and act in the real world

At a glance:

- From prompts to events — always-on, stateful, asynchronous agents.
- Events in, actions out; state in the middle.
- QoS + backpressure to stay real-time; router picks edge vs cloud.

Further reading: see `docs/POSITIONING.md` and `docs/INTEGRATIONS.md`.

## 🎯 Core Philosophy

Unlike traditional request-response patterns, Loom uses **event-driven architecture** to enable AI systems to:

- 📡 **Continuous Sensing**: Real-time multimodal event streams (vision, audio, touch, sensors)
- 🧠 **Stateful Reasoning**: Maintains long-term memory and short-term context
- 🎛️ **Intelligent Routing**: Dynamic scheduling between local and cloud models
- 🔌 **Plugin Architecture**: Extensible WASM plugin system
- 🔒 **Privacy-First**: Built-in privacy controls and data protection

In one line: From prompts to events — Loom turns LLM chatbots into always-on, stateful, asynchronous agents that subscribe to real-time multimodal streams and emit actions with QoS, backpressure, and intelligent edge/cloud routing.

## 🏗️ Architecture

```
Event Sources (Camera, Audio, Sensors, UI, Network)
            ↓
      Event Bus (Pub/Sub with QoS & Backpressure)
            ↓
    Agents (Stateful, Actor-based)
            ↓
      Model Router (Local/Cloud/Hybrid)
            ↓
    Plugins & Actions (TTS, UI, APIs)
```

See more in `docs/ARCHITECTURE.md`.

More docs:

- `docs/POSITIONING.md` — concise positioning and comparisons
- `docs/MOBILE.md` — mobile‑friendly targets and build choices
- `docs/EXTENSIBILITY.md` — plugin tiers, security, and SDKs
- `docs/INTEGRATIONS.md` — ecosystem integrations and adapters
- `docs/EXAMPLES.md` — out‑of‑the‑box examples and onboarding
- `docs/ROADMAP.md` — near‑term milestones
- `docs/BACKPRESSURE.md` — EventBus QoS levels, bounded queues, and backpressure policy

## 🚀 Quick Start

### Prerequisites

- Rust 1.70+
- Protocol Buffers compiler

### Installation

```bash
git clone https://github.com/yourusername/loom.git
cd loom/core
cargo build --release
```

### Basic Usage

```rust
use loom_core::{Loom, Event, QoSLevel};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut system = Loom::new().await?;
    system.start().await?;

    let event = Event {
        id: "evt_001".to_string(),
        r#type: "face_event".to_string(),
        timestamp_ms: chrono::Utc::now().timestamp_millis(),
        source: "camera.front".to_string(),
        ..Default::default()
    };

    system.event_bus.publish("camera.front", event).await?;
    Ok(())
}
```

Your first agent and app:

- Start with the Quick Start example in `docs/QUICKSTART.md` (basic pub/sub and event handling)
- Then compose a simple pipeline: Mic → wake word (WASM plugin) → Router → cloud LLM → TTS action
- We provide examples under `core/examples/` and will keep adding end-to-end demos in `examples/`

See `docs/EXAMPLES.md` for out‑of‑the‑box examples and onboarding.

## 📦 Project Structure

```
loom/
├── core/              # Rust core runtime
│   ├── src/           # Event bus, agents, router, plugins
│   └── proto/         # Protobuf definitions
├── plugins/           # Plugins
├── examples/          # Demo applications
├── infra/             # Infrastructure (Docker, k8s)
└── docs/              # Documentation
```

## 🔑 Core Components

- **Event Bus**: Async pub/sub with QoS, backpressure, and topic routing
- **Agent Runtime**: Stateful actors with persistent state (RocksDB) and ephemeral context
- **Model Router**: Intelligent local/cloud/hybrid routing based on privacy, latency, and cost
- **Plugin System**: Extensible architecture with WASM isolation
- **Storage**: RocksDB for state persistence, Vector DB integration for long-term memory
- **Telemetry**: Built-in metrics, tracing, and observability

## 🧩 Extensions and Plugins

Loom supports a tiered extension model:

- Tier 1 — Native (Rust) plugins for trusted, perf‑critical paths (best perf, weakest isolation)
- Tier 2 — WASM plugins for third‑party and sandboxed execution (portable, capability‑based security). For mobile, prefer AOT runtimes (e.g., WAMR) due to iOS JIT restrictions
- Tier 3 — Out‑of‑process plugins over gRPC/UDS for heavyweight or remote services (strong isolation, language‑agnostic)

All tiers share the same protobuf‑defined plugin protocol (see `core/proto/plugin.proto`).

## 📐 Scope: mobile‑first core, feature‑gated plus

- Core (mobile‑friendly default): Event Bus, Agent Runtime, Router abstraction, minimal telemetry
- Plus (desktop/server via Cargo features): WASM runtime, local ML backends (TFLite/ONNX), cloud connectors, vector DB, advanced telemetry

Example profiles:

- mobile‑lite: `--no-default-features --features "event-bus,router"`
- desktop‑plus: `--features "event-bus,router,wasm,local-ml,cloud-ml,metrics,rocksdb"`

Targets: < 5–8 MB code and 20–40 MB RAM for core mobile runtime; cold start ~< 200 ms

## 🖥️ Cross‑OS Adapters

Keep the core OS‑agnostic and provide thin host adapters:

- iOS: static `.xcframework` + C ABI (cbindgen) + Swift wrapper; AOT WASM runtime; AVFoundation for TTS/audio; Camera/Audio bridged as events
- Android: `cargo-ndk` + JNI bindings; AudioRecord/CameraX; TextToSpeech; AOT WASM runtime
- Desktop/Server: native processes and gRPC plugins; richer WASI capabilities allowed

Common traits (examples): `HostAdapter`, `EventSource`, `ActionSink`, capability tokens; consistent protobuf events/actions across all platforms.

## 🔗 Integrations

Loom is designed to interoperate rather than replace existing stacks:

- LangChain / LlamaIndex: integrate as out‑of‑process tools or agents via gRPC, or as WASM plugins exposing tool interfaces
- vLLM: use as a cloud/local serving backend behind the Model Router; vLLM Semantic Router signals can seed routing policies
- Kubernetes: run Loom runtime and out‑of‑process plugins as deployments; use a lightweight operator/Helm for config and secrets
- n8n / workflow tools: treat Loom as an event source/sink; provide a Loom node that subscribes to topics and emits actions/events

Integration details and adapter patterns: see `docs/INTEGRATIONS.md`.

## 🌱 Lowering the Barrier (not just Rust)

- Polyglot plugins: WASM (Rust/Go/TinyGo/C/C++/AssemblyScript) or out‑of‑process gRPC (Python/Node/Java)
- SDKs and templates: minimal plugin templates for Python/Node (gRPC), Rust (native/WASM); codegen from `proto/`
- Language bindings for apps: Swift/Kotlin/TypeScript (C ABI + wrappers) to embed Loom on iOS/Android/desktop
- Examples: we’ll maintain a growing set under `core/examples/` and `examples/`

## 🗣️ Messaging cheat‑sheet

- “From prompts to events”: real‑time, stateful, async agents
- “Events in, actions out; state in the middle”
- “QoS + backpressure prevent overload; router picks edge vs cloud”

## 🎯 Use Cases

- **AR/VR Assistants**: Real-time processing of camera, gestures, and spatial data
- **Mobile Agents**: Lightweight on-device models with cloud escalation
- **Robotics**: Sensor fusion, real-time decision-making, and action execution
- **Desktop Assistants**: System event capture, context understanding, and automation

## 🛣️ Roadmap

- ✅ **MVP 0**: Event Bus, Agent Runtime, Basic Router
- 🚧 **MVP 1**: Local model integration (TFLite/ONNX), cloud endpoints, hybrid inference
- 📅 **MVP 2**: WASM plugins, Vector DB, ML-based router
- 🔮 **MVP 3**: vLLM integration, advanced privacy controls, production optimization

## 🤝 Contributing

Contributions welcome! See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## 📄 License

Apache License 2.0 - see [LICENSE](LICENSE)

---

**Loom** - The next-generation AI operating system layer
