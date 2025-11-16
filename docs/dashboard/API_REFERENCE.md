# Dashboard API Reference

Complete API documentation for the Loom Dashboard HTTP server.

## Base URL

```
http://localhost:3030
```

Configure via environment variables:

- `LOOM_DASHBOARD_HOST` (default: `127.0.0.1`)
- `LOOM_DASHBOARD_PORT` (default: `3030`)

---

## Endpoints Overview

| Method | Endpoint                | Purpose                      | Response Type           |
| ------ | ----------------------- | ---------------------------- | ----------------------- |
| `GET`  | `/`                     | Dashboard UI (HTML)          | text/html               |
| `GET`  | `/static/*asset`        | Static assets (JS, CSS)      | varies                  |
| `GET`  | `/api/events/stream`    | Real-time event stream       | text/event-stream (SSE) |
| `GET`  | `/api/events/status`    | SSE subscriber count         | application/json        |
| `GET`  | `/api/topology`         | Agent topology snapshot      | application/json        |
| `GET`  | `/api/flow`             | Flow graph snapshot          | application/json        |
| `GET`  | `/api/metrics`          | Key metrics                  | application/json        |
| `GET`  | `/api/spans/recent`     | Recent trace spans           | application/json        |
| `GET`  | `/api/traces/:trace_id` | Spans for specific trace     | application/json        |
| `GET`  | `/api/spans/stream`     | Real-time span updates       | text/event-stream (SSE) |
| `POST` | `/api/debug/emit`       | Emit synthetic event (debug) | text/plain              |

---

## GET `/`

**Description**: Serves the main Dashboard UI (React SPA).

**Response**: HTML document

**Example**:

```bash
curl http://localhost:3030/
```

**Response**:

```html
<!DOCTYPE html>
<html>
  <head>
    <title>Loom Dashboard</title>
    <link rel="stylesheet" href="/static/assets/style-XXX.css" />
  </head>
  <body>
    <div id="root"></div>
    <script src="/static/assets/index-XXX.js"></script>
  </body>
</html>
```

**Notes**:

- Built from `core/src/dashboard/frontend/` (Vite + React)
- Assets hashed for cache busting
- Falls back to placeholder if frontend not built

---

## GET `/static/*asset`

**Description**: Serves static assets (JS, CSS, images).

**Parameters**:

- `asset` (path): Asset filename (e.g., `assets/index-XXX.js`)

**Example**:

```bash
curl http://localhost:3030/static/assets/index-XXX.js
```

**Response**: Asset content with appropriate `Content-Type` header

**Status Codes**:

- `200 OK`: Asset found
- `404 Not Found`: Asset doesn't exist

---

## GET `/api/events/stream`

**Description**: Server-Sent Events (SSE) stream for real-time Dashboard events.

**Protocol**: SSE (text/event-stream)

**Connection**: Long-lived, persistent

**Example**:

```bash
curl -N http://localhost:3030/api/events/stream
```

**Response** (continuous stream):

```
data: {"timestamp":"2025-11-16T10:30:00Z","event_type":"event_published","event_id":"evt-001","topic":"agent.task","sender":"planner","thread_id":"thread-123","correlation_id":null,"payload_preview":"Task: Research AI trends","trace_id":""}

event: ping
data: {}

data: {"timestamp":"2025-11-16T10:30:05Z","event_type":"tool_invoked","event_id":"evt-002","topic":"action.search","sender":"researcher","thread_id":"thread-123","correlation_id":"corr-456","payload_preview":"web.search query: AI trends 2025","trace_id":"trace-789"}
```

**Event Types**:

| Event Type           | Description                   | Example Use Case          |
| -------------------- | ----------------------------- | ------------------------- |
| `event_published`    | Event published to EventBus   | Track all system events   |
| `event_delivered`    | Event delivered to subscriber | Verify delivery to agents |
| `agent_registered`   | New agent registered          | Monitor agent lifecycle   |
| `agent_unregistered` | Agent unregistered            | Detect agent shutdowns    |
| `tool_invoked`       | Tool/capability called        | Track tool usage          |
| `routing_decision`   | Router made decision          | Debug routing logic       |

**Heartbeat**:

- Named event `ping` sent every 10 seconds
- Keeps connection alive through proxies/firewalls
- Client should ignore or use for connection health

**JavaScript Client Example**:

```javascript
const eventSource = new EventSource("http://localhost:3030/api/events/stream");

eventSource.onmessage = (e) => {
  const event = JSON.parse(e.data);
  console.log("Dashboard event:", event);
};

eventSource.addEventListener("ping", () => {
  console.log("Heartbeat received");
});

eventSource.onerror = (err) => {
  console.error("SSE error:", err);
  eventSource.close();
};
```

**Rust Client Example**:

```rust
use futures_util::StreamExt;
use reqwest_eventsource::{Event, EventSource};

let mut es = EventSource::get("http://localhost:3030/api/events/stream");

while let Some(event) = es.next().await {
    match event {
        Ok(Event::Message(msg)) => {
            let dashboard_event: DashboardEvent = serde_json::from_str(&msg.data)?;
            println!("Received: {:?}", dashboard_event);
        }
        Ok(Event::Open) => println!("SSE connected"),
        Err(err) => eprintln!("SSE error: {}", err),
    }
}
```

---

## GET `/api/events/status`

**Description**: Get current SSE subscriber count.

**Response**: JSON

**Example**:

```bash
curl http://localhost:3030/api/events/status
```

**Response**:

```json
{
  "subscribers": 3
}
```

**Use Cases**:

- Health check for Dashboard connectivity
- Monitor client connections
- Debug SSE issues

---

## GET `/api/topology`

**Description**: Get current agent topology snapshot.

**Response**: JSON

**Example**:

```bash
curl http://localhost:3030/api/topology
```

**Response**:

```json
{
  "agents": [
    {
      "id": "planner",
      "topics": ["task.plan", "agent.planner"],
      "capabilities": ["plan.create", "plan.validate"]
    },
    {
      "id": "researcher",
      "topics": ["task.research", "agent.researcher"],
      "capabilities": ["web.search", "doc.summarize"]
    }
  ],
  "edges": [
    {
      "from_topic": "task.plan",
      "to_agent": "planner",
      "event_count": 0
    },
    {
      "from_topic": "task.research",
      "to_agent": "researcher",
      "event_count": 0
    }
  ],
  "timestamp": "2025-11-16T10:30:00Z"
}
```

**Schema**:

```typescript
interface TopologySnapshot {
  agents: AgentNode[];
  edges: TopologyEdge[];
  timestamp: string; // ISO 8601
}

interface AgentNode {
  id: string;
  topics: string[];
  capabilities: string[];
}

interface TopologyEdge {
  from_topic: string;
  to_agent: string;
  event_count: number; // Currently always 0 (placeholder)
}
```

**Use Cases**:

- Visualize agent network
- Verify agent registration
- Debug topic subscriptions

**Update Frequency**: Polled by frontend every 10 seconds (configurable)

---

## GET `/api/flow`

**Description**: Get current event flow graph (last 60 seconds of activity).

**Response**: JSON

**Example**:

```bash
curl http://localhost:3030/api/flow
```

**Response**:

```json
{
  "nodes": [
    {
      "id": "EventBus",
      "node_type": "eventbus",
      "event_count": 156,
      "topics": ["agent.task", "agent.research", "action.result"],
      "last_active_ms": 1700132400000
    },
    {
      "id": "planner",
      "node_type": "agent",
      "event_count": 42,
      "topics": ["agent.task", "agent.planner"],
      "last_active_ms": 1700132395000
    },
    {
      "id": "researcher",
      "node_type": "agent",
      "event_count": 38,
      "topics": ["agent.research", "action.search"],
      "last_active_ms": 1700132398000
    }
  ],
  "flows": [
    {
      "source": "planner",
      "target": "EventBus",
      "topic": "agent.research",
      "count": 12,
      "last_event_ms": 1700132395000
    },
    {
      "source": "EventBus",
      "target": "researcher",
      "topic": "agent.research",
      "count": 12,
      "last_event_ms": 1700132395000
    }
  ],
  "timestamp": "2025-11-16T10:33:20Z"
}
```

**Schema**:

```typescript
interface FlowGraph {
  nodes: FlowNode[];
  flows: EventFlow[];
  timestamp: string; // ISO 8601
}

interface FlowNode {
  id: string;
  node_type: "agent" | "eventbus" | "router" | "llm" | "tool" | "storage";
  event_count: number;
  topics: string[]; // Max 20 most recent
  last_active_ms: number; // Unix timestamp (milliseconds)
}

interface EventFlow {
  source: string; // Node ID
  target: string; // Node ID
  topic: string;
  count: number;
  last_event_ms: number; // Unix timestamp (milliseconds)
}
```

**Node Types**:

- `agent`: Regular agent
- `eventbus`: Central EventBus
- `router`: Model router
- `llm`: LLM client
- `tool`: Tool provider
- `storage`: Storage layer

**Retention**:

- Flows: 60 seconds
- Nodes: 120 seconds (unless EventBus, which persists)
- Topics per node: Max 20 (FIFO)

**Use Cases**:

- Animated flow visualization
- Debug event routing
- Monitor agent communication patterns

**Update Frequency**: Polled by frontend every 3 seconds

---

## GET `/api/metrics`

**Description**: Get aggregated metrics snapshot.

**Response**: JSON

**Example**:

```bash
curl http://localhost:3030/api/metrics
```

**Response**:

```json
{
  "events_per_sec": 12.5,
  "active_agents": 3,
  "active_subscriptions": 8,
  "tool_invocations_per_sec": 0
}
```

**Schema**:

```typescript
interface MetricsSnapshot {
  events_per_sec: number;
  active_agents: number;
  active_subscriptions: number;
  tool_invocations_per_sec: number;
}
```

**Calculation Details**:

- `events_per_sec`: Derived from recent FlowGraph activity (last 5 seconds)
- `active_agents`: Count of agent-type nodes in FlowGraph
- `active_subscriptions`: Count of active flows
- `tool_invocations_per_sec`: Placeholder (always 0 currently)

**Note**: This endpoint returns approximate values. For production monitoring, use OpenTelemetry metrics.

**Update Frequency**: Polled by frontend every 5 seconds

---

## GET `/api/spans/recent`

**Description**: Get recent trace spans for Timeline view.

**Query Parameters**:

- `limit` (optional): Max spans to return (default: 100, max: 1000)

**Response**: JSON array of spans

**Example**:

```bash
curl "http://localhost:3030/api/spans/recent?limit=50"
```

**Response**:

```json
[
  {
    "trace_id": "trace-abc-123",
    "span_id": "span-xyz-456",
    "parent_span_id": null,
    "name": "process_task",
    "start_time_unix_nano": 1700132400000000000,
    "end_time_unix_nano": 1700132401500000000,
    "attributes": {
      "agent.id": "planner",
      "event.id": "evt-001",
      "topic": "agent.task"
    },
    "status": "Ok"
  }
]
```

**Use Cases**:

- Timeline visualization
- Trace analysis
- Performance debugging

---

## GET `/api/traces/:trace_id`

**Description**: Get all spans for a specific trace.

**Parameters**:

- `trace_id` (path): Trace ID to query

**Response**: JSON array of spans

**Example**:

```bash
curl http://localhost:3030/api/traces/trace-abc-123
```

**Response**: Same schema as `/api/spans/recent`

**Use Cases**:

- View complete trace
- Analyze span relationships
- Debug distributed workflows

---

## GET `/api/spans/stream`

**Description**: Real-time stream of new trace spans.

**Protocol**: SSE (text/event-stream)

**Example**:

```bash
curl -N http://localhost:3030/api/spans/stream
```

**Response**:

```
event: spans
data: [{"trace_id":"...","span_id":"...","name":"..."}]

event: ping
data: {}
```

**Polling Interval**: 500ms

**Heartbeat**: Every 10 seconds

**Use Cases**:

- Live Timeline updates
- Real-time trace monitoring

---

## POST `/api/debug/emit`

**Description**: Emit a synthetic Dashboard event (debug only).

**Authentication**: Requires `LOOM_DASHBOARD_DEBUG=true`

**Content-Type**: application/json

**Request Body**:

```json
{
  "topic": "debug.test",
  "sender": "debug_client",
  "payload": "test payload"
}
```

**Response**: `200 OK` with body `ok`

**Example**:

```bash
export LOOM_DASHBOARD_DEBUG=true

curl -X POST http://localhost:3030/api/debug/emit \
  -H "Content-Type: application/json" \
  -d '{"topic":"debug.test","sender":"test","payload":"hello"}'
```

**Use Cases**:

- Test SSE client without real events
- Debug Dashboard UI
- Demo Dashboard features

**Security**: Disabled by default. Only enable in development environments.

---

## CORS Policy

All endpoints support CORS with:

```
Access-Control-Allow-Origin: *
Access-Control-Allow-Methods: *
Access-Control-Allow-Headers: *
```

**Production Recommendation**: Restrict CORS to specific origins.

---

## Rate Limiting

**Current**: No rate limiting implemented.

**Recommendation for Production**:

- Limit SSE connections per client IP
- Rate limit POST requests (when authentication added)
- Use reverse proxy (nginx, Caddy) for rate limiting

---

## Authentication

**Current**: No authentication required.

**Production Recommendation**:
Implement authentication for Dashboard access:

1. **Basic Auth** (simple):

```rust
use tower_http::auth::RequireAuthorizationLayer;

let auth_layer = RequireAuthorizationLayer::basic("admin", "password");
app.layer(auth_layer)
```

2. **JWT/OAuth** (recommended):
   Use middleware to validate tokens in Authorization header.

3. **Reverse Proxy Auth** (deployment):
   Let nginx/Caddy handle authentication, Dashboard behind proxy.

---

## Error Responses

### 404 Not Found

Endpoint doesn't exist or asset not found.

```json
{
  "error": "Not found"
}
```

### 403 Forbidden

Debug endpoint called without `LOOM_DASHBOARD_DEBUG=true`.

```
Forbidden: debug disabled
```

### 500 Internal Server Error

Server error (rare, indicates bug).

```json
{
  "error": "Internal server error"
}
```

---

## Client Integration Examples

### JavaScript (Browser)

```javascript
// SSE event stream
const es = new EventSource("/api/events/stream");
es.onmessage = (e) => {
  const event = JSON.parse(e.data);
  updateDashboard(event);
};

// Fetch topology
const topology = await fetch("/api/topology").then((r) => r.json());
console.log("Agents:", topology.agents);

// Fetch metrics
const metrics = await fetch("/api/metrics").then((r) => r.json());
console.log("Events/sec:", metrics.events_per_sec);
```

### Python

```python
import requests
import json
from sseclient import SSEClient

# SSE event stream
messages = SSEClient('http://localhost:3030/api/events/stream')
for msg in messages:
    if msg.data:
        event = json.loads(msg.data)
        print(f"Event: {event['event_id']}")

# Fetch topology
topology = requests.get('http://localhost:3030/api/topology').json()
print(f"Agents: {len(topology['agents'])}")
```

### Rust

```rust
use reqwest;
use serde_json::Value;

// Fetch topology
let topology: Value = reqwest::get("http://localhost:3030/api/topology")
    .await?
    .json()
    .await?;
println!("Agents: {}", topology["agents"].as_array().unwrap().len());

// Fetch metrics
let metrics: Value = reqwest::get("http://localhost:3030/api/metrics")
    .await?
    .json()
    .await?;
println!("Events/sec: {}", metrics["events_per_sec"]);
```

---

## Performance Characteristics

| Endpoint             | Response Time   | Update Frequency    | Memory Impact       |
| -------------------- | --------------- | ------------------- | ------------------- |
| `/`                  | <10ms           | N/A                 | Low (cached)        |
| `/api/events/stream` | Immediate (SSE) | Real-time           | ~1KB per event      |
| `/api/topology`      | <5ms            | 10s (frontend poll) | Low                 |
| `/api/flow`          | <10ms           | 3s (frontend poll)  | Medium (graph size) |
| `/api/metrics`       | <5ms            | 5s (frontend poll)  | Low                 |
| `/api/spans/recent`  | <20ms           | On-demand           | Medium (span count) |

**Optimization Tips**:

- Use SSE (`/api/events/stream`) instead of polling for real-time data
- Increase frontend poll intervals if high CPU usage observed
- Reduce `limit` parameter on `/api/spans/recent` for faster response

---

## Monitoring Dashboard API

### Health Check

```bash
# Check if server is running
curl -f http://localhost:3030/api/events/status

# Expected: {"subscribers":N}
# Exit code 0 = healthy
```

### Prometheus Metrics (Future)

Planned endpoint: `GET /metrics` (Prometheus format)

Example metrics:

```
# HELP dashboard_sse_subscribers Current SSE subscribers
# TYPE dashboard_sse_subscribers gauge
dashboard_sse_subscribers 5

# HELP dashboard_events_total Total events broadcast
# TYPE dashboard_events_total counter
dashboard_events_total{event_type="event_published"} 1234
```

---

## Troubleshooting

### SSE Connection Fails

**Symptom**: EventSource error or no events received.

**Check**:

```bash
curl -N http://localhost:3030/api/events/stream
```

**Common Issues**:

- Dashboard not started: Check `LOOM_DASHBOARD=true`
- Firewall blocking: Check port 3030 accessible
- Reverse proxy timeout: Increase proxy read timeout (nginx: `proxy_read_timeout 3600s;`)

### Empty Topology

**Symptom**: `/api/topology` returns `{"agents":[]}`

**Cause**: No agents registered in AgentDirectory.

**Fix**: Verify agents are created and `AgentDirectory::register()` called.

### Outdated Flow Graph

**Symptom**: `/api/flow` shows stale data.

**Cause**: Flows expire after 60 seconds.

**Fix**: Normal behavior. Increase retention by modifying `FLOW_RETENTION_MS` in `flow_tracker.rs`.

---

## API Versioning

**Current Version**: v1 (implicit, no version in URL)

**Future**: Breaking changes will introduce `/api/v2/` prefix.

**Stability**: API is stable for v1. Additive changes (new fields) may occur without version bump.

---

## Resources

- [Dashboard Quickstart](./DASHBOARD_QUICKSTART.md)
- [Dashboard Testing Guide](./TESTING_GUIDE.md)
- [SSE Specification](https://html.spec.whatwg.org/multipage/server-sent-events.html)
- [OpenAPI Spec](./openapi.yaml) (TODO)

---

**Last Updated**: 2025-11-16
**API Version**: v1
**Stability**: Stable
