# Telemetry Module

OpenTelemetry tracing integration for Loom Python SDK.

## Overview

This module provides OpenTelemetry (OTEL) tracing instrumentation for Loom agents:

- Automatic span creation for agent operations
- OTLP exporter for collector integration
- Context propagation across events
- Integration with Rust Core tracing

## Quick Start

### Initialize Tracing

```python
from loom import init_telemetry

# Initialize with defaults
init_telemetry()

# Custom configuration
init_telemetry(
    service_name="my-agent",
    otlp_endpoint="http://localhost:4317",
)
```

### Automatic Instrumentation

Tracing is automatically enabled for:

1. **Event Operations** (EventContext):
   - `emit()`: Publishing events
   - `request()`: Request-reply patterns
   - `tool()`: Tool invocations

2. **Cognitive Operations** (CognitiveAgent):
   - `run()`: Full cognitive loop execution
   - `run_stream()`: Streaming cognitive loop
   - Tool execution with context engineering

3. **LLM Operations** (LLMProvider):
   - `generate()`: LLM API calls
   - `stream()`: Streaming LLM responses

### Manual Spans

```python
from opentelemetry import trace

tracer = trace.get_tracer(__name__)

async def my_operation():
    with tracer.start_as_current_span("my_operation") as span:
        span.set_attribute("key", "value")
        # Do work
        result = await some_async_work()
        span.set_attribute("result.size", len(result))
        return result
```

## Configuration

### Environment Variables

```bash
# Service name for traces
export OTEL_SERVICE_NAME="my-agent"

# OTLP collector endpoint
export OTEL_EXPORTER_OTLP_ENDPOINT="http://localhost:4317"

# Deployment environment
export DEPLOYMENT_ENV="production"
```

### Programmatic Configuration

```python
from loom.telemetry import init_telemetry

init_telemetry(
    service_name="researcher-agent",
    otlp_endpoint="http://otel-collector:4317",
)
```

## Span Attributes

### Event Context Spans

**`event.emit`:**
```python
{
    "event.topic": "agent.requests",
    "event.type": "task.execute",
    "agent.id": "researcher",
    "envelope.id": "evt_123",
}
```

**`event.request`:**
```python
{
    "event.topic": "agent.requests",
    "event.type": "query",
    "agent.id": "researcher",
    "timeout.ms": 5000,
    "correlation.id": "req_456",
}
```

**`tool.invoke`:**
```python
{
    "tool.name": "web:search",
    "agent.id": "researcher",
    "timeout.ms": 5000,
    "tool.status": "TOOL_OK",
    "tool.output.size": 1024,
}
```

### Cognitive Loop Spans

**`cognitive.run`:**
```python
{
    "agent.id": "researcher",
    "goal": "Research AI trends",
    "strategy": "react",
    "max_iterations": 5,
    "iterations": 3,
    "tool_calls": 2,
    "duration.ms": 1543,
}
```

**`cognitive.tool_call`:**
```python
{
    "tool.name": "web:search",
    "tool.arguments": '{"query": "AI trends"}',
    "tool.success": true,
    "tool.latency_ms": 234,
    "context.reduced": true,
    "context.offloaded": true,
}
```

### LLM Provider Spans

**`llm.generate`:**
```python
{
    "llm.provider": "deepseek",
    "llm.model": "deepseek-chat",
    "llm.temperature": 0.7,
    "llm.max_tokens": 4096,
    "llm.prompt_tokens": 512,
    "llm.completion_tokens": 128,
    "llm.total_tokens": 640,
    "llm.duration_ms": 1234,
}
```

## Integration with Rust Core

Loom Python SDK automatically propagates trace context to Rust Core:

1. **Outgoing Events**: Trace context injected into `Envelope` headers
2. **Tool Calls**: Trace context included in gRPC metadata
3. **Correlation**: Spans linked across Python ↔ Rust boundary

### Distributed Tracing Flow

```
Python Agent (Span 1)
    └─> emit() → EventContext
            └─> Envelope.inject_trace_context()
                    └─> Rust Core Event Bus (Span 2)
                            └─> Tool Execution (Span 3)
                                    └─> Response (linked to Span 1)
```

## Observability Stack

### Collector Setup

```yaml
# docker-compose.yml
services:
  otel-collector:
    image: otel/opentelemetry-collector:latest
    ports:
      - "4317:4317"  # OTLP gRPC
      - "4318:4318"  # OTLP HTTP
    volumes:
      - ./otel-config.yaml:/etc/otel/config.yaml
    command: ["--config=/etc/otel/config.yaml"]
```

```yaml
# otel-config.yaml
receivers:
  otlp:
    protocols:
      grpc:
        endpoint: 0.0.0.0:4317

exporters:
  jaeger:
    endpoint: jaeger:14250
    tls:
      insecure: true
  prometheus:
    endpoint: 0.0.0.0:8889

service:
  pipelines:
    traces:
      receivers: [otlp]
      exporters: [jaeger]
```

### Visualization

**Jaeger UI** (http://localhost:16686):
- View distributed traces
- Analyze span durations
- Debug cross-service calls

**Grafana** (http://localhost:3000):
- Query Prometheus metrics
- Create dashboards
- Set up alerts

## Best Practices

### 1. Meaningful Span Names

```python
# ❌ Bad: Generic name
with tracer.start_as_current_span("process"):
    ...

# ✅ Good: Descriptive name
with tracer.start_as_current_span("cognitive.tool_execution"):
    ...
```

### 2. Rich Attributes

```python
# ❌ Bad: Missing context
span.set_attribute("result", "ok")

# ✅ Good: Detailed context
span.set_attribute("tool.name", tool_name)
span.set_attribute("tool.success", success)
span.set_attribute("tool.latency_ms", latency)
span.set_attribute("context.tokens_saved", tokens_saved)
```

### 3. Error Recording

```python
try:
    result = await risky_operation()
    span.set_status(trace.Status(trace.StatusCode.OK))
except Exception as e:
    span.set_status(trace.Status(trace.StatusCode.ERROR, str(e)))
    span.record_exception(e)
    raise
```

### 4. Sampling

For high-throughput agents, configure sampling:

```python
from opentelemetry.sdk.trace.sampling import TraceIdRatioBased

# Sample 10% of traces
sampler = TraceIdRatioBased(0.1)
provider = TracerProvider(sampler=sampler)
```

## Troubleshooting

### No Traces Appearing

1. Check collector endpoint:
   ```bash
   curl http://localhost:4317  # Should respond
   ```

2. Verify initialization:
   ```python
   from loom import init_telemetry
   init_telemetry()  # Call before any operations
   ```

3. Check environment:
   ```bash
   echo $OTEL_EXPORTER_OTLP_ENDPOINT
   ```

### High Overhead

1. Enable sampling:
   ```python
   # Sample 10% of traces
   init_telemetry(sampling_rate=0.1)
   ```

2. Disable in production:
   ```python
   if os.getenv("ENV") != "production":
       init_telemetry()
   ```

### Missing Attributes

Ensure spans are created in async context:

```python
# ❌ Bad: Span lost in async
def sync_function():
    with tracer.start_as_current_span("sync"):
        asyncio.run(async_work())  # Context lost!

# ✅ Good: Async all the way
async def async_function():
    with tracer.start_as_current_span("async"):
        await async_work()  # Context preserved
```

## API Reference

### Functions

**`init_telemetry(service_name, otlp_endpoint)`**
- Initialize OpenTelemetry tracing
- Args:
  - `service_name`: Service identifier (default: "loom-python")
  - `otlp_endpoint`: Collector endpoint (default: "http://localhost:4317")
- Returns: None
- Note: Idempotent (safe to call multiple times)

**`shutdown_telemetry()`**
- Flush pending spans and shutdown
- Blocks until all spans are exported
- Call before process exit

### Usage

```python
from loom import init_telemetry, shutdown_telemetry
import atexit

# Initialize at startup
init_telemetry(service_name="my-agent")

# Register cleanup
atexit.register(shutdown_telemetry)

# Your agent code here...
```

## Related Modules

- **agent**: EventContext with automatic tracing
- **cognitive**: CognitiveAgent with span instrumentation
- **llm**: LLMProvider with API call tracing
- **bridge**: gRPC metadata propagation

## See Also

- [OpenTelemetry Python Docs](https://opentelemetry.io/docs/instrumentation/python/)
- [Observability Setup](../../../docs/observability/README.md)
- [Grafana Dashboards](../../../observability/grafana/)
