## Event Bus

Responsibility

- Reliable in-process event publication and subscription.
- Supports QoS, backpressure strategies, and flexible dispatch semantics so components can communicate through events rather than direct calls.

Key files

- `core/src/event.rs` — canonical event types and envelopes.
- `core/benches/event_bus_benchmark.rs` — benchmarks for throughput and latency.

Key concepts

- Publishers and subscribers: components subscribe to event types or filters.
- QoS: priorities or levels applied to event delivery (used to control ordering/importance).
- Backpressure: sampling, drop-old, or aggregate strategies are applied when consumers cannot keep up.

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
