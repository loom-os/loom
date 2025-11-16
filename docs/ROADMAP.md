# Roadmap (Loom OS 1.0)

**Goal**: Enable developers to build a long-running, observable, and extensible event‑driven Multi‑Agent System in Python/JS within 10 minutes.

**Current Status** (as of 2025-11): Core runtime, Bridge, Python SDK, MCP client, Dashboard MVP, OpenTelemetry metrics **and initial Trace Timeline UI** are **production-ready**. Focus shifting to **trace UX refinement** (flamegraph/search/heatmaps) and **testing hardening**.

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
- Dashboard MVP (React SSE): event stream, agent topology, flow graph, trace timeline (swimlanes v1)

**Developer Experience**:

- `loom run` orchestration (binary management, process lifecycle, config propagation)
- `loom.toml` configuration with env var substitution
- Market Analyst demo (5-agent async fan-out/fan-in with DeepSeek LLM)

### 🔥 Critical Gaps (Blocking Production)

#### ✅ **Distributed Tracing & Timeline** — COMPLETE (Phase 1)

**Problem**: Trace context was lost at process boundaries (Bridge ↔ Python agents), making cross-process debugging impossible.

**Solution**: Implemented end-to-end distributed tracing using the W3C Trace Context standard.

- **Envelope & Proto**: The `Envelope` and Protobuf `Event` definitions were updated to carry trace context (`traceparent` header). Helper methods `inject_trace_context` and `extract_trace_context` were added for propagation.
- **Rust Bridge**: The gRPC bridge was instrumented to extract trace context from incoming Python events and inject it into events delivered to Python agents. It now creates its own spans (`bridge.publish`, `bridge.forward`) that are correctly parented to the remote Python spans, closing the gap in the trace.
- **Python SDK**: The SDK now has a full OpenTelemetry integration. The `Agent` class automatically initializes tracing, and the `Context` object automatically handles trace context injection and extraction for all event operations (`emit`, `request`, `reply`).
- **Developer Experience**: Telemetry setup is now automated. The `loom run` command injects default OpenTelemetry environment variables, and the `Agent` class handles initialization, removing the need for boilerplate code in agent logic.

**Result**: End-to-end traces are visible in Jaeger and a condensed swimlane rendering appears in the Dashboard Timeline. Phase 2 will add flamegraph, search, deep-linking, and latency overlays.

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

**Acceptance Criteria (updated)**:

- ✅ CI runs Market Analyst E2E tests on every commit
- ✅ Code coverage > 70% for core, bridge, loom-py
- ✅ Stress tests pass: 20 agents, 10k events/min, 1 hour runtime
- ✅ Chaos tests demonstrate graceful degradation
- ✅ Timeline v1 populated during demo runs (trace-test, market-analyst)

**Estimated Effort**: 7-10 days

---

#### 3. **Binary Selection & Caching** — ✅ FIXED (2025-11-15)

**Problem**: `loom run` used stale cached binaries after local rebuild, causing confusion during development.

**Solution** (`loom-py/src/loom/embedded.py`):

- ✅ **Version validation**: Calls `binary --version` to validate cached binaries before use
- ✅ **Cache invalidation**: Automatically invalidates cache on version mismatch
- ✅ **Priority order**: local builds → cached → download (developer-friendly)
- ✅ **Release preference**: Prefers release over debug by default (`prefer_release=True`)
- ✅ **CLI flags**: Added `--use-debug` and `--force-download` to `loom run` and `loom up`
- ✅ **Visibility**: Logs which binary is being used (path + version)

**Remaining Work** (moved to P1):

- 🚧 Add `--clear-cache` command to CLI
- 🚧 Search from current directory upwards (not just repo root)

---

## P1 — Developer Experience & Ecosystem

**Focus**: Polish DX, complete Dashboard features, expand SDK language support.

### 1. Dashboard Enhancements (Phase 2)

**Current State**: Event stream + topology graph + flow graph + **Trace Timeline v1** (flat swimlanes, basic span attributes).

Required (Phase 2):

- 🚧 **Flamegraph View**: Hierarchical span stacking (parent/child collapse)
- 🚧 **Span Search & Filter**: By name, agent_id, topic, trace_id
- 🚧 **Latency Heatmaps**: Per agent/component
- 🚧 **Tool Call Overlay**: Annotate spans with capability invocation results
- 🚧 **Prometheus Metrics Integration**: Replace placeholder `/api/metrics`
- 🚧 **Thread Inspector**: Correlate collaboration patterns (fanout/fanin, barrier)
- 🚧 **Event Playback**: Replay sequence from stored spans + event snapshots

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

**Before declaring Phase 1 complete**:

- ✅ All Phase 1 gaps resolved (tracing propagation, timeline v1, binary selection)
- ✅ Market Analyst demo runs reliably with full observability + timeline spans
- ✅ CI/CD pipeline includes E2E tests and chaos engineering
- ✅ Documentation updated (quickstart, architecture, timeline, testing guide)
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
