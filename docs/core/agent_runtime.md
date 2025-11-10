## Agent Runtime

Responsibility

- Manage agent lifecycle, mailboxing and dispatch of messages to agent behaviors.
- Provide isolation between agents and hooks for persistence and state management.

Key files

- `core/src/agent/runtime.rs` — runtime loop and scheduling.
- `core/src/agent/instance.rs` — agent instance representation and state machine.
- `core/src/agent/behavior.rs` — behavior abstractions.

Key interfaces

- Mailbox API: enqueue/dequeue messages with backpressure handling.
- Lifecycle hooks: start, stop, checkpoint, and restore.

Common error paths and test cases

- Lifecycle transitions: invalid state transitions (start -> start, stop -> execute) should be guarded.
- Mailbox overflow and cancellation: verify messages are handled or discarded according to policy.
- Persistence mismatch: state read/write failures are surfaced and cause deterministic fallback behavior.

Tuning knobs

- Mailbox capacity per agent.
- Scheduling quantum and priority.
- Checkpoint interval for durable state.

Example
Refer to `core/src/agent/` for integration points used by tests and example agents.
