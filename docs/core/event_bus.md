## Event Bus

Responsibility

- Reliable in-process event publication and subscription.
- Supports QoS, backpressure strategies, and flexible dispatch semantics so components can communicate through events rather than direct calls.

Key files

- `core/src/event.rs` — canonical event types, envelopes, and helper extensions (EventExt trait).
- `core/benches/event_bus_benchmark.rs` — benchmarks for throughput and latency.

Key concepts

- Publishers and subscribers: components subscribe to event types or filters.
- QoS: priorities or levels applied to event delivery (used to control ordering/importance).
- Backpressure: sampling, drop-old, or aggregate strategies are applied when consumers cannot keep up.
- **Event Helpers**: Fluent API for envelope metadata via `EventExt` trait.

## Event Helper Functions

The `EventExt` trait provides fluent helpers for reading and writing envelope metadata:

**Write helpers** (chainable):

```rust
use loom_core::{Event, EventExt};

let event = Event { /* ... */ }
    .with_thread("task-123".to_string())
    .with_correlation("task-123".to_string())
    .with_sender("agent.coordinator".to_string())
    .with_reply_to("thread.task-123.reply".to_string());
```

**Read helpers**:

```rust
let thread_id = event.thread_id();       // Option<&str>
let corr_id = event.correlation_id();    // Option<&str>
let sender = event.sender();             // Option<&str>
let reply_to = event.reply_to();         // Option<&str>
```

**Benefits**:

- Reduces boilerplate (no need for `Envelope::from_event()` / `attach_to_event()` for simple cases)
- Fluent chaining for building events
- Type-safe access to envelope fields
- Compatible with existing `Envelope` API

**When to use**:

- Use helpers for simple metadata access and event construction
- Use `Envelope` when you need full envelope features (TTL, hop tracking, topic helpers)

Common error paths and test cases

- Subscription/Unsubscription correctness: ensure handlers are not called after unsubscribe.
- Empty or malformed events: validate that invalid events are filtered or cause a well-defined error path.
- Backpressure edge cases: sampling correctness under sustained overload; verify P50/P99 latency change.

Tuning and operational knobs

- Batch sizes for dispatch loops.
- Sampling window and thresholds for drop or aggregate policies.
- Subscriber concurrency limits and mailbox sizes.

Example (pseudocode)

```text
// publisher
event_bus.publish(Event::new("input"))

// subscriber
event_bus.subscribe(|e| { /* handle */ })
```

Notes

- Unit tests should include subscribe/unsubscribe, QoS enforcement, and explicit backpressure scenarios.
- See `tests/event_helpers_test.rs` for Event helper usage examples and patterns.
