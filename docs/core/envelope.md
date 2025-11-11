# Envelope: Thread & Correlation Semantics

The Envelope is a small, unified metadata block that travels with every Event and ActionCall. It provides thread and correlation semantics so multi‑agent workflows can coordinate reliably.

Fields:

- thread_id: Stable conversation/request id; scopes broadcast/replies.
- correlation_id: Per‑message correlation token (defaults to thread_id). Used to tie responses to requests.
- sender: Logical identity of the emitter, e.g., `agent.voice` or `broker.weather`.
- reply_to: Topic to receive replies for this thread. Convention: `thread.{thread_id}.reply` for thread-scoped replies, or `agent.{agent_id}.replies` for agent-specific private replies.
- ttl: Remaining hop budget. Decremented by each agent hop; messages are dropped when ttl <= 0.
- hop: Hop counter, incremented on each agent.
- ts: Millisecond timestamp of envelope creation.

Topic conventions:

**Thread-scoped topics** (multi-agent collaboration):

- `thread.{id}.broadcast` — multicast to all participants in a thread
- `thread.{id}.reply` — unicast replies back to the requester or coordinator

**Agent-specific topics** (point-to-point communication):

- `agent.{agent_id}.replies` — private mailbox for direct agent-to-agent messages
- Every agent is automatically subscribed to its private reply topic at creation

## Reply Topic Semantics

### Thread Reply (collaboration)

Use when replies should go to all participants or the thread coordinator:

```rust
let env = Envelope::new("task-123", "agent.worker");
// env.reply_to = "thread.task-123.reply"
```

### Agent Reply (point-to-point)

Use when replies should go directly to a specific agent:

```rust
let env = Envelope::with_agent_reply("task-123", "agent.worker", "coordinator");
// env.reply_to = "agent.coordinator.replies"
```

### Dynamic Reply Selection

Extract reply topic from sender:

```rust
let env = Envelope::new("req-1", "agent.helper");
let agent_topic = env.agent_reply_topic(); // "agent.helper.replies"
let thread_topic = env.reply_topic();      // "thread.req-1.reply"
```

Lifecycle:

- Events: Envelope is stored in `Event.metadata`. Agents ensure it exists, increment `hop`, decrement `ttl`, and drop when exhausted, then write it back.
- Actions: Envelope is stored in `ActionCall.headers` and `ActionCall.correlation_id` is set; the ActionBroker and providers can use it for tracing.

Helpers:

- `Envelope::new(thread_id, sender)` - Creates envelope with thread reply topic
- `Envelope::with_agent_reply(thread_id, sender, agent_id)` - Creates envelope with agent reply topic
- `Envelope::from_event(&Event)` / `from_metadata(&HashMap)`
- `Envelope::attach_to_event(&mut Event)`
- `Envelope::apply_to_action_call(&mut ActionCall)`
- `Envelope::next_hop()` -> bool (increments hop, decrements ttl, returns ttl > 0)
- `Envelope::agent_reply_topic()` -> String - Extracts agent private topic from sender
- `Envelope::broadcast_topic()` -> String - Returns thread broadcast topic
- `Envelope::reply_topic()` -> String - Returns thread reply topic
- `ThreadTopicKind::{Broadcast, Reply}.topic(thread_id)`
- `agent_reply_topic(agent_id)` - Standalone helper for building agent reply topics

Interaction with collaboration:

- The Collaborator APIs publish control events (request/reply/cfp/proposal/award) on `thread.{id}.broadcast`/`reply` and use Envelope to ensure correlation and TTL management.
- Agents automatically receive messages on their private `agent.{id}.replies` topic for direct communication.

Testing guidance:

- Unit tests: roundtrip via metadata; topic helpers; next_hop TTL behavior; agent_reply_topic extraction.
- Integration tests: agent drops TTL=1 before behavior; ActionBroker receives headers with correlation_id == call.id; private reply topic isolation.
- See `tests/integration/e2e_agent_reply.rs` for point-to-point communication tests.
