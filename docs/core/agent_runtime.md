## Agent Runtime

Responsibility

- Manage agent lifecycle, mailboxing and dispatch of messages to agent behaviors.
- Provide isolation between agents and hooks for persistence and state management.
- Enable dynamic subscription management for multi-agent collaboration.

Key files

- `core/src/agent/runtime.rs` — runtime loop, scheduling, and subscription management.
- `core/src/agent/instance.rs` — agent instance representation and state machine.
- `core/src/agent/behavior.rs` — behavior abstractions.

Key interfaces

- **Lifecycle Management**
  - `create_agent()` — Create and start an agent with initial subscriptions
  - `delete_agent()` — Stop agent and cleanup all subscriptions
- **Auto-subscription (v0.2.0)**
  - Every agent is automatically subscribed to `agent.{agent_id}.replies` at creation
  - Enables point-to-point agent communication without explicit setup
- **Dynamic Subscription API** (v0.2.0)
  - `subscribe_agent(agent_id, topic)` — Add subscription at runtime
  - `unsubscribe_agent(agent_id, topic)` — Remove subscription at runtime
  - `get_agent_subscriptions(agent_id)` — List current subscriptions
- **Mailbox API**
  - Enqueue/dequeue messages with backpressure handling
  - Automatic forwarding from EventBus subscriptions to agent mailbox

Auto-subscription Details

Every agent automatically subscribes to its private reply topic:

- Topic format: `agent.{agent_id}.replies`
- Used for point-to-point agent communication
- Distinct from thread-scoped replies (`thread.{thread_id}.reply`)
- Cannot be manually unsubscribed (managed by runtime)

Example:

```rust
// Create agent "worker-1"
runtime.create_agent(config, behavior).await?;
// Agent is now subscribed to:
// 1. "agent.worker-1.replies" (auto-subscribed)
// 2. Any topics in config.subscribed_topics

// Send direct message to worker-1
let env = Envelope::with_agent_reply("task-1", "agent.coordinator", "worker-1");
bus.publish("agent.worker-1.replies", event).await?;
```

Dynamic Subscription Use Cases

1. **Expert Consultation**: Agent joins thread when expertise is needed

   ```rust
   // Agent joins thread mid-conversation
   runtime.subscribe_agent("expert-1", "thread.task-123.broadcast").await?;
   ```

2. **Task Delegation**: Agent monitors multiple work queues dynamically

   ```rust
   // Subscribe to new work queue
   runtime.subscribe_agent("worker-1", "jobs.priority.high").await?;
   ```

3. **Adaptive Monitoring**: Agent adjusts subscriptions based on load
   ```rust
   // Unsubscribe from low-priority topics when busy
   runtime.unsubscribe_agent("monitor-1", "events.low_priority").await?;
   ```

Common error paths and test cases

- Lifecycle transitions: invalid state transitions (start -> start, stop -> execute) should be guarded.
- Mailbox overflow and cancellation: verify messages are handled or discarded according to policy.
- Persistence mismatch: state read/write failures are surfaced and cause deterministic fallback behavior.
- **Dynamic subscription errors**:
  - Subscribe to topic already subscribed → error
  - Unsubscribe from non-subscribed topic → error
  - Subscribe/unsubscribe non-existent agent → error

Tuning knobs

- Mailbox capacity per agent (default: 1000).
- Subscription QoS level (Batched for agent subscriptions).
- Scheduling quantum and priority.
- Checkpoint interval for durable state.

Example

Basic agent with dynamic subscription:

```rust
use loom_core::{AgentRuntime, AgentConfig, EventBus, ActionBroker, ModelRouter};
use std::sync::Arc;

async fn example() -> loom_core::Result<()> {
    let event_bus = Arc::new(EventBus::new().await?);
    let action_broker = Arc::new(ActionBroker::new());
    let model_router = ModelRouter::new().await?;

    let runtime = AgentRuntime::new(event_bus, action_broker, model_router).await?;

    // Create agent with initial subscription
    let config = AgentConfig {
        agent_id: "coordinator".to_string(),
        agent_type: "orchestrator".to_string(),
        subscribed_topics: vec!["tasks.incoming".to_string()],
        capabilities: vec![],
        parameters: std::collections::HashMap::new(),
    };

    runtime.create_agent(config, behavior).await?;

    // Later: agent joins collaboration thread
    runtime.subscribe_agent("coordinator", "thread.project-99.broadcast").await?;

    // Check subscriptions
    let subs = runtime.get_agent_subscriptions("coordinator")?;
    println!("Active subscriptions: {:?}", subs);

    // Leave thread when done
    runtime.unsubscribe_agent("coordinator", "thread.project-99.broadcast").await?;

    Ok(())
}
```

Integration Tests

- `tests/integration/e2e_dynamic_subscription.rs` — Dynamic subscription scenarios
- `tests/agent_runtime_test.rs` — Basic lifecycle and static subscriptions
