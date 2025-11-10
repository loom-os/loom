# Quick Start Guide

## Prerequisites

1. Rust toolchain (1.70+)

   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. Protocol Buffers compiler — not required on your system. We vendor `protoc` in `loom-proto` via `protoc-bin-vendored`.

## Option A — Run the Voice Agent demo (recommended)

From the repository root:

```bash
cargo build --workspace
bash demo/voice_agent/scripts/setup_models.sh   # optional helper to fetch a small Whisper model and a Piper voice
cargo run -p voice_agent
```

Tips:

- The demo prefers Piper for TTS; if Piper isn’t installed, it falls back to espeak‑ng.
- vLLM (or any OpenAI‑compatible server) is optional; see `demo/voice_agent/README.md` to point the LLM client to your backend.
- On Linux, install `libasound2-dev` and `pkg-config` for audio.

## Option B — Minimal `loom-core` pub/sub

Create a new Rust binary project and add `loom-core` as a dependency (or use a workspace member). Paste this into `main.rs`:

```rust
use loom_core::{Loom, Event, QoSLevel};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut system = Loom::new().await?;
    system.start().await?;

    // Subscribe to events
    let (_sub_id, mut rx) = system
        .event_bus
        .subscribe("test.topic".to_string(), vec![], QoSLevel::QosBatched)
        .await?;

    // Spawn receiver
    tokio::spawn(async move {
        while let Some(event) = rx.recv().await {
            println!("Received: {}", event.id);
        }
    });

    // Publish an event
    let event = Event {
        id: "evt_001".to_string(),
        r#type: "test_event".to_string(),
        timestamp_ms: chrono::Utc::now().timestamp_millis(),
        source: "demo".to_string(),
        ..Default::default()
    };

    system.event_bus.publish("test.topic", event).await?;

    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    system.shutdown().await?;
    Ok(())
}
```

## Routing policy (per agent)

Tune Local/Cloud/Hybrid behavior via `AgentConfig.parameters`:

```rust
// key → string values
// routing.privacy = public | sensitive | private | local-only
// routing.latency_budget_ms = integer (u64)
// routing.cost_cap = float (f32)
// routing.quality_threshold = float (f32)
```

Hybrid processing runs a quick local pass and, if needed, a cloud refine pass. Behaviors receive metadata: `routing_target`, `phase` (quick/refine), and `refine=true` on the second pass.

## Core component references

For detailed runtime internals consult:

- `docs/core/overview.md`
- `docs/core/event_bus.md`
- `docs/core/agent_runtime.md`
- `docs/core/router.md`
- `docs/core/action_broker.md`
- `docs/core/llm.md`
- `docs/core/plugin_system.md`
- `docs/core/storage.md`
- `docs/core/telemetry.md`

## Troubleshooting

- "protoc not found": not needed — `loom-proto` vendors `protoc` during build.
- Enable debug logs: `RUST_LOG=debug cargo run ...`
- Audio build issues on Linux: `sudo apt-get install -y libasound2-dev pkg-config`

---

Questions? Open an issue on [GitHub](https://github.com/loom-os/loom/issues)
