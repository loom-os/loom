# Dashboard MVP & OpenTelemetry - Issues with Priority

**Goal**: 实现 Dashboard MVP，展示 event 流动和多智能体系统的可观测性

## 代码分析总结

### 现有基础设施

✅ **已完成**:

- Core Runtime (EventBus, ActionBroker, Router, ToolOrchestrator)
- gRPC Bridge (Python/JS SDK 连接)
- MCP Client (工具集成)
- Envelope (thread_id/correlation_id 元数据传播)
- 基础 tracing (使用 `tracing` crate，但未连接 OTLP)

❌ **缺失**:

- OpenTelemetry 集成（无 OTLP exporter）
- 结构化 metrics (只有简单的 MetricsCollector)
- Trace context propagation (W3C TraceContext)
- Dashboard (前端和后端都不存在)
- 实时 event stream API

### 关键埋点位置

**EventBus** (`core/src/event.rs`):

- Line 157: `publish()` - 需要 span + metrics (published/delivered/dropped)
- Line 229: `subscribe()` - 需要 span
- Line 162-171: 统计更新 - 需要 Prometheus metrics

**ActionBroker** (`core/src/action_broker.rs`):

- Line 52: `invoke()` - 需要 span + latency histogram
- Line 62-65: Cache hit - 需要 metric
- Line 90-127: Timeout/error - 需要 error metrics

**Router** (`core/src/router.rs`):

- Line 160: `route()` - 需要 span + decision metrics
- Line 151-161 (agent/instance.rs): 已有日志，需增强为 span attributes

**ToolOrchestrator** (`core/src/llm/tool_orchestrator.rs`):

- Line 110: `run()` - 已有 `#[tracing::instrument]`，需增强
- Line 172-190: Tool invocation loop - 需要 child spans

**Agent Runtime** (`core/src/agent/runtime.rs`):

- Line 82: `create_agent()` - 需要 span
- Line 54-109 (instance.rs): Agent event loop - 需要 span

**MCP Manager** (`core/src/mcp/manager.rs`):

- Line 38: `add_server()` - 需要 span
- Line 137: `register_tools()` - 需要 span + metrics

**Bridge** (`bridge/src/lib.rs`):

- Line 99: `register_agent()` - 需要 span
- Line 127: `event_stream()` - 需要 span + metrics

---

## Issues with Priority

### 🔴 P0 - Critical (必须完成才能有基本可观测性)

#### #1: OpenTelemetry Core Integration

**Files**: `core/Cargo.toml`, `core/src/telemetry.rs`, `core/src/lib.rs`

**任务**:

1. 添加依赖: `opentelemetry`, `opentelemetry_sdk`, `opentelemetry-otlp`, `tracing-opentelemetry`
2. 扩展 `telemetry.rs`: 初始化 OTLP exporter (gRPC to port 4317)
3. 在 `Loom::new()` 调用 `init_telemetry()`
4. 在 `Loom::shutdown()` 调用 `shutdown_telemetry()`

**验收**: Traces 导出到 Jaeger, Metrics 导出到 Prometheus

---

#### #2: EventBus 完整埋点

**Files**: `core/src/event.rs`

**Spans**:

- `event_bus.publish` (line 157) - 属性: topic, event_id, qos_level
- `event_bus.subscribe` (line 229)
- `event_bus.unsubscribe` (line 269)

**Metrics**:

- `loom.event_bus.published_total{topic, event_type}`
- `loom.event_bus.delivered_total{topic, qos_level}`
- `loom.event_bus.dropped_total{topic, qos_level, reason}`
- `loom.event_bus.backlog_size{topic}` (gauge)
- `loom.event_bus.publish_latency_ms{topic}` (histogram)

**验收**: 在 Jaeger 中看到完整的 event 流动链路

---

#### #3: ActionBroker 埋点

**Files**: `core/src/action_broker.rs`

**Spans**:

- `action_broker.invoke` (line 52) - 属性: capability, version, timeout_ms
- `action_broker.register_provider` (line 36)

**Metrics**:

- `loom.action_broker.invocations_total{capability, status}`
- `loom.action_broker.invoke_latency_ms{capability}` (histogram)
- `loom.action_broker.timeouts_total{capability}`
- `loom.action_broker.cache_hits_total{capability}`

**验收**: 工具调用的延迟和成功率可见

---

#### #4: Router 决策跟踪

**Files**: `core/src/router.rs`

**Spans**:

- `router.route` (line 160) - 属性: route, confidence, reason, privacy_level

**Metrics**:

- `loom.router.decisions_total{route, reason, event_type}`
- `loom.router.confidence_score{route}` (histogram)

**验收**: 路由决策（Local/Cloud/Hybrid）在 Dashboard 中可见

---

#### #5: ToolOrchestrator 增强埋点

**Files**: `core/src/llm/tool_orchestrator.rs`

**增强已有 span** (line 110):

- 添加更多属性: tool_count, refine_enabled
- 为每个 tool call 创建 child span (line 172-190)

**Metrics**:

- `loom.tool_orch.runs_total{tool_choice}`
- `loom.tool_orch.tool_calls_total{tool_name, status}`
- `loom.tool_orch.tool_latency_ms{tool_name}` (histogram)

**验收**: LLM 工具使用模式清晰可见

---

### 🟡 P1 - High (完整可观测性)

#### #6: Agent Runtime 埋点

**Files**: `core/src/agent/runtime.rs`, `core/src/agent/instance.rs`

**Spans**: create_agent, delete_agent, agent.run (event loop)

**Metrics**:

- `loom.agent_runtime.active_agents` (gauge)
- `loom.agent.events_processed_total{agent_id}`
- `loom.agent.event_processing_latency_ms{agent_id}`

---

#### #7: MCP Manager 埋点

**Files**: `core/src/mcp/manager.rs`

**Spans**: add_server, register_tools, reconnect_server

**Metrics**:

- `loom.mcp.connected_servers` (gauge)
- `loom.mcp.tools_registered_total{server_name}`

---

#### #8: Bridge (gRPC) 埋点

**Files**: `bridge/src/lib.rs`

**Spans**: register_agent, event_stream, forward_action

**Metrics**:

- `loom.bridge.connected_agents` (gauge)
- `loom.bridge.events_forwarded_total{agent_id, direction}`
- `loom.bridge.stream_latency_ms{agent_id}`

---

### 🟢 P2 - Medium (Dashboard MVP)

#### #9: OpenTelemetry Collector 部署

**Files**: `infra/otel-collector-config.yaml`, `infra/docker-compose.yml`

创建配置文件，启动 Collector + Jaeger + Prometheus + Grafana

---

#### #10: Dashboard Backend API

**Files**: 新建 `dashboard-backend/` crate

**API Endpoints**:

- `GET /api/topology` - 生成 agent/topic/capability 拓扑图
- `GET /api/traces?thread_id={id}` - 查询 thread 的完整 trace
- `GET /api/metrics/summary` - 汇总指标
- `WebSocket /ws/events` - 实时 event stream

---

#### #11: Dashboard Frontend - Topology Graph

**Tech**: Next.js + ReactFlow/D3.js + TailwindCSS

**Features**:

- 节点: Agents (蓝色圆), Topics (黄色矩形), Capabilities (绿色六边形)
- 边: Subscriptions, Publishes (动画), Capability calls
- 实时更新 via WebSocket

---

#### #12: Dashboard Frontend - Event Swimlanes


**Features**:

- 横轴: 时间轴
- 纵轴: thread_id (每个 thread 一行)
- Event 卡片显示: timestamp, type, sender, payload 预览
- 点击展开完整详情

---

#### #13: Dashboard Frontend - Metrics Panel


**显示**:

- 关键指标卡片: Events/sec, P50/P90/P99, Error rate, Active agents
- 图表: 延迟分布直方图, 吞吐量折线图, 工具调用饼图

---

#### #14: Dashboard Frontend - Tool Timeline


**显示**:

- Gantt-style timeline
- Tool calls 显示为条形，长度=延迟
- 颜色: 绿色(成功), 红色(错误), 黄色(超时)

---

### ⚪ P3 - Low (锦上添花)

#### #15: Trace Context Propagation 完善

确保 W3C TraceContext 在 Event.metadata, ActionCall.headers, gRPC metadata 中正确传播

---

#### #16: Alerting & Health Checks

Prometheus 告警规则 + `/health` 端点

---

#### #17: Documentation

编写 `docs/observability/` 下的完整文档

---

## 验证方式

### 阶段 1 验证 (Week 2 结束)

```bash
# 启动 Jaeger
docker run -p 16686:16686 -p 4317:4317 jaegertracing/all-in-one

# 运行 Loom
OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317 cargo run

# 在 Jaeger UI 查看 traces
open http://localhost:16686
```

**预期**: 看到 EventBus publish/subscribe spans, ActionBroker invoke spans

---

### 阶段 2 验证 (Week 3 结束)

```bash
# 启动完整 stack
cd infra && docker-compose up -d

# 查看 Prometheus metrics
curl localhost:9090/api/v1/query?query=loom_event_bus_published_total
```

**预期**: 所有 metrics 都有数据

---

### 阶段 3 验证 (Week 5 结束)

```bash
# 启动 Dashboard
cd dashboard-frontend && npm run dev
open http://localhost:3000
```

**预期**:

- Topology graph 显示 agents/topics/capabilities
- Swimlanes 显示最近 100 个 events
- Metrics panel 显示实时统计

---

## 关键文件清单

### 需要修改的文件 (P0-P1)

```
core/Cargo.toml                      # 添加 opentelemetry 依赖
core/src/telemetry.rs                # 扩展为完整 OTLP 支持
core/src/lib.rs                      # 初始化 telemetry
core/src/event.rs                    # EventBus 埋点
core/src/action_broker.rs            # ActionBroker 埋点
core/src/router.rs                   # Router 埋点
core/src/llm/tool_orchestrator.rs   # 增强已有埋点
core/src/agent/runtime.rs            # Agent Runtime 埋点
core/src/agent/instance.rs           # Agent instance 埋点
core/src/mcp/manager.rs              # MCP Manager 埋点
bridge/src/lib.rs                    # Bridge 埋点
```

### 需要创建的文件 (P2)

```
infra/otel-collector-config.yaml     # OTLP Collector 配置
infra/docker-compose.yml             # 更新，添加 observability stack

dashboard-backend/                   # 新 crate
  Cargo.toml
  src/main.rs
  src/api/traces.rs
  src/api/metrics.rs
  src/api/events.rs
  src/websocket.rs

dashboard-frontend/                  # 新 Next.js app
  package.json
  app/page.tsx
  app/components/TopologyGraph.tsx
  app/components/EventSwimlanes.tsx
  app/components/MetricsPanel.tsx
  app/components/ToolTimeline.tsx
  lib/api.ts
  lib/websocket.ts
```

---

## 成功标准

✅ **Technical**:

- Trace 采样率 ≥ 10%
- 埋点开销 < 5% latency
- Dashboard 首屏加载 < 500ms
- Metrics cardinality < 10k

✅ **User**:

- 开发者能在 5 分钟内定位 multi-agent 交互问题
- 延迟回归能在 1 小时内被发现
- 工具调用失败的根因可从 Dashboard 直接看出
