# Distributed Tracing Implementation Summary

## 🎯 目标

实现 market-analyst demo 的全链路分布式追踪，解决 P0 Critical Gap #1。

## ✅ 已完成工作

### 1. Rust Core - Envelope 扩展 (/core/src/envelope.rs)

**新增字段**：

- `trace_id`: OpenTelemetry trace ID (128-bit hex)
- `span_id`: OpenTelemetry span ID (64-bit hex)
- `trace_flags`: Trace flags (8-bit hex, typically "01" for sampled)

**新增方法**：

```rust
pub fn inject_trace_context(&mut self)
pub fn extract_trace_context(&self) -> bool
```

**自动注入点**：

- `EventBus::publish()` - 在事件发布前自动注入当前 span 的 trace context
- `ActionBroker::invoke()` - 在 action 调用前自动注入 trace context

### 2. Bridge - Trace Propagation (/bridge/src/lib.rs)

**event_stream 处理**：

- 从 Python ClientEvent 提取 trace context
- 使用`envelope.extract_trace_context()`设置远程父 span
- 创建`bridge_publish` span 继续 trace 链路
- 包含属性：agent_id, topic, event_id, trace_id, span_id

### 3. Python SDK - OpenTelemetry 集成

**依赖添加** (pyproject.toml):

```toml
opentelemetry-api>=1.22.0
opentelemetry-sdk>=1.22.0
opentelemetry-exporter-otlp-proto-grpc>=1.22.0
```

**envelope.py 扩展**：

- 添加 trace_id/span_id/trace_flags 字段
- `inject_trace_context()` - 从当前 span 注入
- `extract_trace_context()` - 提取并返回 SpanContext

**context.py 修改**：

- `emit()` - 自动调用`env.inject_trace_context()`

**agent.py 修改**：

- `_run_stream()` - 在 on_event 前提取 trace context 并创建子 span
- 创建`agent.on_event` span with attributes (agent.id, event.id, event.type, topic, thread_id, correlation_id)

**tracing.py (新模块)**：

- `init_telemetry()` - 初始化 OTLP exporter 和 TracerProvider
- `shutdown_telemetry()` - 优雅关闭并刷新 pending spans
- 支持环境变量：OTEL_SERVICE_NAME, OTEL_EXPORTER_OTLP_ENDPOINT

### 4. Trace Test Demo (/demo/trace-test/)

**简化的 3-agent 线性 workflow**：

```
sensor-agent → sensor.data → processor-agent → processed.data → output-agent
```

**目的**：

- 验证 Python → Rust → Python 的完整 trace 链路
- 验证 parent-child span 关系
- 避免 market-analyst 的复杂 fan-out/fan-in

**文件**：

- `loom.toml` - agent 配置
- `agents/sensor.py` - 数据生成器（每 2 秒）
- `agents/processor.py` - 数据处理器（×1.5）
- `agents/output.py` - 数据消费者

## 📋 下一步行动

### Priority 1: 测试 trace-test demo

```bash
# Terminal 1: 启动observability stack
cd observability
docker compose -f docker-compose.observability.yaml up

# Terminal 2: 运行demo
cd demo/trace-test
loom run

# Terminal 3: 查看Jaeger
open http://localhost:16686
```

**验证项**：

- [ ] Jaeger 中能看到完整 trace
- [ ] sensor → processor → output 的 span hierarchy 正确
- [ ] trace_id 在所有 span 中一致
- [ ] Python spans 有正确的 attributes

### Priority 2: Dashboard 集成 (TODO #5)

- 修改`flow_tracker.rs`添加 trace_id 字段
- 修改 EventFlow struct 包含 trace_id
- Dashboard UI 显示 trace_id 并链接到 Jaeger

### Priority 3: Market-Analyst 验证 (TODO #6)

- 在 data/trend/risk/sentiment/planner agents 中添加 init_telemetry()
- 验证 fan-out/fan-in 的 trace 拓扑
- 确认 LLM 调用的 span 可见

### Priority 4: E2E 测试和文档 (TODO #7)

- 添加 integration test 验证 trace propagation
- 更新 ROADMAP.md 标记 tracing 完成
- 创建 docs/observability/TRACING.md

## 🏗️ 架构图

```
┌─────────────────────────────────────────────────────────────┐
│                    Distributed Trace Flow                   │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  Python Agent A                                             │
│  ┌──────────────┐                                          │
│  │ agent.emit() │ ←── inject_trace_context()              │
│  │  span_id: A1 │                                          │
│  └──────┬───────┘                                          │
│         │ gRPC ClientEvent                                 │
│         ↓                                                   │
│  ┌─────────────────────┐                                   │
│  │ Bridge              │                                   │
│  │ extract_trace_ctx() │ ←── read from Event.metadata     │
│  │ span_id: B1         │                                   │
│  │ parent: A1          │                                   │
│  └─────────┬───────────┘                                   │
│            │                                                │
│            ↓                                                │
│  ┌─────────────────────┐                                   │
│  │ EventBus.publish()  │                                   │
│  │ inject_trace_ctx()  │ ←── read from current span       │
│  │ span_id: E1         │                                   │
│  │ parent: B1          │                                   │
│  └─────────┬───────────┘                                   │
│            │                                                │
│            ↓                                                │
│  ┌─────────────────────┐                                   │
│  │ Bridge → Python B   │                                   │
│  │ extract_trace_ctx() │ ←── read from Event.metadata     │
│  │ span_id: B2         │                                   │
│  │ parent: E1          │                                   │
│  └─────────┬───────────┘                                   │
│            │                                                │
│            ↓                                                │
│  ┌──────────────────────┐                                  │
│  │ Agent B.on_event()   │                                  │
│  │ span_id: A2          │                                  │
│  │ parent: B2           │                                  │
│  └──────────────────────┘                                  │
│                                                             │
│  Jaeger displays:                                          │
│  trace_id: XXX (same across all spans)                     │
│  ├─ A1 (Python emit)                                       │
│  │  ├─ B1 (Bridge receive)                                │
│  │  │  ├─ E1 (EventBus publish)                           │
│  │  │  │  ├─ B2 (Bridge forward)                          │
│  │  │  │  │  └─ A2 (Python on_event)                      │
└─────────────────────────────────────────────────────────────┘
```

## 🔑 关键代码片段

### Rust: Envelope 注入

```rust
// In EventBus::publish()
let mut envelope = crate::Envelope::from_event(&event);
envelope.inject_trace_context();
envelope.attach_to_event(&mut event);
```

### Rust: Bridge 提取

```rust
// In event_stream inbound handler
let envelope = loom_core::Envelope::from_event(&ev);
envelope.extract_trace_context();

let span = tracing::info_span!(
    "bridge_publish",
    trace_id = %envelope.trace_id,
    span_id = %envelope.span_id
);
```

### Python: Agent 处理

```python
# In agent._run_stream()
env = Envelope.from_proto(delivery.event)
parent_ctx = env.extract_trace_context()
if parent_ctx:
    ctx = set_span_in_context(trace.NonRecordingSpan(parent_ctx))

with tracer.start_as_current_span("agent.on_event", context=ctx):
    await self._on_event(self._ctx, delivery.topic, env)
```

## 💡 设计决策

1. **自动注入** - EventBus 和 ActionBroker 自动注入，无需手动调用
2. **向后兼容** - trace 字段为 Optional，不影响现有代码
3. **标准格式** - 使用 W3C Trace Context 格式（128-bit trace_id, 64-bit span_id）
4. **Envelope 为载体** - 统一使用 Envelope 传递 trace context，避免分散
5. **Environment-based 配置** - OTEL_SERVICE_NAME, OTEL_EXPORTER_OTLP_ENDPOINT

## 🐛 已知问题

1. **Python 依赖未安装** - 需要`pip install -e loom-py`重新安装
2. **Dashboard 未集成** - FlowTracker 还没有 trace_id 字段
3. **Market-Analyst 未更新** - agents 需要调用 init_telemetry()

## 📚 参考资料

- [OpenTelemetry Python](https://opentelemetry-python.readthedocs.io/)
- [W3C Trace Context](https://www.w3.org/TR/trace-context/)
- [Jaeger UI Guide](https://www.jaegertracing.io/docs/latest/frontend-ui/)
- [ROADMAP.md](../../docs/ROADMAP.md) - P0 Critical Gap #1

---

**Status**: ✅ Core implementation 完成，等待 testing 验证
**Next**: 运行 trace-test demo 并验证 Jaeger traces
