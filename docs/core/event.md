# Event

`Event` is the fundamental message type carried by the Event Bus. It encapsulates a typed payload together with metadata used for routing, correlation, and tracing. This document describes the Event structure, metadata conventions, lifecycle, and best practices.

## Structure

Events are defined in `proto` and used across languages. A typical Rust view looks like:

```rust
pub struct Event {
    pub id: String,
    pub r#type: String,
    pub metadata: std::collections::HashMap<String, String>,
    pub payload: Vec<u8>,
    pub timestamp_ms: i64,
    pub source: String,
    pub confidence: f32,
    pub tags: Vec<String>,
    pub priority: i32,
}
```

Field semantics:

- `id`: unique identifier for the event instance.
- `type`: semantic name of the event, e.g., `tick`, `tool.result`, `agent.message`.
- `metadata`: string map carrying envelope and custom headers (see below).
- `payload`: opaque bytes; often JSON. Choose a stable schema for cross-service use.
- `timestamp_ms`: event creation timestamp (ms since epoch).
- `source`: origin of the event (feed/service/agent).
- `confidence`: producer’s confidence score (0–1); domain-specific.
- `tags`: freeform labels for filtering.
- `priority`: scheduling hint (higher means higher priority); used by higher layers.

## Envelope metadata (reserved keys)

Envelope standardizes coordination and tracing across the system. It lives inside `metadata` using these reserved keys:

- `thread_id`, `correlation_id`, `sender`, `reply_to`
- `ttl` (remaining hops), `hop` (current hop count), `ts` (timestamp)
- `trace_id`, `span_id`, `trace_flags` (OpenTelemetry)

See `docs/core/envelope.md` for detailed behavior and lifecycle. Prefer manipulating these via `Envelope` APIs instead of manually editing the map.

## Convenience API: EventExt

`EventExt` adds fluent helpers to read/write common envelope fields on `Event`:

```rust
use loom_core::EventExt;

let evt = Event { /* ... */ };
let evt = evt
  .with_thread("thread-1".to_string())
  .with_correlation("thread-1".to_string())
  .with_reply_to("thread.thread-1.reply".to_string())
  .with_sender("agent.publisher".to_string());

let sender = evt.sender();       // Option<&str>
let thread = evt.thread_id();    // Option<&str>
```

Notes:

- `sender()` is compatible with Rust and Python SDKs (`"sender"` or `"loom.sender"`).
- These helpers are handy, but for full behavior (TTL/hop/trace) use `Envelope`.

## Lifecycle

1. Create: Producer creates an `Event` with `id`, `type`, and payload.
2. Attach envelope: Use `Envelope::new(thread_id, sender)` and `env.attach_to_event(&mut evt)`.
   - Call `env.inject_trace_context()` to include current OpenTelemetry context.
3. Publish: `EventBus::publish(topic, evt).await` handles routing and metrics.
4. Consume: Subscriber receives `Event`. Extract envelope via `Envelope::from_event(&evt)`.
   - Call `env.extract_trace_context()` to link traces.
   - If forwarding, call `env.next_hop()`; drop if it returns `false` (TTL expired), otherwise re-attach.

## JSON example

Human-readable example (payload encoded as JSON string for illustration):

```json
{
  "id": "evt-123",
  "type": "tool.result",
  "metadata": {
    "thread_id": "req-42",
    "correlation_id": "req-42",
    "sender": "agent.worker-1",
    "reply_to": "thread.req-42.reply",
    "ttl": "16",
    "hop": "0",
    "ts": "1731916800000",
    "trace_id": "f3e1c1b8e0d7c8f0f1e2d3c4b5a69788",
    "span_id": "a1b2c3d4e5f60708",
    "trace_flags": "01"
  },
  "payload": "{\"result\":\"ok\",\"value\":123}",
  "timestamp_ms": 1731916800000,
  "source": "broker.translate",
  "confidence": 1.0,
  "tags": ["translate", "demo"],
  "priority": 50
}
```

## Best practices

- Prefer `Envelope` to set thread/correlation/reply routing and tracing.
- Use stable `type` names; keep them short and namespaced (e.g., `tool.result`).
- Document payload schema; if JSON, consider versioning or `type` sub-variants.
- Set meaningful `priority` only if your runtime/router uses it.
- Keep metadata small; reserve large data for `payload`.
- Validate TTL in agent loops with `next_hop()` to avoid forwarding loops.

## Minimal send/receive

```rust
use loom_core::{Envelope, proto::Event};
use std::collections::HashMap;

// Producer
let mut evt = Event {
    id: "evt-1".into(),
    r#type: "agent.message".into(),
    metadata: HashMap::new(),
    payload: br#"{\"text\":\"hello\"}"#.to_vec(),
    timestamp_ms: chrono::Utc::now().timestamp_millis(),
    source: "agent.ui".into(),
    confidence: 1.0,
    tags: vec![],
    priority: 50,
};
let mut env = Envelope::new("chat-1", "agent.ui");
env.inject_trace_context();
env.attach_to_event(&mut evt);
bus.publish("chat.messages", evt).await?;

// Consumer
let (id, mut rx) = bus.subscribe("chat.messages".into(), vec![], QoSLevel::QosBatched).await?;
if let Some(evt) = rx.recv().await {
    let mut env = Envelope::from_event(&evt);
    env.extract_trace_context();
    if env.next_hop() {
        // ... process or forward ...
    }
}
```
