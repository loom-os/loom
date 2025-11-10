# Loom Python SDK Design (MVP -> Roadmap)

## Goals

- Minimize mental overhead for Python developers: build multi-agent flows without knowing Rust core internals.
- Provide a single `loom` CLI that can scaffold projects, start a local core (bridge), and run agents.
- Offer progressive runtime modes: Local (dev), Managed (remote hosted core), Embedded (future packaged core).

## Layers

1. **BridgeClient (`client.py`)**: gRPC calls (RegisterAgent, EventStream, ForwardAction, Heartbeat). Handshake: first stream message is Ack(agent_id).
2. **Agent (`agent.py`)**: Lifecycle (connect, register, stream loop), capability dispatch, graceful stop.
3. **Context (`context.py`)**: Event primitives and tool invocation. Maintains correlation map for simple request/reply.
4. **Capability (`capability.py`)**: Decorator converts function signature to Pydantic model; metadata holds JSON Schema for input/output.
5. **Envelope (`envelope.py`)**: Unifies thread/correlation/reply semantics inside `Event.metadata` keys (`loom.thread_id`, etc.).
6. **Memory (`memory.py`)**: In-process, thread-scoped KV store (MVP). Future: plugin interface.
7. **CLI (`cli.py`)**: Commands: `loom proto`, `loom dev`, `loom new`, `loom run`.

## Runtime Modes (Planned)

| Mode       | Description                                                 | Setup                   | Target Use                          |
| ---------- | ----------------------------------------------------------- | ----------------------- | ----------------------------------- |
| Local      | Start bridge via `loom dev` (Cargo)                         | Requires Rust toolchain | Development and debugging           |
| Managed    | Point SDK at a remote LOOM_BRIDGE_ADDR (cloud/core service) | Env var or config       | Production, low-latency multi-agent |
| Embedded\* | Ship a slim pre-built bridge/core artifact (binary/ffi)     | Optional download       | Quick start, sandboxes, education   |

\*Embedded mode requires a reproducible static build of core; planned after performance stabilization.

## Event Semantics

- **emit(topic, type, payload)**: Fire-and-forget publish.
- **request(topic, type)**: Emit with `correlation_id = event.id`, `reply_to = agent.<id>.replies`; waits for matching delivery.
- **reply(original, type, payload)**: Emits to `original.reply_to` (or sender's default reply topic) preserving correlation.
- **tool(capability, payload)**: ForwardAction gRPC call; returns raw bytes (JSON recommended).

## Capability Dispatch

- Matching by `ActionCall.capability` name.
- Input payload JSON → validated via Pydantic model derived from function signature.
- Errors surfaced as `ActionStatus.ACTION_ERROR` with `CAPABILITY_ERROR` code.

## Envelope Fields

| Field          | Purpose                                       |
| -------------- | --------------------------------------------- |
| thread_id      | Conversation / workflow grouping              |
| correlation_id | Linking requests and replies                  |
| sender         | Origin agent id                               |
| reply_to       | Target reply topic                            |
| ttl_ms         | Future expiry semantics (drop after deadline) |

All encoded as metadata prefix `loom.<key>` for transport simplicity.

## Reducing User Mental Load

1. **Zero-config default**: If no `LOOM_BRIDGE_ADDR`, attempt to start local bridge (future auto-start in Agent.start with opt-out flag).
2. **CLI scaffolding**: `loom new` creates runnable agent with minimal handler + capability example.
3. **Managed endpoint**: Provide a known default (e.g., `bridge.loomcloud.dev:443`) with auth token; user sets `LOOM_TOKEN` and runs agents; no local core required.
4. **Unified error messages**: Translate gRPC errors into high-level Python exceptions (planned wrapper around AioRpcError).
5. **Observability-lite**: Auto print basic stats (latency, deliveries) in dev mode (future).

## Testing Strategy (Upcoming)

- Stub bridge server: spawn Rust process or implement minimal Python fake for unit tests of Agent/Context logic.
- Property tests for envelope metadata roundtrip.
- Capability schema validation tests.

## Repository Split Criteria

Extract `loom-py` to dedicated repo when:

- Test coverage > 70% lines on SDK core.
- Stable gRPC surface (no breaking changes for 2 minor versions).
- Published PyPI releases (>=0.2.0) with changelog automation.
- At least one external dependent project using the SDK.

Migration steps:

1. Create `loom-os/loom-py` repo.
2. Copy `loom-py/` contents; keep proto generation referencing git submodule or release artifact.
3. Set up CI (lint, tests, build, publish to PyPI on tag).
4. Deprecation notice in monorepo README linking to new repo.

## Future Enhancements

See `FUTURE.md` for detailed backlog: memory plugins, barrier primitives, streaming outputs, retry policies, embedded core.

## Open Questions

- Auth & namespace negotiation on RegisterAgent (token-based, multi-tenant RBAC).
- Backpressure signals surfacing in Python async API (channel saturation callbacks).
- Tool concurrency limits & circuit breaker strategy.

---

Feedback welcome; refine before 0.1.0 release.
