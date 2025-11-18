# Event Bus

The Event Bus is the in-process message hub that delivers Events from publishers to subscribers with QoS-aware backpressure, topic-based routing (exact and simple wildcard), OpenTelemetry metrics/tracing, and optional Dashboard/FlowTracker visualization.

This page explains responsibilities, key concepts, delivery guarantees, metrics, and how to use the API with concise examples.

## Overview

- Purpose: decouple producers/consumers via topics; enforce QoS policies; surface health/latency with metrics; propagate distributed tracing context via Envelopes.
- Where it lives: `core/src/messaging/` module (event_bus.rs, event_ext.rs, envelope.rs).
- Interop: Events carry coordination metadata in `Event.metadata`, managed by `Envelope` (see `docs/core/envelope.md`).

## Concepts

- Topic: string channel used to route events. Supports exact match and simple wildcard prefixes (`prefix.*` matches topics that start with `prefix.` and have one more segment).
- Subscription: a bounded queue (`tokio::mpsc`) receiving events for a given topic (and optionally filtered by event type). Each subscription has a unique `subscription_id`.
- QoS (Quality of Service): controls latency vs. reliability trade-offs per subscription.
  - `QosRealtime`: low-latency; drop on pressure or full queue; never block.
  - `QosBatched`: throughput oriented; wait for queue capacity (bounded, backpressure applies).
  - `QosBackground`: similar to batched with larger queue for bulk/low-priority work.
- Backpressure: global per-topic backlog threshold that, when exceeded, causes realtime deliveries to be dropped aggressively to protect the system.
- Envelope: standardized metadata for thread/correlation/reply routing/TTL/tracing. EventBus injects current trace context into the Envelope on publish. See `docs/core/envelope.md`.

## Delivery model

1. Publish
   - Caller invokes `EventBus::publish(topic, event)`. The bus converts the event into an `Envelope`, injects the current OpenTelemetry context, and writes it back into `event.metadata`.
   - The bus increments per-topic backlog and emits metrics.
2. Match subscribers
   - Exact topic matches and wildcard prefix matches (`pattern.*`).
   - Optional event-type filter: a subscriber may specify `event_types`; non-matching events are skipped for that subscriber.
3. Deliver with QoS policy
   - Realtime: if global backlog is above threshold or the subscriber queue is full, the event is dropped (no await).
   - Batched/Background: enqueue with `send().await` (bounded mpsc); may await for capacity, applying natural backpressure.
4. Visualize & trace
   - Optionally emits Dashboard events (`EventPublished`, `EventDelivered`) and FlowTracker edges (`sender -> EventBus -> subscriber`).
   - Records publish latency histogram; increments delivered/dropped counters with reasons.

## Backpressure

- Threshold: `backpressure_threshold` (default `10_000`) measures per-topic backlog. When `>= threshold`, realtime deliveries are dropped aggressively.
- Span annotation: the current span records `backpressure=true` when threshold is exceeded.
- Drop reasons: exported as metric attribute `reason` with values `backpressure` or `queue_full`.
- Tuning: adjust in code today (field on `EventBus`); consider exposing via config if your deployment needs dynamic tuning.

## Metrics (OpenTelemetry)

All metrics use meter name `loom.event_bus`.

- `loom.event_bus.published_total` (u64 counter)
  - Attr: `topic`, `event_type`
- `loom.event_bus.delivered_total` (u64 counter)
  - Attr: `topic`
- `loom.event_bus.dropped_total` (u64 counter)
  - Attr: `topic`, `reason` (`backpressure`|`queue_full`)
- `loom.event_bus.backlog_size` (i64 up-down counter)
  - Attr: `topic`
- `loom.event_bus.active_subscriptions` (i64 up-down counter)
  - Attr: `topic`
- `loom.event_bus.publish_latency_ms` (f64 histogram)
  - Attr: `topic`

Example questions you can answer with metrics:

- Is any topic saturated? Check high `backlog_size` and `dropped_total{reason="backpressure"}`.
- Are consumers overwhelmed? Look for `dropped_total{reason="queue_full"}` spikes.
- What is end-to-end publish latency trend? Inspect `publish_latency_ms` percentiles.

## Tracing (OpenTelemetry)

- On publish, the bus creates an Envelope and calls `inject_trace_context()` so `trace_id/span_id/trace_flags` are stored in metadata.
- Consumers should extract: `let env = Envelope::from_event(&evt); env.extract_trace_context();` to link spans across processes.

## Topic matching

- Exact: `topic == subscription.topic`.
- Wildcard: subscription patterns ending with `.*` match topics that start with the prefix and have one additional segment (e.g., `market.price.*` matches `market.price.BTC`).

## API snippets

### Create and start the bus

```rust
use loom_core::EventBus;

let bus = EventBus::new().await?;
bus.start().await?;
```

### Subscribe with QoS and type filter

```rust
use loom_core::{proto::QoSLevel, proto::Event};

let (sub_id, mut rx) = bus
        .subscribe(
                "market.price.BTC".to_string(),
                vec!["tick".to_string()], // empty vec means all types
                QoSLevel::QosBatched,
        )
        .await?;

tokio::spawn(async move {
        while let Some(evt) = rx.recv().await {
                // Extract envelope and adopt remote trace parent
                let env = loom_core::Envelope::from_event(&evt);
                env.extract_trace_context();
                // TTL/hop guard in your agent loop (if forwarding)
                // ... process evt ...
        }
});
```

### Publish an event (with envelope metadata and tracing)

```rust
use loom_core::{proto::Event, Envelope};
use std::collections::HashMap;

let mut evt = Event {
        id: "evt-1".into(),
        r#type: "tick".into(),
        metadata: HashMap::new(),
        payload: br#"{\"p\": 68000.0}"#.to_vec(),
        timestamp_ms: chrono::Utc::now().timestamp_millis(),
        source: "feed.okx".into(),
        confidence: 1.0,
        tags: vec!["btc".into()],
        priority: 50,
};

// Prepare envelope and attach (sets thread/correlation/reply_to/ttl/hop/timestamp)
let mut env = Envelope::new("thread-abc", "agent.publisher");
env.inject_trace_context();
env.attach_to_event(&mut evt);

let delivered = bus.publish("market.price.BTC", evt).await?;
```

### Unsubscribe

```rust
bus.unsubscribe(&sub_id).await?;
```

## Dashboard & FlowTracker (optional)

If configured via `set_dashboard_broadcaster()` and/or `set_flow_tracker()`, the bus:

- Emits `EventPublished` and `EventDelivered` events with a small payload preview.
- Records directed edges for flow visualization: `sender -> EventBus -> subscriber`.

These hooks are noop unless explicitly set by the runtime.

## Stats (programmatic)

`EventBus::get_stats(topic)` returns per-topic counters:

- `total_published`, `total_delivered`, `dropped_events`, `active_subscriptions`, `backlog_size`.

## Troubleshooting

- No subscribers: WARN log and backlog decremented; check topic naming and wildcard usage.
- High realtime drops: consider switching critical consumers to `QosBatched` or lowering publish rate; tune `backpressure_threshold`.
- Queue full drops: consumer too slow or queue too small; scale out or increase queue cap (QoS-dependent).
- Missing traces: ensure you call `extract_trace_context()` on the consumer side and that tracing is initialized.

## Notes & design choices

- Bounded queues prevent unbounded memory growth; QoS allows per-subscriber trade-offs.
- `DashMap` is used for low-contention concurrent access to subscriptions and stats.
- A `broadcast` channel is present for potential future high-priority fanout; currently unused.

## Event Bus

Responsibility

- Reliable in-process event publication and subscription.
- Supports QoS, backpressure strategies, and flexible dispatch semantics so components can communicate through events rather than direct calls.

Key files

- `core/src/messaging/event_bus.rs` — EventBus implementation with QoS and backpressure.
- `core/src/messaging/event_ext.rs` — EventExt trait for fluent envelope helpers.
- `core/src/messaging/envelope.rs` — Envelope coordination metadata.
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
