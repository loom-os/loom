# Roadmap (Loom OS 1.0)

**Goal**: Enable developers to build a long-running, observable, and extensible event‑driven Multi‑Agent System in Python/JS within 10 minutes.

**Current Status** (as of 2025-11): Core runtime, Bridge, Python SDK, MCP client, Dashboard MVP, and OpenTelemetry metrics are **production-ready**. Focus shifting to **observability completion** (distributed tracing) and **testing hardening**.

## Architecture Overview

See `docs/ARCHITECTURE.md` for detailed component documentation.

- **Core** (Rust): EventBus, ActionBroker, ToolOrchestrator, Router, Agent Runtime, Directories
- **Bridge** (gRPC): Cross-process event/action forwarding with reconnection support
- **SDK** (Python): Agent/Context API with @capability decorator and collaboration primitives
- **Ecosystem**: MCP client (✅ stdio), Dashboard (✅ React SSE), OpenTelemetry (✅ metrics, 🚧 tracing)
- **CLI**: `loom run` orchestration, binary management, config system

---

## P0 — Production Readiness ✅ MOSTLY COMPLETE

**Delivery target**: `pip install loom && loom run` works reliably with observable multi-agent systems.

### ✅ Completed

**Core Components** (see `docs/core/overview.md` for details):

- EventBus with QoS/backpressure, AgentRuntime, Router, ActionBroker, ToolOrchestrator
- Envelope (thread/correlation/TTL), Collaboration primitives, Directories
- MCP Client (JSON-RPC over stdio, qualified naming, error handling)

**Bridge & SDK** (see `docs/BRIDGE.md`, `loom-py/docs/SDK_GUIDE.md`):

- gRPC Bridge with RegisterAgent/EventStream/ForwardAction/Heartbeat
- Python SDK with Agent/Context API, @capability decorator, reconnection logic
- Example: `trio.py` (Planner/Researcher/Writer collaboration)

**Observability** (see `docs/observability/QUICKSTART.md`):

- OpenTelemetry metrics (60+ metrics → Prometheus)
- Grafana dashboards (throughput, latency, routing, tool invocations)
- Dashboard MVP (React SSE): event stream, agent topology, flow graph

**Developer Experience**:

- `loom run` orchestration (binary management, process lifecycle, config propagation)
- `loom.toml` configuration with env var substitution
- Market Analyst demo (5-agent async fan-out/fan-in with DeepSeek LLM)

### 🔥 Critical Gaps (Blocking Production)

#### 1. **Distributed Tracing** — 🎯 HIGHEST PRIORITY

**Problem**: Trace context **lost at process boundaries** (Bridge ↔ Python agents), making cross-process debugging impossible.

**Current State**:

- ✅ Rust components instrumented (`#[tracing::instrument]` on EventBus, AgentRuntime, ActionBroker, Router)
- ✅ OTLP exporter configured (traces → Jaeger)
- ❌ **Trace context NOT propagated** across gRPC Bridge
- ❌ **Envelope does NOT carry** `trace_id`/`span_id`/`trace_flags`
- ❌ **Python SDK has NO OpenTelemetry integration**
- ❌ **Dashboard FlowTracker** cannot correlate flows to traces

**Required Work**:

```
Priority 1: Envelope trace context
  - Add trace_id/span_id/trace_flags to envelope.rs metadata keys
  - Implement attach_trace_context() / extract_trace_context()
  - Update Event.metadata and ActionCall.headers propagation

Priority 2: Bridge trace propagation
  - Extract trace context from ClientEvent in event_stream()
  - Inject trace context into ServerEvent deliveries
  - Create child spans for event forwarding loops
  - Test: Event from Python agent A → Rust EventBus → Python agent B preserves trace

Priority 3: Python SDK tracing
  - Add opentelemetry-api + opentelemetry-exporter-otlp-proto-grpc to pyproject.toml
  - Instrument Agent._run_stream() to extract trace from Envelope
  - Instrument Context.emit/request/reply to inject trace
  - Test: Full trace from agent.start() → on_event() → ctx.emit() → remote agent

Priority 4: Dashboard trace timeline
  - Add trace_id to FlowTracker.EventFlow
  - New API: GET /api/trace/:trace_id → full event timeline
  - UI: Click event → show full trace with spans and latencies
```

**Acceptance Criteria**:

- ✅ Trace spans visible in Jaeger from Rust → Bridge → Python → Bridge → Rust
- ✅ Market Analyst demo shows complete trace: data agent → trend/risk/sentiment → planner
- ✅ Dashboard can display trace timeline for any event_id
- ✅ Trace context survives agent reconnection

**Estimated Effort**: 5-7 days

---

#### 2. **Testing & Validation** — 🎯 HIGH PRIORITY

**Problem**: Complex multi-process async system lacks systematic testing, leading to regression risks and difficult debugging.

**Current State**:

- ✅ Core unit tests: event_bus, agent_runtime, router, action_broker
- ✅ Integration tests: e2e_basic, e2e_collab, e2e_tool_use
- ✅ Bridge tests: registration, heartbeat, forward_action
- ❌ **NO end-to-end tests** for full Rust + Bridge + Python stack
- ❌ **NO tests** for Market Analyst demo workflow
- ❌ **NO stress tests** for concurrent agents or high-frequency events
- ❌ **NO tests** for error scenarios (timeout, reconnection, partial failures)

**Required Work**:

```
Priority 1: Market Analyst E2E tests
  Location: demo/market-analyst/tests/

  - test_simple_flow.py
    • Start loom-bridge-server
    • Mock data agent: emit BTC price event
    • Mock trend agent: receive, analyze, reply
    • Mock planner agent: receive analysis, emit plan
    • Verify: event ordering, payload correctness, timing

  - test_fanout_aggregation.py
    • Start all 5 agents
    • Emit market.price.BTC
    • Verify: planner receives 3 analyses within timeout
    • Verify: planner handles partial data (2/3 analyses)
    • Verify: planner timeout logic (no responses)

  - test_llm_integration.py
    • Mock DeepSeek API responses
    • Verify: planner uses LLM reasoning
    • Verify: fallback to rule-based on LLM error

Priority 2: Bridge stress tests
  Location: bridge/tests/stress/

  - test_concurrent_agents.rs
    • 20 agents register simultaneously
    • Each subscribes to 10 topics
    • High-frequency event publishing (1000 events/sec)
    • Verify: no dropped events, no deadlocks

  - test_reconnection.rs
    • Agent disconnects mid-stream
    • Verify: graceful cleanup (subscriptions, flows)
    • Agent reconnects with same agent_id
    • Verify: re-registration, topic re-subscription

Priority 3: Python SDK integration tests
  Location: loom-py/tests/integration/

  - test_bridge_roundtrip.py
    • Python agent A emits event
    • Python agent B receives via Bridge
    • Verify: Envelope metadata preserved

  - test_capability_invocation.py
    • Python agent registers capability
    • Rust ActionBroker invokes via Bridge
    • Verify: timeout handling, error propagation

Priority 4: Chaos engineering tests
  Location: tests/chaos/

  - Simulate: Bridge crash, EventBus backpressure, slow agents
  - Verify: system recovers, no data loss, metrics accurate
```

**Acceptance Criteria**:

- ✅ CI runs Market Analyst E2E tests on every commit
- ✅ Code coverage > 70% for core, bridge, loom-py
- ✅ Stress tests pass: 20 agents, 10k events/min, 1 hour runtime
- ✅ Chaos tests demonstrate graceful degradation

**Estimated Effort**: 7-10 days

---

#### 3. **Binary Selection & Caching** — 🐛 KNOWN BUG

**Problem**: `loom run` may use stale cached binaries after local rebuild, causing confusion during development.

**Current State** (`loom-py/src/loom/embedded.py`):

- ✅ Finds local builds in `target/{debug,release}/`
- ✅ Downloads from GitHub Releases with SHA256 verification
- ✅ Caches binaries in `~/.cache/loom/bin/{version}/`
- ❌ **No version validation** for cached binaries
- ❌ **Cache invalidation** requires manual deletion
- ❌ **Priority order** unclear (cached vs local build)

**Required Work**:

```
Priority 1: Add version validation
  - get_binary() calls binary --version
  - Compare with expected version
  - Invalidate cache on mismatch

Priority 2: Improve local build detection
  - Prefer release over debug by default
  - Add --use-debug flag to loom run
  - Search from current directory upwards (not just repo root)

Priority 3: Developer ergonomics
  - Add --force-rebuild flag to loom run
  - Add --clear-cache command to CLI
  - Log which binary is being used (path + version)
```

**Acceptance Criteria**:

- ✅ After `cargo build --release`, next `loom run` uses new binary
- ✅ Cache invalidated automatically when version changes
- ✅ Clear error message if binary version mismatch

**Estimated Effort**: 1-2 days

---

## P1 — Developer Experience & Ecosystem

**Focus**: Polish DX, complete Dashboard features, expand SDK language support.

### 1. Dashboard Enhancements

**Current State**: Basic event stream + topology graph (see `docs/dashboard/FLOW_VISUALIZATION_GUIDE.md`)

**Required**:

- 🚧 **Trace Timeline View**: Swimlane visualization with span hierarchy (requires P0 tracing)
- 🚧 **Prometheus Integration**: Replace placeholder `/api/metrics` with real Prometheus queries
- 🚧 **Thread Inspector**: Filter/group events by thread_id, show collaboration patterns
- 🚧 **Tool Call Timeline**: Success/failure breakdown, latency distribution per capability
- 🚧 **Event Playback**: Time-travel debugging with event replay from Jaeger traces

### 2. CLI Tooling

**Required**:

- 🚧 `loom dev`: Watch Python/JS files, hot-reload agents on change
- 🚧 `loom list`: Show registered agents/capabilities with filtering
- 🚧 `loom bench`: Run predefined performance tests (latency, throughput, concurrency)
- 🚧 `loom logs`: Structured log viewer with grep-like filtering by agent/thread/correlation
- 🚧 `loom trace <trace_id>`: Fetch and display full trace from Jaeger CLI

### 3. JavaScript SDK (loom-js)

**Goal**: Feature parity with loom-py

**Required**:

- `defineAgent()` with TypeScript types
- `ctx.emit/request/reply/tool()` API
- gRPC Bridge client with reconnection
- Envelope extraction and metadata helpers
- OpenTelemetry trace propagation (from day 1)

### 4. SDK Improvements

**Python**:

- Streaming API for long-running tasks (async generators)
- Memory plugin interface (Redis, PostgreSQL, in-memory backends)
- Better type hints and Pydantic v2 validation
- Middleware hooks (logging, tracing, auth)

### 5. MCP Protocol Extensions

**Current**: stdio transport only (see `docs/MCP.md`)

**Required**:

- SSE transport (HTTP-based for web integration)
- Resources API (read/write/list)
- Prompts API (list/get with arguments)
- Sampling support (multi-turn tool use)
- Notifications (server-initiated events)

---

## P2 — Enterprise & Production Hardening

**Focus**: Security, scalability, advanced routing, persistence.

### 1. MCP Server Mode

**Goal**: Expose Loom capabilities to external systems (n8n, Make, Zapier, custom MCP clients)

**Required**:

- Implement MCP server protocol (bidirectional: client ✅ + server)
- Register Loom ActionBroker capabilities as MCP tools
- Support SSE transport for web integration
- Authentication and rate limiting

### 2. Router Evolution

**Current**: Rule-based routing (privacy/latency/cost/quality policies)

**Required**:

- Historical metrics collection (success rate, latency, cost per route)
- Learning-based routing (bandit/RL algorithms)
- A/B testing framework for routing strategies
- Cost optimization with provider pricing models
- Dashboard UI for tuning routing policies

### 3. Security & Multi-tenancy

**Required**:

- Namespaces/ACLs for agent isolation
- Token-based authentication for Bridge connections
- MCP endpoint allowlist (security policies)
- Audit logs for all actions and capability invocations
- Rate limiting per agent/namespace

### 4. Event Persistence & Replay

**Goal**: Durability, time-travel debugging, disaster recovery

**Required**:

- Write-Ahead Log (WAL) for EventBus
- Event snapshots for recovery
- Time-travel debugging (replay from timestamp)
- Backup/restore tools
- Standardized memory topics: `memory.update`, `memory.retrieve`

### 5. WASI Plugin Isolation

**Goal**: Sandboxed execution for untrusted tools/plugins

**Required**:

- WASM runtime integration (wasmtime/wasmer)
- Resource limits (CPU/memory/network) per plugin
- AOT compilation for edge/mobile
- Plugin security policies and capability allowlists

---

## P3 — Edge & Mobile

**Focus**: On-device deployment, deep optimization.

### 1. Mobile Packaging

**Required**:

- iOS/Android POC (xcframework/AAR for Rust core)
- Lightweight wrappers with minimal dependencies
- On-device model inference (CoreML, TensorFlow Lite)
- Background task management and power optimization
- Push notification integration for event delivery

### 2. Performance Optimization

**Required**:

- EventBus lock-free data structures
- Tool execution parallelism and scheduling
- Memory footprint reduction (arena allocators, zero-copy)
- Power consumption profiling
- GPU/NPU acceleration for local models

### 3. Production SRE

**Required**:

- Graceful degradation under load (adaptive QoS)
- Circuit breakers for external dependencies
- Health checks and readiness probes
- Blue-green deployment support
- Canary releases for agents and capabilities

---

## Quality Gates

**Metrics** (baseline targets for P0 completion):

- **Time to First Message (TTFM)**: ≤ 10 minutes (Python SDK, fresh environment)
- **Stability**: 24-hour continuous run without memory leaks or crashes
- **Latency**: P99 event latency < 200ms under mixed QoS (realtime events)
- **Throughput**: 1000+ events/sec per EventBus instance
- **Tool Invocation**: Error rate < 2% (excludes external API failures)
- **Observability**: Dashboard first paint < 2s, all traces visible in Jaeger
- **Testing**: Code coverage > 70% (core + bridge + loom-py)

**Before declaring P0 complete**:

- ✅ All P0 critical gaps resolved (tracing, testing, binary selection)
- ✅ Market Analyst demo runs reliably with full observability
- ✅ CI/CD pipeline includes E2E tests and chaos engineering
- ✅ Documentation complete (quickstart, architecture, troubleshooting)
- ✅ PyPI package published (`loom` v0.1.0)

---

## Implementation Notes

**Tracing Strategy** (P0 Critical Gap #1):

1. Start with Envelope modifications (lowest risk, highest value)
2. Bridge trace propagation (critical path for cross-process debugging)
3. Python SDK integration (completes the loop)
4. Dashboard timeline (user-visible impact)

**Testing Strategy** (P0 Critical Gap #2):

1. Market Analyst E2E tests first (validates core workflows)
2. Bridge stress tests (validates scalability assumptions)
3. Python SDK integration tests (validates SDK robustness)
4. Chaos tests last (validates fault tolerance)

**Focus Principle**:

- Don't add new features until P0 critical gaps are closed
- Distributed tracing is blocking for production debugging
- Testing validates reliability claims
- Everything else is polish or nice-to-have
