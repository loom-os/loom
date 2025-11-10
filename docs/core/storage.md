## Storage

Responsibility

- Abstract storage for short-term (ephemeral) and persistent state required by agents and other components.

Key files

- `core/src/storage.rs` — storage trait and implementations.

Storage modes

- Ephemeral (in-memory): low-latency, non-durable state suitable for transient agent context.
- Persistent: durable key/value or object stores used for checkpoints and long-term agent state.

Common error paths and test cases

- Persistence failures: ensure state write/read failures surface deterministic errors and do not silently corrupt runtime.
- Consistency and race conditions: concurrent read/write tests to validate locking or versioning guarantees.

Configuration and tuning

- Checkpoint frequency and batching.
- Max object size and eviction policy for ephemeral stores.

Notes

- Storage implementations should be mocked during unit tests and exercised for error injection in integration tests.
