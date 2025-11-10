# Roadmap (Loom OS 1.0)

Goal: Enable developers to build a long-running, observable, and extensible event‑driven Multi‑Agent System in Python/JS within 10 minutes. The system can act on the outside world via MCP/function‑call tools, while the Rust Core provides performance and reliability.

Layered architecture (bottom‑up):

- Core (Rust): EventBus (QoS/backpressure/stats), ActionBroker (capability registry/invocation/timeouts), ToolOrchestrator (unified tool parsing/refine/stats), Router (policies for privacy/latency/cost/quality).
- Bridge (protocol): gRPC or WebSocket for Agent registration, event streaming, capability invocation, heartbeat and backpressure signals.
- SDK (Python/JS): Agent abstraction (on_event), Context (emit/request/reply/tool/memory/join_thread), @capability declaration, collaboration primitives (fanout/fanin/barrier/contract‑net).
- Ecosystem: MCP client (ingest tools → capability directory → invoke), optional MCP server (expose Loom capabilities externally).
- UX: CLI (loom new/dev/bench/list), Dashboard (topology, swimlanes, latency, backpressure, routing, tool calls).

---

## P0 — Minimal viable multi‑language multi‑agent (highest priority)

Delivery target: Minimal Vertical Slice (MVS). Spin up 3 agents (Planner/Researcher/Writer) in Python/JS, collaborate via events to perform search and summarization, invoke web.search/weather.get; Dashboard shows basic event flow; single‑command run via CLI.

Scope:

1. Bridge MVP (gRPC/WS, choose one first)
   - ✅ Implemented (gRPC): AgentRegister (topics, capabilities), bidirectional EventStream (publish/delivery), client-initiated ForwardAction, server-initiated ActionCall (internal push API + result correlation map), heartbeat, stateless reconnection.
   - Pending: external admin RPC for server push, metrics/backpressure export, auth/namespaces.
2. Python SDK MVP (loom‑py)
   - Core Agent/Context API: emit/reply/tool implemented; request (with correlation_id)/basic in‑process memory/join_thread topic convention scaffolded for MVP.
   - @capability decorator: implemented (auto Pydantic input schema + optional output schema) in `loom-py/src/loom/capability.py`.
   - Unified envelope: implemented (`Envelope` dataclass stores thread_id/correlation_id/sender/reply_to/ttl_ms via metadata prefix `loom.`).
   - gRPC bridge client: `BridgeClient` with RegisterAgent/EventStream/ForwardAction/Heartbeat handshake (Ack first) in `client.py`.
   - Agent orchestration: `Agent` class manages stream loop, capability invocation, action_result correlation.
   - Packaging: `loom-py/pyproject.toml` (name `loom`) ready for PyPI alpha publish (`0.1.0a1`). Generation script `python -m loom.proto.generate`.
   - Next: add example trio (Planner/Researcher/Writer) + tests (registration, emit roundtrip, capability invocation); expand request/barrier primitives.
3. JS SDK MVP (loom‑js)
   - defineAgent(handler), ctx.emit/request/reply/tool.
4. Collaboration primitives (first batch)
   - request/reply, barrier (wait for N replies or timeout), thread broadcast topic: `thread.{thread_id}.events`.
5. MCP client (basic tool ingest)
   - Connect to MCP servers on startup → fetch tool JSON Schema → register as CapabilityDescriptor.
   - Invoke MCP tools via ActionBroker; unify error codes (INVALID_PARAMS/TIMEOUT/...).
6. Dashboard MVP
   - Nodes and edges (Agents, Topics, Tool invocations); swimlane of last N events (by thread_id).
   - Metric cards: published/delivered/dropped, tool_calls_total, simple latency stats.
7. CLI basics
   - `loom new multi‑agent` (template with 3 agents), `loom dev` (hot‑boot external agents), `loom list`.

Acceptance:

- After `loom new multi-agent && loom dev`, opening the Dashboard shows the Planner→Researcher→Writer flow; running the sample question produces a summarized answer.
- Python/JS agents can call web.search/weather.get; at least one MCP tool is ingested and callable.
- Publish P50/P99 round‑trip event latency; auto‑reconnect works after network interruptions.

---

## P1 — Observable, iterative collaboration system

1. Collaboration primitives expansion
   - contract‑net (call for proposals/bids/award/execute), fanout/fanin strategies (any/first‑k/majority).
2. Streaming and parallelism
   - SSE partial answers (LLM token stream), parallel tool invocation with limits (semaphore/circuit breaker).
3. Dashboard enhancements
   - Latency histograms (P50/P90/P99), backpressure gauges, error heatmaps, per‑topic QoS insights.
4. Error taxonomy and unified error_event
   - MODEL_FALLBACK / TOOL_PARSE_ERROR / INVALID_PARAMS / CAPABILITY_ERROR / TIMEOUT / PROVIDER_UNAVAILABLE; Prometheus labels.
5. CLI and templates
   - New templates: voice assistant, home automation, vision camera agent, system helper.
6. SDK ergonomics
   - Memory plugins (pluggable KV backends), better type hints and pydantic validation.

---

## P2 — Ecosystem and policy advancement

1. MCP server mode
   - Expose Loom’s internal capabilities to external systems; cross‑ecosystem interop.
2. Router as a policy engine
   - Learning‑based routing with historical success/latency/cost (bandit/RL); tunable via Dashboard.
3. Security and multi‑tenancy
   - Namespaces/ACLs, token auth; MCP endpoint allowlist; audit logs.
4. Event persistence and replay
   - WAL/snapshots; long‑run stability metrics and tools.
5. WASI/external tool isolation
   - Sandboxed execution, resource limits, AOT readiness (mobile).

---

## P3 — Performance and mobile

1. Mobile/edge packaging
   - iOS/Android POC (xcframework/AAR), lightweight wrappers.
2. Deep performance work
   - EventBus throughput/latency optimization; tool execution parallelism and scheduling; footprint and power observations.

---

## Metrics and quality gates

- TTFM: ≤ 10 minutes (Python/JS).
- Stability: reconnect on drop; 24h longevity without memory blow‑ups.
- Performance: P99 event latency target < 200ms under mixed QoS (realtime path); tool invocation error rate < 2%.
- Observability: Prometheus/OTel metrics present; Dashboard first paint < 2s.

---

## Notes

- The Core already provides EventBus/ActionBroker/Router/ToolOrchestrator with pressure tests; on top of this, prioritize the minimal viable path for Bridge + SDK + Dashboard + MCP.
- The Voice Agent remains one of the showcase templates (available in the P1 template library).
