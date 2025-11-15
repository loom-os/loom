# 🔍 Trace Test 完整验证指南

## 当前修改 ✅

### 1. **Sensor Agent** - 添加 Root Span

```python
# agents/sensor.py 现在会创建root span
with tracer.start_as_current_span("sensor.emit_data", ...):
    await agent._ctx.emit(...)
```

### 2. **Bridge Server** - 初始化 Telemetry

```rust
// bridge/src/bin/server.rs 现在会初始化OpenTelemetry
loom_core::telemetry::init_telemetry()
```

### 3. **Python SDK** - 重新安装

```bash
conda run -n loom pip install -e loom-py
```

---

## 🚀 运行测试

### 方法 1: 使用脚本 (推荐)

```bash
cd /home/jared/loom/demo/trace-test
./run_test.sh
```

### 方法 2: 手动运行

```bash
# 1. 确保observability stack运行中
cd /home/jared/loom/observability
docker compose -f docker-compose.observability.yaml up -d

# 2. 等待10秒让服务启动

# 3. 设置环境变量并运行
cd /home/jared/loom/demo/trace-test
export OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317
export OTEL_SERVICE_NAME=trace-test
conda run -n loom loom run

# 4. 运行30秒后Ctrl+C停止
```

---

## 📊 预期结果

### Jaeger UI (http://localhost:16686)

#### 步骤 1: 选择 Service

应该看到**至少 3 个 services**：

- ✅ `trace-test-sensor` (新增！)
- ✅ `trace-test-processor`
- ✅ `trace-test-output`
- ✅ `loom-core` (可能需要单独查询)

#### 步骤 2: 查看 Trace

点击任意 trace，应该看到：

```
Trace Timeline (约7-10个spans):

├─ sensor.emit_data (Python sensor-agent) ← ROOT SPAN
│  └─ agent.on_event (Python processor-agent)
│     ├─ (可能) bridge.event_stream (Rust Bridge)
│     ├─ (可能) event_bus.publish (Rust Core)
│     └─ agent.on_event (Python output-agent)
```

**关键验证点**:

1. ✅ **Trace ID 相同** - 所有 spans 共享同一个 trace_id
2. ✅ **Parent-Child 关系** - 树状结构清晰
3. ✅ **3 个 Python agents** - sensor → processor → output
4. ✅ **Spans 数量** - 每个 trace 至少 5-7 个 spans（不再是 2 个）

#### 步骤 3: 检查 Span 详情

点击任意 span，查看：

- **Tags**: 应该包含`agent.id`, `event.id`, `topic`等
- **Process**: 显示 service name
- **Duration**: 显示执行时间
- **Logs**: 可能包含 event payload 预览

---

## 🔍 Troubleshooting

### 问题 1: 看不到 sensor service

**可能原因**:

- sensor.py 没有成功启动
- init_telemetry()失败

**解决**:

```bash
# 检查logs目录
cat logs/sensor-agent.log

# 手动运行sensor
conda run -n loom python agents/sensor.py
```

### 问题 2: Traces 仍然分离

**可能原因**:

- Trace context 没有正确传播
- envelope.inject_trace_context()失败

**验证**:
在 output.py 的 handler 中添加：

```python
print(f"[output] Trace ID: {event.trace_id}")
print(f"[output] Span ID: {event.span_id}")
```

应该看到非空的 trace_id。

### 问题 3: 看不到 Rust spans

**可能原因**:

- Bridge 没有重新编译
- OTEL_EXPORTER_OTLP_ENDPOINT 未设置

**解决**:

```bash
# 重新编译
cd /home/jared/loom
cargo build --release -p loom-bridge

# 确保环境变量正确
export OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317

# 检查bridge是否使用正确的binary
loom down
loom up
```

### 问题 4: Jaeger 没有数据

**检查 OTLP Collector**:

```bash
# 查看collector日志
docker logs loom-otel-collector

# 应该看到类似输出：
# Trace received with span count: X
```

**检查 Python 是否发送**:

```bash
# 在Python agent日志中应该看到：
# [loom.tracing] OpenTelemetry initialized: service=trace-test-sensor, endpoint=http://localhost:4317
```

---

## 🎯 成功标准

运行测试后，你应该能够：

- [ ] 在 Jaeger 中看到 3 个 Python services
- [ ] 每个 trace 包含 5-10 个 spans（不是 2 个）
- [ ] 同一个 trace_id 贯穿 sensor → processor → output
- [ ] Trace timeline 显示完整的 event flow
- [ ] 点击 span 可以看到详细的 tags 和 metadata
- [ ] 能追踪单个 event 从产生到消失的完整路径

如果以上都满足，恭喜！分布式追踪已经成功实现！🎉

---

## 📈 下一步

一旦 trace-test 验证成功，可以：

1. **应用到 Market-Analyst** - 在 5 个 agents 中添加 init_telemetry()
2. **Dashboard 集成** - 添加 trace_id 显示和 Jaeger 链接
3. **性能优化** - 分析 trace 找出瓶颈
4. **告警配置** - 基于 trace latency 设置告警

---

## 🔗 相关文档

- [VERIFY_TRACE.md](./VERIFY_TRACE.md) - 问题诊断
- [README.md](./README.md) - Demo 说明
- [../observability/TRACING_IMPL.md](../../docs/observability/TRACING_IMPL.md) - 实现细节
