# Loom - Event-Driven AI Operating System

[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://www.rust-lang.org)

> A runtime layer that enables AI agents to continuously sense, reason, and act in the real world

## 🎯 Core Philosophy

Loom redefines agent systems from first principles:

```
Sensing → Reasoning → Acting
```

Unlike traditional request-response patterns, Loom uses **event-driven architecture** to enable AI systems to:

- 📡 **Continuous Sensing**: Real-time multimodal event streams (vision, audio, touch, sensors)
- 🧠 **Stateful Reasoning**: Maintains long-term memory and short-term context
- 🎛️ **Intelligent Routing**: Dynamic scheduling between local and cloud models
- 🔌 **Plugin Architecture**: Extensible WASM plugin system
- 🔒 **Privacy-First**: Built-in privacy controls and data protection

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

## 📦 Project Structure

```
loom/
├── core/              # Rust core runtime
│   ├── src/           # Event bus, agents, router, plugins
│   └── proto/         # Protobuf definitions
├── plugins/           # Plugin examples
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
