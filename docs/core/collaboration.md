# Collaboration Primitives (Threads + Semantics)

This document defines lightweight, event-based collaboration primitives built on top of Loom Core's EventBus and Envelope conventions.

## Envelope Conventions

We standardize a small set of metadata keys carried in `Event.metadata` and `ActionCall.headers`:

- thread_id: Correlates messages within a collaboration thread
- correlation_id: Links a reply/proposal/award to its originating request/CFP
- sender: Logical identity of the publisher (e.g., agent.foo)
- reply_to: Canonical reply topic for this thread (thread-scoped or agent-specific)
- ttl: Remaining hops budget (integer), decremented per hop
- hop: Hop counter, incremented per hop
- ts: Millisecond timestamp at emission

Topic naming:

**Thread-scoped (collaboration)**:

- Thread broadcast: `thread.{thread_id}.broadcast`
- Thread reply: `thread.{thread_id}.reply`

**Agent-specific (point-to-point)**:

- Agent private mailbox: `agent.{agent_id}.replies`
- Every agent is automatically subscribed to its private reply topic

Helpers available via `Envelope`:

- `new(thread_id, sender)` - Creates envelope with thread reply topic
- `with_agent_reply(thread_id, sender, agent_id)` - Creates envelope with agent reply topic
- `from_event(&Event)`
- `attach_to_event(&mut Event)`
- `apply_to_action_call(&mut ActionCall)`
- `next_hop()` -> bool
- `broadcast_topic()` / `reply_topic()` - Thread-scoped topics
- `agent_reply_topic()` - Extract agent private topic from sender

Standalone helper:

- `agent_reply_topic(agent_id)` - Build agent private topic

## Control Event Types

We use a few reserved `Event.type` values for coordination:

- collab.request / collab.reply
- collab.cfp / collab.proposal / collab.award
- collab.barrier (optional heartbeat)
- collab.timeout / collab.summary (observability)

## Collaborator API

The `Collaborator` wraps these conventions to implement common patterns:

### request_reply(topic, payload, timeout_ms) -> Result<Option<Event>>

- Subscribes to thread reply topic, publishes a `collab.request`, waits for first `collab.reply` with matching correlation.
- Returns `Ok(Some(Event))` on successful reply, `Ok(None)` on timeout.
- Returns `Err` if `timeout_ms == 0` (validation failure).
- Emits `collab.timeout` on reply topic if timed out.

**Parameters:**

- `timeout_ms`: Must be > 0, otherwise returns error.

### fanout_fanin(topics, payload, first_k, timeout_ms) -> Result<Vec<Event>>

- Broadcasts `collab.request` to multiple topics and collects up to `first_k` replies within timeout.
- Returns collected replies (may be fewer than `first_k` if timeout).
- Returns `Err` if `first_k == 0` or `timeout_ms == 0` (validation failures).
- Returns `Ok(Vec::new())` if `topics.is_empty()` (no-op).
- Emits `collab.summary` on the thread reply topic with received count and target.

**Parameters:**

- `first_k`: Must be > 0, otherwise returns error.
- `timeout_ms`: Must be > 0, otherwise returns error.

### contract_net(thread_id, cfp_payload, window_ms, max_awards) -> Result<Vec<Event>>

- Publishes `collab.cfp` to `thread.{thread_id}.broadcast`, listens on reply topic for `collab.proposal`, ranks by `metadata.score` (desc), publishes `collab.award` for winners, and emits a `collab.summary`.
- Returns top `max_awards` proposals sorted by score (descending).
- Returns `Err` if `window_ms == 0` or `max_awards == 0` (validation failures).
- Proposals without valid `score` metadata are treated as score 0.0.

**Parameters:**

- `window_ms`: Must be > 0, otherwise returns error.
- `max_awards`: Must be > 0, otherwise returns error.

## Best Practices

- Always include `sender` in envelopes for accountability.
- Use `ttl` to guard against runaway loops. Drop when `next_hop()` returns false.
- For proposals, include a numeric `score` in metadata to enable generic ranking.
- Keep payload formats minimal and agreed by participants; metadata carries coordination.

## Reply Semantics: Thread vs Agent

### Thread Reply (Multi-Agent Collaboration)

Use thread reply topics when coordination involves multiple participants:

```rust
// Request-reply in a thread
let env = Envelope::new("task-123", "agent.coordinator");
// env.reply_to = "thread.task-123.reply"

// All participants subscribed to thread.task-123.reply will receive responses
```

**Use cases:**

- Contract-net protocol (multiple proposals)
- Fanout-fanin (collecting from multiple workers)
- Broadcast questions to group

### Agent Reply (Point-to-Point)

Use agent private topics for direct agent-to-agent communication:

```rust
// Direct request to specific agent
let env = Envelope::with_agent_reply("req-456", "agent.requester", "expert");
// env.reply_to = "agent.expert.replies"

// Only the requester receives the response (automatically subscribed)
```

**Use cases:**

- Expert consultation (one-on-one)
- Task delegation (supervisor → worker)
- Private handshake or negotiation

### Auto-subscription

Every agent is automatically subscribed to `agent.{agent_id}.replies` at creation.
No explicit subscription needed for receiving direct messages.

### Example: Hybrid Pattern

```rust
// Phase 1: Broadcast to find experts
let cfp_env = Envelope::new("project-1", "agent.coordinator");
bus.publish("thread.project-1.broadcast", cfp_event).await?;

// Phase 2: Direct communication with selected expert
let direct_env = Envelope::with_agent_reply("project-1", "agent.coordinator", "expert-3");
bus.publish("agent.expert-3.replies", task_event).await?;
```

## Examples

See integration tests:

- `core/tests/collab_test.rs` - Thread-scoped collaboration (request/reply, fanout/fanin, contract-net)
- `core/tests/integration/e2e_agent_reply.rs` - Point-to-point agent communication
