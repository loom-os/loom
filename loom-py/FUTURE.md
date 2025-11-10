# Loom Python SDK Future Work

Short-term (pre-0.1.0):

- Add pytest suite: mock bridge server or use in-process tonic server via subprocess for registration, emit/reply, forward_action.
- Barrier primitive: wait for N correlated replies or timeout (Context.barrier([...])).
- Thread broadcast convenience: Context.broadcast(thread_id, type, payload).
- Structured logging + traces (correlation_id propagation).

Medium-term (0.1.x):

- Memory plugin interface & pluggable backends (Redis, SQLite, DuckDB, local vector store).
- Async capability invocation concurrency limits; cancellation/timeouts surfaced as ActionStatus.TIMEOUT.
- Automatic JSON Schema derivation for return types (pydantic models) and validation of tool outputs.
- Retry strategy for transient tool errors (ACTION_RETRYABLE).
- Health monitoring: periodic heartbeat loop + latency metrics.
- CLI integration (loom new/dev) for Python template scaffolding.

Long-term:

- Streaming token events for LLM providers (partial outputs -> writer agent).
- Multi-agent coordination primitives (contract-net, fanout/fanin majority, first-k).
- WASI sandbox for external tool execution.
- Advanced routing hints (cost/latency budgets) integrated with Router policies.
- Type-safe codegen for proto updates (managed inside build pipeline).
