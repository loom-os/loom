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

### ✅ Completed in P0

1. **Bridge (gRPC)** — ✅ COMPLETE
   - AgentRegister (topics, capabilities), bidirectional EventStream (publish/delivery)
   - Client-initiated ForwardAction, server-initiated ActionCall (internal push API + result correlation map)
   - Heartbeat, stateless reconnection
   - Integration tests: registration, event roundtrip, forward action, heartbeat
2. **Python SDK (loom‑py)** — ✅ COMPLETE

   - Core Agent/Context API: emit/reply/tool/request (with correlation_id)
   - @capability decorator with auto Pydantic input/output schema
   - Unified Envelope (thread_id/correlation_id/sender/reply_to/ttl/hop via metadata)
   - gRPC BridgeClient with RegisterAgent/EventStream/ForwardAction/Heartbeat
   - Agent orchestration: stream loop, capability invocation, action_result correlation
   - Packaging: `pyproject.toml` ready for PyPI (`0.1.0a1`)
   - Example: trio.py (Planner/Researcher/Writer collaboration)

3. **Collaboration primitives** — ✅ COMPLETE

   - request/reply with first_k/timeout strategies
   - fanout/fanin (any/first-k/majority)
   - barrier (wait for N replies or timeout)
   - contract-net (call for proposals/bids/award/execute)
   - Thread broadcast topic: `thread.{thread_id}.broadcast`
   - Reply topic: `thread.{thread_id}.reply`

4. **MCP Client** — ✅ COMPLETE

   - Connect to MCP servers via stdio → fetch tool JSON Schema → register as CapabilityDescriptor
   - McpClient (JSON-RPC 2.0 over stdio), McpToolAdapter (implements CapabilityProvider)
   - McpManager for multiple server lifecycle
   - Invoke MCP tools via ActionBroker with unified error codes (INVALID_PARAMS/TIMEOUT/TOOL_ERROR/...)
   - Configurable protocol version with validation
   - Auto-discovery and qualified tool naming (server:tool)
   - Comprehensive tests and documentation

5. **Directories** — ✅ COMPLETE
   - AgentDirectory: discover agents by id/topics/capabilities
   - CapabilityDirectory: snapshot providers from ActionBroker
   - Integration with Agent Runtime for auto-registration

### 🚧 In Progress / Pending in P0

6. **JS SDK MVP (loom‑js)** — 🚧 TODO

   - defineAgent(handler), ctx.emit/request/reply/tool
   - Similar API surface to loom-py for consistency

7. **Dashboard MVP** — 🚧 TODO

   - Nodes and edges (Agents, Topics, Tool invocations)
   - Swimlane of last N events (by thread_id)
   - Metric cards: published/delivered/dropped, tool_calls_total, latency stats
   - Technology choice: Web-based (React/Vue + WebSocket) or terminal UI (Ratatui)

8. **CLI basics** — 🚧 TODO
   - `loom new <template>` (multi-agent, voice-assistant, etc.)
   - `loom dev` (hot-boot external agents, watch for changes)
   - `loom list` (show registered agents/capabilities)
   - `loom bench` (performance profiling)

### Acceptance Criteria (P0 Complete)

- ✅ Core runtime stable: EventBus, Agent Runtime, Router, ActionBroker, ToolOrchestrator
- ✅ Python agents can register, emit/receive events, invoke capabilities
- ✅ Multi-agent collaboration works (trio example functional)
- ✅ MCP tools can be ingested and invoked via ActionBroker
- ✅ Bridge supports gRPC with full lifecycle management
- 🚧 Dashboard shows real-time topology and metrics (pending)
- 🚧 CLI provides quick-start templates (pending)
- 🚧 Auto-reconnect tested with network interruptions (needs formal test)
- 🚧 P50/P99 latency benchmarks published (needs benchmark suite)

---

## P1 — Observable, iterative collaboration system

### Focus: Enhanced observability, streaming, error handling, and developer ergonomics

1. **Dashboard enhancements** — 🎯 PRIORITY

   - Technology selection: Web (React/Vue + WebSocket) vs Terminal UI (Ratatui)
   - Real-time topology graph with auto-layout
   - Event swimlanes with thread_id grouping and filtering
   - Latency histograms (P50/P90/P99) per agent/capability
   - Backpressure gauges and QoS insights per topic
   - Error heatmaps and per-topic failure rates
   - Tool invocation timeline and success/failure breakdown

2. **CLI and templates** — 🎯 PRIORITY

   - `loom new <template>`: multi-agent, voice-assistant, home-automation, vision-camera, system-helper
   - `loom dev`: hot-reload for external agents (watch Python/JS files)
   - `loom list`: show registered agents, topics, capabilities with filtering
   - `loom bench`: built-in performance profiling and latency reports
   - `loom logs`: structured log viewer with filtering by agent/thread/correlation

3. **Streaming and parallelism**

   - SSE partial answers (LLM token streaming via ActionBroker)
   - Parallel tool invocation with semaphore/circuit breaker
   - Stream backpressure propagation to LLM providers
   - Chunked event payloads for large data (e.g., video frames)

4. **Error taxonomy and unified error_event**

   - Standardize error codes: MODEL_FALLBACK / TOOL_PARSE_ERROR / INVALID_PARAMS / CAPABILITY_ERROR / TIMEOUT / PROVIDER_UNAVAILABLE
   - Publish error_event on dedicated topic for monitoring
   - Prometheus labels for error classification
   - Error recovery strategies (retry with exponential backoff, fallback provider)

5. **SDK ergonomics**

   - Memory plugins (pluggable KV backends: Redis, PostgreSQL, in-memory)
   - Better type hints and runtime validation (Pydantic v2 for Python)
   - Streaming API for long-running tasks (async generators)
   - Middleware hooks for logging, tracing, auth

6. **MCP enhancements**
   - SSE transport (HTTP-based) in addition to stdio
   - Resources API support (read/write/list resources)
   - Prompts API support (list/get prompts with arguments)
   - Sampling support for multi-turn tool use
   - Notifications support (server-initiated events)

---

## P2 — Ecosystem and policy advancement

### Focus: MCP server mode, intelligent routing, security, and persistence

1. **MCP server mode**

   - Expose Loom's internal capabilities as MCP tools to external systems
   - Bidirectional MCP integration (client ✅ + server)
   - Cross-ecosystem interop (n8n, Make, Zapier, custom MCP clients)

2. **Router as a policy engine**

   - Learning-based routing with historical success/latency/cost metrics
   - Bandit/RL algorithms for adaptive model selection
   - Tunable routing policies via Dashboard UI
   - A/B testing support for routing strategies
   - Cost optimization with provider pricing models

3. **Security and multi-tenancy**

   - Namespaces/ACLs for agent isolation
   - Token-based authentication for Bridge connections
   - MCP endpoint allowlist (security policies for external tools)
   - Audit logs for all agent actions and capability invocations
   - Rate limiting per agent/namespace

4. **Event persistence and replay**

   - Write-Ahead Log (WAL) for event durability
   - Event snapshots for recovery and replay
   - Time-travel debugging (replay from specific timestamp)
   - Long-run stability metrics (24h+ uptime tests)
   - Backup/restore tools for production deployments

5. **WASI/external tool isolation**
   - Sandboxed tool execution (WASM runtime for untrusted tools)
   - Resource limits (CPU/memory/network) per tool
   - AOT compilation for edge/mobile deployment
   - Plugin security policies and capability allowlists

---

## P3 — Performance and mobile

### Focus: Edge deployment, deep optimization, and production hardening

1. **Mobile/edge packaging**

   - iOS/Android POC (xcframework/AAR for Rust core)
   - Lightweight wrappers with minimal dependencies
   - On-device model inference (CoreML, TensorFlow Lite)
   - Background task management and power optimization
   - Push notification integration for event delivery

2. **Deep performance work**

   - EventBus throughput/latency optimization (lock-free data structures)
   - Tool execution parallelism and smart scheduling
   - Memory footprint reduction (arena allocators, zero-copy)
   - Power consumption profiling and optimization
   - GPU/NPU acceleration for local models

3. **Production hardening**
   - Graceful degradation under load (adaptive QoS)
   - Circuit breakers for external dependencies
   - Health checks and readiness probes
   - Blue-green deployment support
   - Canary releases for agents and capabilities

---

## Current Status Summary (as of MCP completion)

### ✅ Fully Implemented

- **Core Runtime**: EventBus (QoS/backpressure), Agent Runtime (stateful actors), Router (policy-based), ActionBroker (capability registry), ToolOrchestrator
- **Envelope**: Thread/correlation metadata with TTL/hop, reply topics
- **Collaboration**: request/reply, fanout/fanin (any/first-k/majority/timeout), barrier, contract-net
- **Directories**: AgentDirectory (discover by id/topics/capabilities), CapabilityDirectory (provider snapshot)
- **MCP Client**: JSON-RPC 2.0 over stdio, auto-discovery, qualified naming (server:tool), configurable protocol version, comprehensive error handling
- **Bridge**: gRPC with RegisterAgent/EventStream/ForwardAction/Heartbeat, integration tests
- **Python SDK**: Agent/Context API, @capability decorator, Envelope support, trio example

### 🚧 In Progress

- **Dashboard**: Technology selection and initial implementation
- **CLI**: Template scaffolding and dev workflow tools
- **JS SDK**: API design and initial implementation

### 📋 Next Up (P1 Focus)

1. Dashboard MVP (web or terminal UI)
2. CLI basics (new/dev/list/bench)
3. JS SDK parity with Python
4. Streaming APIs and error taxonomy
5. MCP SSE transport and additional APIs

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
