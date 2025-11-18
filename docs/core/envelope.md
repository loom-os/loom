# Envelope: Coordination & Tracing Metadata

Envelope is the unified coordination header that rides inside `Event.metadata` and `ActionCall.headers`. It standardizes thread/correlation, reply routing, hop/TTL safety, timestamps, and distributed tracing across the runtime.

It is implemented in `core/src/envelope.rs` and consumed broadly (Event Bus, Agents, Action Broker, Bridge).

## Reserved Keys

Envelope fields are stored as string keys in metadata. The canonical keys are exposed as `envelope::keys::*`:

- `thread_id`: groups related messages in a collaboration session
- `correlation_id`: links replies/proposals to the originating request (default = `thread_id`)
- `sender`: logical identity of the sender (e.g., `agent.worker-1`, `broker.translate`)
- `reply_to`: canonical reply topic (defaults to `thread.{thread_id}.reply`)
- `ttl`: remaining hop budget (int). Prevents infinite loops
- `hop`: hop counter (uint), incremented on each forwarding
- `ts`: creation timestamp (ms since epoch)
- `trace_id`, `span_id`, `trace_flags`: OpenTelemetry context for distributed tracing

Notes:

- Trace keys are only written when non-empty to preserve backward compatibility.
- Python SDK compatibility: some producers may use `loom.sender`. `EventExt::sender()` reads either `sender` or `loom.sender`.

## Topic Conventions

Thread-scoped topics (multi-agent collaboration):

- `thread.{thread_id}.broadcast` — fan-out to participants of a thread
- `thread.{thread_id}.reply` — canonical reply path for the thread

Agent mailbox topics (point-to-point):

- `agent.{agent_id}.replies` — private mailbox per agent
- Helper: `agent_reply_topic(agent_id)` builds this topic

`ThreadTopicKind::{Broadcast, Reply}.topic(thread_id)` provides safe builders for thread topics.

## Envelope API

### Create

```rust
use loom_core::Envelope;

let env = Envelope::new("task-42", "agent.coordinator");
// Defaults: correlation_id=thread_id, ttl=16, hop=0,
// reply_to=thread.{thread_id}.reply, timestamp_ms=now
```

### Create with agent reply

```rust
let env = Envelope::with_agent_reply("task-1", "agent.coordinator", "coordinator");
// reply_to = agent.coordinator.replies
```

### Attach / Extract from Event

```rust
use loom_core::{Envelope, proto::Event};
use std::collections::HashMap;

let mut evt = Event { /* ... */ metadata: HashMap::new(), /* ... */ };
let mut env = Envelope::new("thread-1", "agent.sender");
env.inject_trace_context();        // capture current trace into the envelope
env.attach_to_event(&mut evt);     // write envelope into evt.metadata

let env2 = Envelope::from_event(&evt); // parse from evt.metadata with fallback
```

### Apply to ActionCall

```rust
use loom_core::{Envelope, proto::ActionCall};

let env = Envelope::new("thread-1", "agent.sender");
let mut call = ActionCall { /* ... */ headers: Default::default(), correlation_id: String::new(), /* ... */ };
env.apply_to_action_call(&mut call);
assert_eq!(call.correlation_id, env.correlation_id);
```

### Hop / TTL guard

```rust
let mut env = Envelope::new("thread-1", "agent.sender");
if !env.next_hop() { /* drop expired */ }
// increments hop, decrements ttl; returns false when ttl <= 0
```

### Topic helpers

```rust
let env = Envelope::new("req-1", "agent.worker-1");
env.broadcast_topic(); // => thread.req-1.broadcast
env.reply_topic();     // => thread.req-1.reply
env.agent_reply_topic(); // => agent.worker-1.replies (derived from sender)
```

## Tracing Integration (OpenTelemetry)

Cross-process tracing is first-class:

- Inject on produce: `inject_trace_context()` copies the current span’s OTel context (`trace_id`, `span_id`, `trace_flags`) into the envelope before sending across the Bridge or Event Bus.
- Extract on consume: `extract_trace_context()` parses the envelope and sets the current span’s parent to the remote context. This stitches traces across services.

```rust
let mut env = Envelope::new("thread-1", "agent.sender");
env.inject_trace_context();
// ... send ...
let env = Envelope::from_event(&evt);
env.extract_trace_context(); // returns true on success
```

Implementation details:

- Uses `opentelemetry` + `tracing_opentelemetry` under the hood.
- If trace fields are absent or invalid hex, extraction is a no-op (returns `false`).

## Metadata Roundtrip & Defaults

`Envelope::from_metadata(meta, fallback_event_id)` is robust to missing fields:

- `thread_id`: from metadata or `fallback_event_id`
- `correlation_id`: defaults to `thread_id`
- `sender`: empty string if missing
- `reply_to`: defaults to `thread.{thread_id}.reply`
- `ttl`: parsed int or default `16`
- `hop`: parsed uint or default `0`
- `timestamp_ms`: parsed or `now()`
- `trace_*`: empty if absent

`apply_to_metadata(map)` writes all basic fields and only writes trace fields when non-empty.

## Patterns

Common collaboration patterns enabled by Envelope:

- Request–Reply (thread-scoped): producer sets `reply_to=thread.{id}.reply`; responders preserve `correlation_id`.
- Fan-out/Fan-in: publish proposals to `thread.{id}.broadcast`, reply via `thread.{id}.reply`.
- Direct Mailbox: for targeted responses, set `reply_to=agent.{id}.replies` (via `with_agent_reply`).

## Best Practices

- Always attach an Envelope before publishing or invoking tools; prefer `Envelope` APIs over manual map edits.
- On forwarding, call `next_hop()` and drop expired messages to avoid loops.
- Choose reply routing deliberately: thread replies for coordination; agent mailbox for P2P.
- Keep metadata modest; large data belongs in `payload`.
- For cross-language interop, ensure `sender` is present; readers should tolerate `loom.sender`.

## Pitfalls & Troubleshooting

- Missing replies: verify `reply_to` selection (thread vs agent mailbox) and topic names.
- TTL expired: events silently dropped by your agent loop if `next_hop()` returns false; tune TTL if needed (default 16).
- Traces not linked: ensure `inject_trace_context()` on produce and `extract_trace_context()` on consume, and initialize OTel exporter.
- Inconsistent sender key: ensure producers set `sender`; consumers may rely on it to compute mailbox topic.

## See Also

- Event Bus delivery, QoS, and backpressure: `docs/core/event_bus.md`
- Event structure and lifecycle: `docs/core/event.md`
