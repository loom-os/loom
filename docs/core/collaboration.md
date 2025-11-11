# Collaboration Primitives (Threads + Semantics)

This document defines lightweight, event-based collaboration primitives built on top of Loom Core's EventBus and Envelope conventions.

## Envelope Conventions

We standardize a small set of metadata keys carried in `Event.metadata` and `ActionCall.headers`:

- thread_id: Correlates messages within a collaboration thread
- correlation_id: Links a reply/proposal/award to its originating request/CFP
- sender: Logical identity of the publisher (e.g., agent.foo)
- reply_to: Canonical reply topic for this thread
- ttl: Remaining hops budget (integer), decremented per hop
- hop: Hop counter, incremented per hop
- ts: Millisecond timestamp at emission

Topic naming:

- Thread broadcast: `thread.{thread_id}.broadcast`
- Thread reply: `thread.{thread_id}.reply`

Helpers available via `Envelope`:

- new(thread_id, sender)
- from_event(&Event)
- attach_to_event(&mut Event)
- apply_to_action_call(&mut ActionCall)
- next_hop() -> bool
- broadcast_topic()/reply_topic()

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

## Examples

See `core/tests/collab_test.rs` for E2E examples of request/reply, fanout/fanin, and contract-net.
