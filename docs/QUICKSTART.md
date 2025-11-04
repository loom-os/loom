# Quick Start Guide

## Prerequisites

1. **Rust toolchain** (1.70+)

   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. **Protocol Buffers compiler**

   ```bash
   # macOS
   brew install protobuf

   # Ubuntu/Debian
   sudo apt install protobuf-compiler
   ```

## Build

```bash
git clone https://github.com/loom-os/loom.git
cd loom/core
cargo build --release
cargo test
```

## Basic Example

Create `core/examples/basic_pubsub.rs`:

```rust
use loom_core::{Loom, proto::Event, proto::QoSLevel};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut system = Loom::new().await?;
    system.start().await?;

    // Subscribe to events
    let (sub_id, mut rx) = system.event_bus
        .subscribe("test.topic".to_string(), vec![], QoSLevel::QosBatched)
        .await?;

    // Spawn receiver
    tokio::spawn(async move {
        while let Some(event) = rx.recv().await {
            println!("Received: {}", event.id);
        }
    });

    // Publish events
    let event = Event {
        id: "evt_001".to_string(),
        r#type: "test_event".to_string(),
        timestamp_ms: chrono::Utc::now().timestamp_millis(),
        source: "demo".to_string(),
        ..Default::default()
    };

    system.event_bus.publish("test.topic", event).await?;

    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    system.shutdown().await?;

    Ok(())
}
```

Run:

```bash
cargo run --example basic_pubsub
```

## Next Steps

- Read [Architecture](ARCHITECTURE.md) for system design
- Check `examples/` for more demos
- See [Contributing](../CONTRIBUTING.md) to get involved

## Routing Policy Configuration (per agent)

The router decides between Local/Cloud/Hybrid per event. You can tune the policy per agent via `AgentConfig.parameters`:

```rust
use loom_core::agent::AgentConfig;
use std::collections::HashMap;

let mut params: HashMap<String, String> = HashMap::new();
params.insert("routing.privacy".into(), "sensitive".into());
params.insert("routing.latency_budget_ms".into(), "300".into());
params.insert("routing.cost_cap".into(), "0.02".into());
params.insert("routing.quality_threshold".into(), "0.9".into());

let config = AgentConfig {
    agent_id: "agent_1".into(),
    agent_type: "demo".into(),
    subscribed_topics: vec!["test.topic".into()],
    capabilities: vec![],
    parameters: params,
};
```

Hybrid processing runs a local quick pass followed by a cloud refine pass; behavior receives metadata: `routing_target`, `phase` (quick/refine), and `refine=true` on the second pass.

## Troubleshooting

**"protoc not found"**: Install Protocol Buffers compiler  
**Enable debug logs**: `RUST_LOG=debug cargo run`  
**Need GPU?**: Core doesn't require GPU, but some plugins might

---

Questions? Open an issue on [GitHub](https://github.com/loom-os/loom/issues)
