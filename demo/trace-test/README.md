# Trace Test Demo

Simple 3-agent linear workflow to validate end-to-end distributed tracing.

## Architecture

```
sensor-agent (generates data)
    ↓ emits sensor.data
processor-agent (processes data)
    ↓ emits processed.data
output-agent (consumes processed data)
```

## Purpose

This demo validates:

- ✅ Trace context propagation from Python → Rust → Python
- ✅ Parent-child span relationships in Jaeger
- ✅ Complete trace timeline across all 3 agents
- ✅ Envelope trace_id/span_id carried through event flow

## Running

```bash
# Terminal 1: Start observability stack (Jaeger + Prometheus)
cd ../../observability
docker compose -f docker-compose.observability.yaml up

# Terminal 2: Run demo
cd demo/trace-test
loom run
```

## Verification

1. Open Jaeger UI: http://localhost:16686
2. Select Service: `loom-python`
3. Click "Find Traces"
4. You should see complete traces showing:
   - sensor-agent → EventBus → processor-agent → EventBus → output-agent
   - All spans with correct parent-child relationships
   - Consistent trace_id across all components

## Expected Trace Structure

```
root_span (sensor-agent.emit)
  ├── bridge_publish (bridge)
  ├── event_bus.publish (core)
  └── processor-agent.on_event
      ├── bridge_publish (bridge)
      ├── event_bus.publish (core)
      └── output-agent.on_event
```
