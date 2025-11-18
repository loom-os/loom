# Roadmap: Loom 1.0 Launch and Beyond# Roadmap (Loom OS 1.0)

**Vision**: Evolve Loom from a powerful core into a comprehensive, accessible AI agent operating system, culminating in the launch of Loom and its Python SDK v1.0. Our goal is to empower developers to build, deploy, and manage sophisticated multi-agent systems with ease and confidence.**Current Focus**: Fix Market Analyst demo and complete distributed tracing instrumentation.

---**Status** (2025-11): Core runtime complete. Tracing infrastructure complete. **Now fixing Market Analyst demo blockers**.

## Phase 1: Launch Readiness (Immediate Focus: Next 4-6 Weeks)> See `docs/ARCHITECTURE.md` for system design. See `docs/observability/QUICKSTART.md` for tracing setup.

**Objective**: Solidify the core platform, polish the developer experience, and prepare for a successful public launch.---

### 1. **Agent Memory & Context System (🔥 Top Priority)**## 🎯 Market Analyst Demo Issues (Priority Order)

- **Problem**: Agents are stateless between invocations, leading to inconsistent decisions (e.g., Planner oscillating between BUY/SELL).

- **Solution**:### **Issue 1: Topic Wildcard Subscription** — 🔥 CRITICAL

  - **Short-Term Memory**: Implement a `deque`-based in-memory store for recent events and decisions within the agent's context.

  - **Long-Term Memory**: Design and integrate a persistent memory backend (e.g., SQLite or a simple file-based log) to store key decisions and world-state summaries.**Problem**: Agents use `topics=["market.price.*"]` but only exact matching works → no events delivered.

  - **Memory API**: Expose `ctx.memory.add()` and `ctx.memory.query()` in the Python SDK.

- **Acceptance**: The `planner` agent can access its previous decisions and avoid issuing conflicting plans.**Files to modify**:

### 2. **Dashboard & Observability Polish**- `core/src/event.rs` (line 295): Add pattern matching in `publish()`

- **Problem**: The dashboard has minor data fidelity issues that hinder debugging.- `core/src/agent/runtime.rs` (line 166): Handle wildcard in `subscribe_agent()`

- **Tasks**:

  - **Fix Data Agent Status**: Implement a heartbeat mechanism in the `data-agent` so it correctly reports its status as "running" instead of "idle".**Quick Fix** (1-2 days):

  - **Real QoS Metrics**: Connect the QoS Distribution panel to actual event bus metrics for latency, throughput, and delivery.

  - **MCP Integration**: Ensure MCP tool calls are clearly visualized in the timeline and flow graphs.```rust

// In EventBus::publish, match wildcards:

### 3. **Python SDK v1.0 Finalization**let matching_topics: Vec<String> = self.subscriptions

- **Problem**: The SDK is functional but needs refinement for a public release. .iter()

- **Tasks**: .filter(|entry| {

  - **API Review**: Finalize the `Agent` and `Context` APIs. Ensure method names are intuitive and consistent. let pattern = entry.key().as_str();

  - **Documentation**: Write comprehensive tutorials, API references, and "Getting Started" guides. pattern.ends_with(".\*") && topic.starts_with(&pattern[..pattern.len()-2])

  - **Packaging**: Ensure `pyproject.toml` is robust and the package can be easily installed via `pip`. || pattern == topic

  })

### 4. **Marketing & Launch Preparation** .map(|e| e.key().clone())

- **Objective**: Prepare all materials for a coordinated launch announcement. .collect();

- **Tasks**:```

  - **Blog Post**: Draft a launch announcement blog post detailing Loom's vision, features, and the `market-analyst` demo.

  - **Demo Video**: Record a high-quality video of the `market-analyst` demo, showcasing the dashboard, tracing, and real-time collaboration.**Acceptance**:

  - **Website Update**: Prepare the official website with links to the blog, video, and documentation.

- ✅ `market.price.*` receives `market.price.BTC`

---- ✅ All 3 analysis agents receive price events

## Phase 2: Ecosystem & Developer Experience (Mid-Term)---

**Objective**: Lower the barrier to entry, expand capabilities, and grow the developer community.### **Issue 2: Python SDK Missing Agent Handler Spans** — 🔥 CRITICAL

### 1. **"Loom Studio": The Dashboard Evolves\*\***Problem\*\*: Jaeger only shows 2 spans (`bridge.publish` + `publish`). No agent handler, LLM, or MCP tool spans.

- **Vision**: Transform the dashboard from a passive observability tool into an interactive, low-code IDE for agent development.

- **Features**:**Files to modify**:

  - **Visual Agent Builder**: Drag-and-drop interface for creating new agents and wiring them together.

  - **Event & Topic Explorer**: A UI to browse available topics, inspect event schemas, and publish test events.- `loom-py/src/loom/context.py` (line 48): Wrap `tool()` with span

  - **Live Agent Editor**: Edit agent code (Python) directly in the browser and hot-reload it into the running system.- `loom-py/src/loom/llm.py` (line 67): Wrap `generate()` with span

- `demo/market-analyst/agents/planner.py`: Add explicit spans for aggregation logic

### 2. **Simplified Python SDK & CLI**

- **Problem**: Writing agents still involves some boilerplate.**Quick Fix** (2-3 days):

- **Solution**:

  - **Decorator-based API**: Introduce `@on_event("topic.*")` and `@tool("name")` decorators to simplify agent creation.```python

  - **`loom` CLI**: Create a powerful CLI tool (`loom init`, `loom create agent`, `loom deploy`) to scaffold projects and manage the agent lifecycle.# In context.py

async def tool(self, name: str, ...) -> bytes:

### 3. **Contract (SWAP) Trading & Advanced Execution** with tracer.start_as_current_span(

- **Problem**: The `executor` currently only supports simple SPOT market orders. "capability.invoke",

- **Tasks**: attributes={"capability.name": name, "agent.id": self.agent_id}

  - **Leverage & Margin**: Add support for leveraged trading in the OKX capability. ):

  - **Advanced Order Types**: Implement limit orders, stop-loss, and take-profit. result = await self.client.forward_action(call)

  - **Position Management**: The `executor` needs to track open positions and manage them according to the plan. return bytes(result.output)

````

### 4. **JavaScript/TypeScript SDK**

- **Objective**: Expand Loom's reach to the massive community of web and Node.js developers.**Acceptance**:

- **Tasks**:

    - Create `loom-js` with parity to the Python SDK.- ✅ 15+ spans per Market Analyst trace

    - Enable building browser-based agents and Node.js backend agents.- ✅ LLM spans show provider/model/latency

- ✅ Timeline shows per-agent lanes

---

---

## Phase 3: Platform & Ubiquity (Long-Term)

### **Issue 3: LLM Config Not Loaded from loom.toml** — 🔥 CRITICAL

**Objective**: Make Loom the universal standard for building and running AI agents, from the cloud to the edge.

**Problem**: `LLMProvider.from_name("deepseek")` uses hardcoded config, ignores `[llm.deepseek]` in `loom.toml`.

### 1. **Cross-Platform SDKs (Mobile & Embedded)**

- **Vision**: Run Loom agents anywhere.**Files to modify**:

- **Tasks**:

    - **Lightweight Rust Core**: Optimize the core runtime for minimal resource footprint.- `loom-py/src/loom/llm.py` (line 51): Load from `load_project_config()`

    - **Mobile SDKs**: Provide Swift (iOS) and Kotlin (Android) wrappers around a C-ABI core.

    - **Embedded SDK**: Target devices like the ESP32 with a MicroPython or C SDK for IoT and robotics use cases.**Quick Fix** (0.5-1 day):



### 2. **Advanced Security & Routing**```python

- **Features**:@classmethod

    - **Multi-Tenancy & ACLs**: Securely run agents from different users on shared infrastructure.def from_name(cls, ctx, provider_name):

    - **Intelligent Router**: Evolve the router to make cost/latency-based decisions for LLM and tool routing.    from .config import load_project_config

    config = load_project_config()

### 3. **WASI Plugin Sandboxing**    if provider_name in config.llm:

- **Vision**: Safely run untrusted, third-party tools and agents.        llm_cfg = config.llm[provider_name]

- **Implementation**: Use WebAssembly (WASI) to provide a secure, sandboxed environment for plugin execution.        return cls(ctx, LLMConfig(

            base_url=llm_cfg["api_base"],

---            model=llm_cfg["model"],

            api_key=llm_cfg.get("api_key"),

## Launch Plan            temperature=llm_cfg.get("temperature", 0.7),

            max_tokens=llm_cfg.get("max_tokens", 4096),

1.  **Complete Phase 1**: Execute all tasks in the "Launch Readiness" phase.            timeout_ms=llm_cfg.get("timeout_sec", 30) * 1000,

2.  **Internal Review**: Conduct a final review of the code, documentation, and marketing materials.        ))

3.  **Public Launch**:    # Fallback to hardcoded defaults

    - Publish the Python SDK to PyPI.    ...

    - Publish the blog post.```

    - Share the demo video on social media (Twitter/X, LinkedIn).

4.  **Community Engagement**: Actively engage with the community on GitHub, Discord, and other channels to gather feedback and support new users.**Acceptance**:


- ✅ DeepSeek API called with configured key
- ✅ LLM spans in Jaeger

---

### **Issue 4: MCP Tool Never Called** — 🔥 CRITICAL

**Problem**: `sentiment.py` simulates sentiment, never calls `web-search` MCP tool.

**Files to modify**:

- `demo/market-analyst/agents/sentiment.py` (line 15-36)

**Quick Fix** (0.5-1 day):

```python
async def sentiment_handler(ctx, topic, event):
    data = json.loads(event.payload.decode("utf-8"))

    # Call MCP web-search
    try:
        result = await ctx.tool("web-search",
            payload={"query": f"{data['symbol']} crypto news", "count": 5})
        results = json.loads(result)

        # Analyze keywords for sentiment
        text = " ".join([r.get("title", "") + " " + r.get("snippet", "")
                        for r in results.get("results", [])])

        bullish = sum(text.lower().count(k) for k in ["surge", "bull", "rally"])
        bearish = sum(text.lower().count(k) for k in ["crash", "bear", "drop"])

        if bullish > bearish:
            sentiment = "bullish"
        elif bearish > bullish:
            sentiment = "bearish"
        else:
            sentiment = "neutral"

    except Exception as e:
        # Fallback to simulation
        sentiment = random.choice(["bullish", "neutral", "bearish"])
````

**Acceptance**:

- ✅ MCP tool spans in Jaeger
- ✅ Real news URLs in sentiment results

---

### **Issue 5: No E2E Tests for Market Analyst** — HIGH

**Problem**: No automated tests for 5-agent collaboration workflow.

**Files to create**:

- `demo/market-analyst/tests/test_simple_flow.py`
- `demo/market-analyst/tests/test_fanout_aggregation.py`
- `demo/market-analyst/tests/test_llm_integration.py`

**Quick Fix** (3-4 days):

```python
# test_simple_flow.py
async def test_price_to_analysis():
    # Start bridge + agents
    async with BridgeContext() as bridge:
        data_agent = MockDataAgent()
        trend_agent = MockTrendAgent()

        # Emit market.price.BTC
        await data_agent.emit_price("BTC", 91500.0)

        # Assert: analysis.trend emitted within 1s
        trend_event = await wait_for_event("analysis.trend", timeout=1.0)
        assert trend_event["symbol"] == "BTC"
        assert trend_event["trend"] in ["up", "down", "sideways"]
```

**Acceptance**:

- ✅ CI runs tests on every commit
- ✅ Tests validate full event flow
- ✅ Code coverage > 70%

---

## 🎯 Execution Sprint (Week 1)

| Day         | Task                              | Deliverable                         |
| ----------- | --------------------------------- | ----------------------------------- |
| **Mon**     | Issue 1: Wildcard subscription    | Risk/Trend/Sentiment receive events |
| **Tue**     | Issue 3: LLM config loading       | DeepSeek API working                |
| **Wed**     | Issue 4: MCP tool integration     | Sentiment uses real web search      |
| **Thu-Fri** | Issue 2: SDK span instrumentation | Rich traces in Jaeger               |

**Week 2**: E2E tests (Issue 5) + Dashboard polish

---

## ✅ Already Complete (See Docs)

**Infrastructure** (100% done):

- Distributed tracing: Envelope, Bridge spans, SDK integration
- Core runtime: EventBus, AgentRuntime, ActionBroker
- Python SDK: Agent/Context API, @capability decorator
- Dashboard: Timeline v1, topology, event stream
- CLI: `loom run` orchestration, binary management

**Documentation**:

- `docs/ARCHITECTURE.md` - System design
- `docs/observability/QUICKSTART.md` - Tracing setup
- `loom-py/docs/SDK_GUIDE.md` - Python SDK guide
- `docs/BRIDGE.md` - Cross-process communication

---

## P1 — Dashboard & Developer Tools (After Market Analyst Works)

### Dashboard Phase 2 (10-15 days)

- Agent-level lanes with collapse
- Flamegraph hierarchical view
- Span search & filtering
- Latency heatmaps
- Event persistence & replay

### CLI Tools (3-5 days)

- `loom logs` - Structured log aggregation
- `loom start/stop/restart <agent>` - Lifecycle management
- `loom trace <trace_id>` - Jaeger CLI integration

### SDK Enhancements

- JavaScript SDK (loom-js)
- Streaming API for long-running tasks
- Memory plugin interface
- MCP protocol extensions (SSE, Resources, Prompts)

---

## P2 — Enterprise & Scale (Future)

- **Security**: Multi-tenancy, ACLs, authentication
- **Router Evolution**: Learning-based routing, cost optimization
- **Event Persistence**: WAL, time-travel debugging
- **WASI Plugins**: Sandboxed tool execution

---

## Quality Gates

**Market Analyst Demo Must**:

- ✅ All 5 agents receive events and emit outputs
- ✅ DeepSeek LLM generates real plans
- ✅ Web search provides sentiment sources
- ✅ 15+ spans visible in Jaeger per workflow
- ✅ Timeline shows clear agent lanes
- ✅ E2E tests pass in CI

**Production Ready Means**:

- 24-hour continuous run (no crashes/leaks)
- P99 latency < 200ms (realtime events)
- 1000+ events/sec throughput
- Code coverage > 70%
- Full documentation updated
