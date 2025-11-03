# Event Bus Backpressure & QoS

This document describes how Loom's EventBus applies backpressure and QoS to ensure stable behavior under load.

## QoS levels

- QosRealtime
  - Per-subscriber queue: small (64)
  - Delivery policy: best-effort. If the subscriber queue is full or the bus is over its backpressure threshold, the event is dropped.
- QosBatched
  - Per-subscriber queue: medium (1024)
  - Delivery policy: queued. Publishing awaits enqueue into the bounded channel (no unbounded memory growth). If the subscriber is closed, the event is dropped.
- QosBackground
  - Per-subscriber queue: large (4096)
  - Delivery policy: queued. Same semantics as batched.

## Backpressure threshold

- EventBus maintains a per-topic `backlog_size` counter and a global `backpressure_threshold` (default: 10,000).
- On publish, `backlog_size` is incremented before dispatch and decremented afterwards. This reflects the number of in-flight publishes per topic, not per-subscriber queue depths.
- When `backlog_size >= backpressure_threshold`, realtime deliveries to that topic are dropped early to reduce load. Batched/background deliveries continue to use bounded queues and may await.

## Counters and stats

Per topic, the bus tracks:
- `total_published`: number of publish attempts
- `total_delivered`: number of successful deliveries to subscribers
- `dropped_events`: number of events dropped by policy or due to closed/full channels
- `active_subscriptions`: number of active subscriptions to the topic
- `backlog_size`: approximate in-flight publish backlog

These are accessible via `EventBus::get_stats(topic)`.

## Memory safety

- All queues are bounded; there is no unbounded memory growth. Realtime drops on pressure; batched/background await enqueue into bounded channels.

## Notes

- `backlog_size` is an approximate metric at the bus level and does not include per-subscriber queue depths; it's sufficient for threshold gating.
- Queue capacities are conservative defaults and can be made configurable in the future.
