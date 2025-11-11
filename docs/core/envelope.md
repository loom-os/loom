# Envelope: Thread & Correlation Semantics

The Envelope is a small, unified metadata block that travels with every Event and ActionCall. It provides thread and correlation semantics so multi‑agent workflows can coordinate reliably.

Fields:

- thread_id: Stable conversation/request id; scopes broadcast/replies.
- correlation_id: Per‑message correlation token (defaults to thread_id). Used to tie responses to requests.
- sender: Logical identity of the emitter, e.g., `agent.voice` or `broker.weather`.
- reply_to: Topic to receive replies for this thread. Convention: `thread.{thread_id}.reply`.
- ttl: Remaining hop budget. Decremented by each agent hop; messages are dropped when ttl <= 0.
- hop: Hop counter, incremented on each agent.
- ts: Millisecond timestamp of envelope creation.

Topic conventions:

- thread.{id}.broadcast — multicast to all participants in a thread
- thread.{id}.reply — unicast replies back to the requester or coordinator

Lifecycle:

- Events: Envelope is stored in `Event.metadata`. Agents ensure it exists, increment `hop`, decrement `ttl`, and drop when exhausted, then write it back.
- Actions: Envelope is stored in `ActionCall.headers` and `ActionCall.correlation_id` is set; the ActionBroker and providers can use it for tracing.

Helpers:

- Envelope::new(thread_id, sender)
- Envelope::from_event(&Event) / from_metadata(&HashMap)
- Envelope::attach_to_event(&mut Event)
- Envelope::apply_to_action_call(&mut ActionCall)
- Envelope::next_hop() -> bool (increments hop, decrements ttl, returns ttl > 0)
- ThreadTopicKind::{Broadcast, Reply}.topic(thread_id)

Interaction with collaboration:

- The Collaborator APIs publish control events (request/reply/cfp/proposal/award) on `thread.{id}.broadcast`/`reply` and use Envelope to ensure correlation and TTL management.

Testing guidance:

- Unit tests: roundtrip via metadata; topic helpers; next_hop TTL behavior.
- Integration tests: agent drops TTL=1 before behavior; ActionBroker receives headers with correlation_id == call.id.
