# Dashboard Timeline Feature

## Overview

The Timeline feature provides a **swimlane visualization** of distributed traces across Loom's multi-agent system. It displays spans from Rust components (EventBus, Bridge, ActionBroker) and Python agents in a unified timeline view.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     OpenTelemetry Layer                       │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │ Rust Runtime │  │    Bridge    │  │ Python Agent │      │
│  │   (spans)    │  │   (spans)    │  │   (spans)    │      │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘      │
│         │                  │                  │              │
│         └──────────────────┴──────────────────┘              │
│                            │                                 │
│                    ┌───────▼────────┐                        │
│                    │  SpanCollector │                        │
│                    │  (ring buffer) │                        │
│                    └───────┬────────┘                        │
└────────────────────────────┼──────────────────────────────────┘
                             │
                    ┌────────▼─────────┐
                    │  Dashboard API   │
                    │  - /api/spans/*  │
                    └────────┬─────────┘
                             │
                    ┌────────▼─────────┐
                    │ Timeline UI      │
                    │ (React + Vite)   │
                    └──────────────────┘
```

## Components

### Backend (Rust)

1. **SpanCollector** (`core/src/telemetry.rs`)

   - Implements OpenTelemetry `SpanProcessor` trait
   - Collects spans in a ring buffer (max 10,000)
   - Indexes by trace_id for fast lookup
   - Converts OTel spans to simplified `SpanData` format

2. **Dashboard API** (`core/src/dashboard/api.rs`)

   - `GET /api/spans/recent?limit=N` - Get recent spans
   - `GET /api/traces/{trace_id}` - Get spans for specific trace
   - `GET /api/spans/stream` - SSE stream of new spans

3. **Span Instrumentation**
   - Bridge: `forward_action`, `publish`, `forward` spans
   - EventBus: `publish`, `deliver` spans
   - ActionBroker: `invoke` spans
   - Python SDK: `agent.on_event` spans

### Frontend (React + TypeScript)

1. **Timeline Page** (`src/pages/Timeline.tsx`)

   - Swimlane visualization with track per agent/component
   - Horizontal timeline with proportional span widths
   - Hover tooltips with span details
   - Trace filtering and live/pause controls

2. **Visual Design**
   - Follows existing Dashboard color scheme (cyan/blue gradient)
   - Uses shadcn/ui components (Card, Badge, Button, Select)
   - Responsive layout with custom scrollbars
   - Status-based colors (ok=accent, error=destructive)

## Usage

### Starting the System

```bash
# Terminal 1: Start observability stack (optional, for Jaeger UI)
cd observability
docker-compose up

# Terminal 2: Start Loom with trace-test demo
cd demo/trace-test
loom run

# Dashboard automatically starts at http://localhost:3000
```

### Viewing Timeline

1. Open browser to `http://localhost:3000`
2. Click "View Trace Timeline" button
3. Observe spans in swimlane visualization:
   - Each row = one agent/component
   - Horizontal bars = span duration
   - Hover for details (name, duration, status, attributes)
4. Use controls:
   - **Trace filter**: Select specific trace to isolate
   - **Pause/Resume**: Control live updates
   - **Refresh**: Manual refresh

### Testing

```bash
# Quick API test
./test_span_api.sh

# Full Timeline test
./test_timeline.sh
```

## Span Data Structure

```typescript
interface SpanData {
  trace_id: string; // W3C trace ID (hex)
  span_id: string; // W3C span ID (hex)
  parent_span_id?: string; // Parent span ID
  name: string; // e.g., "bridge.publish", "agent.on_event"
  start_time: number; // Unix timestamp in nanoseconds
  duration: number; // Duration in nanoseconds
  attributes: {
    // Span attributes
    agent_id?: string;
    topic?: string;
    correlation_id?: string;
    // ... other attributes
  };
  status: "ok" | "error" | "unset";
  error_message?: string;
}
```

## Trace Context Propagation

1. **Python Agent → Bridge**:

   - `ctx.emit()` injects trace context via `Envelope.inject_trace_context()`
   - Bridge extracts context from event metadata

2. **Bridge → EventBus**:

   - Bridge creates `bridge.publish` span with remote parent
   - EventBus receives event with trace context

3. **EventBus → Python Agent**:
   - Bridge creates `bridge.forward` span
   - Python Agent extracts context via `Envelope.extract_trace_context()`
   - Creates `agent.on_event` span as child

## Performance Considerations

- **SpanCollector buffer**: 10,000 spans (~1MB memory)
- **SSE poll interval**: 500ms
- **Timeline render**: Optimized for 200 spans per view
- **Frontend caching**: React Query with 5s stale time

## Future Enhancements

1. **Span search**: Search by name, attribute, or trace ID
2. **Flamegraph view**: Hierarchical call graph
3. **Latency heatmap**: Visual latency distribution
4. **Export**: Download trace data as JSON
5. **Critical path**: Highlight slowest spans
6. **Jaeger integration**: Direct link to Jaeger UI for deep dive

## Troubleshooting

### No spans showing up

1. Check if OpenTelemetry is initialized:

   ```bash
   # Should see in logs:
   OpenTelemetry initialized successfully with SpanCollector
   ```

2. Verify OTEL environment variables:

   ```bash
   export OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317
   export OTEL_SERVICE_NAME=loom-core
   ```

3. Check Dashboard API:
   ```bash
   curl http://localhost:3000/api/spans/recent
   ```

### Spans disconnected (gaps in timeline)

- This is expected for async operations
- Python agents and Bridge have slight delays
- Check `bridge.forward` and `agent.on_event` spans for continuity

### Timeline not updating

1. Check SSE connection in browser DevTools (Network tab)
2. Verify `isLive` is enabled (Pause button should show "Pause")
3. Refresh page or click "Refresh" button

## Related Documentation

- [ROADMAP.md](../../ROADMAP.md) - P0 Timeline feature tracking
- [TRACING_IMPL.md](../observability/TRACING_IMPL.md) - Distributed tracing details
- [DASHBOARD_QUICKSTART.md](./DASHBOARD_QUICKSTART.md) - Dashboard setup

## API Reference

### GET /api/spans/recent

Query Parameters:

- `limit` (optional): Max spans to return (default: 100, max: 1000)

Response: `Array<SpanData>`

### GET /api/traces/:trace_id

Path Parameters:

- `trace_id`: W3C trace ID (32 hex chars)

Response: `Array<SpanData>`

### GET /api/spans/stream

SSE stream with `event: spans` containing batch of new spans.

Response format:

```
event: spans
data: [{"trace_id": "...", ...}, ...]
```
