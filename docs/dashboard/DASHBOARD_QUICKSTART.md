# Dashboard Quick Start Guide

Real-time event flow visualization for Loom multi-agent systems.

## Quick Demo (60 seconds)

```bash
# One time (or when frontend changes): build dashboard assets
cd core/src/dashboard/frontend
npm install
npm run build
cd ../../..

# Start Dashboard with demo events
export LOOM_DASHBOARD_PORT=3030
cargo run --example dashboard_demo

# Open in browser
open http://localhost:3030
```

You should see:

- **Hero + Metrics**: Snapshot cards for events/sec, active agents, routing decisions, latency, QoS mix
- **Event Flow**: Streaming list with per-agent filters, QoS badges, thread/correlation hints
- **Agent Communications**: Tool calls, messages, outputs with timestamps and results
- **Agent Network Graph**: Animated canvas showing agent-to-agent traffic with glowing particles

## Usage Scenarios

### Scenario 1: Debug Python Multi-Agent System (trio.py)

**Step 0** (first run): Build dashboard assets

```bash
cd core/src/dashboard/frontend
npm install
npm run build
cd ../../..
```

**Step 1**: Start Loom Core with Dashboard enabled

```bash
cd core
export LOOM_DASHBOARD=true
export LOOM_DASHBOARD_PORT=3030
cargo run --release
```

**Step 2**: Run your Python agents

```bash
cd loom-py/examples
python trio.py
```

**Step 3**: View real-time events at http://localhost:3030

Filter by:

- **thread_id**: See events belonging to specific conversation threads
- **topic**: Filter by `agent.task`, `agent.research`, etc.
- **sender**: Track events from specific agents (`planner`, `researcher`, `writer`)

### Scenario 2: Integrate Dashboard into Your Rust Application

```rust
use loom_core::{
    dashboard::{DashboardConfig, DashboardServer, EventBroadcaster},
    event::EventBus,
    directory::AgentDirectory,
};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Initialize telemetry
    loom_core::telemetry::init_telemetry()?;

    // Create core components
    let mut event_bus = EventBus::new().await?;
    let agent_directory = Arc::new(AgentDirectory::new());

    // Enable Dashboard
    let broadcaster = EventBroadcaster::new(1000);
    event_bus.set_dashboard_broadcaster(broadcaster.clone());

    let event_bus = Arc::new(event_bus);

    // Start Dashboard server (non-blocking)
    let config = DashboardConfig::from_env();
    let dashboard = DashboardServer::new(
        config.clone(),
        broadcaster,
        agent_directory.clone()
    );

    tokio::spawn(async move {
        if let Err(e) = dashboard.serve().await {
            eprintln!("Dashboard error: {}", e);
        }
    });

    println!("Dashboard: http://{}:{}", config.host, config.port);

    // Your application code here...
    // Events will automatically appear in Dashboard

    Ok(())
}
```

### Scenario 3: Production Monitoring

**Step 1**: Deploy with observability stack

```bash
# Start Prometheus/Jaeger/Grafana
cd observability
docker compose -f docker-compose.observability.yaml up -d

# Start Loom with both Dashboard and OpenTelemetry
export LOOM_DASHBOARD=true
export LOOM_DASHBOARD_PORT=3030
export OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317
export OTEL_SERVICE_NAME=loom-production
cargo run --release
```

**Step 2**: Access monitoring tools

- **Dashboard** (event flow): http://localhost:3030
- **Grafana** (metrics/dashboards): http://localhost:3000 (admin/admin)
- **Jaeger** (distributed tracing): http://localhost:16686
- **Prometheus** (raw metrics): http://localhost:9090

## Environment Variables

| Variable              | Default     | Description             |
| --------------------- | ----------- | ----------------------- |
| `LOOM_DASHBOARD`      | `false`     | Enable Dashboard server |
| `LOOM_DASHBOARD_PORT` | `3030`      | Dashboard HTTP port     |
| `LOOM_DASHBOARD_HOST` | `127.0.0.1` | Dashboard bind address  |

## Dashboard Features

### Real-time Event Stream

**What you see:**

```
┌──────────────────────────────────────────────────┐
│ agent.task                          10:30:15 AM  │
│ event_published                                   │
│ sender: planner | thread: thread-123 | corr: ... │
│ {"task": "Research latest AI trends"}            │
└──────────────────────────────────────────────────┘
```

**Interactions:**

- **Filter by thread_id**: Type in filter box to see only events from specific threads
- **Filter by topic**: Show only events on specific topics (e.g., `agent.task`)
- **Filter by sender**: Track events from specific agents
- **Pause auto-scroll**: Click "Auto-scroll" button to pause and inspect events
- **Clear**: Remove all events from view

### Agent Communications

- Tool invocations, agent-to-agent messages, and outputs rendered chronologically
- Badge color highlights communication type (tool call, output, message)
- Thread identifiers (when present) displayed for quick correlation

### Metrics Cards

- Events/sec (client-side rate derived from SSE timestamps while backend metrics mature)
- Active agents (from topology snapshot)
- Routing decisions & latency (placeholder until OpenTelemetry metrics land)
- QoS distribution bar chart (realtime/batched/background mix)

### Agent Network Graph

- Canvas-based circular layout of active agents with animated message particles
- Node glow indicates recent activity (`active`, `processing`, `idle`)
- Connection list below the graph summarizes capabilities per agent
- Shows latest inter-agent messages derived from flow tracker data

## API Endpoints

### `GET /`

Main Dashboard HTML page. Serves the Vite-built React bundle (`/static/index.html` + hashed assets).

### `GET /api/events/stream`

**Server-Sent Events (SSE)** stream for real-time events

**Response format**:

```json
{
  "timestamp": "2025-11-12T10:30:00Z",
  "event_type": "event_published",
  "event_id": "event-123",
  "topic": "agent.task",
  "sender": "planner",
  "thread_id": "thread-456",
  "correlation_id": "corr-789",
  "payload_preview": "Task payload..."
}
```

**Event types**:

- `event_published`: Event published to EventBus
- `event_delivered`: Event delivered to subscriber
- `agent_registered`: New agent registered
- `agent_unregistered`: Agent unregistered
- `tool_invoked`: Tool/capability invoked
- `routing_decision`: Router made a decision

### `GET /api/topology`

Agent topology snapshot (JSON)

**Response**:

```json
{
  "agents": [
    {
      "id": "planner",
      "topics": ["agent.task"],
      "capabilities": ["plan.create"]
    }
  ],
  "edges": [
    {
      "from_topic": "agent.task",
      "to_agent": "planner",
      "event_count": 0
    }
  ],
  "timestamp": "2025-11-12T10:30:00Z"
}
```

### `GET /api/metrics`

Key metrics snapshot (JSON)

**Response**:

```json
{
  "events_per_sec": 12,
  "active_agents": 3,
  "active_subscriptions": 5,
  "tool_invocations_per_sec": 2
}
```

## Troubleshooting

### Dashboard not loading

**Check if server is running**:

```bash
curl http://localhost:3030
```

**Check logs**:

```bash
# Should see:
# INFO dashboard: Starting Dashboard server addr=127.0.0.1:3030
# INFO dashboard: Dashboard server ready url=http://127.0.0.1:3030
```

### No events appearing

**Verify EventBroadcaster is connected**:

```rust
// Make sure you called:
event_bus.set_dashboard_broadcaster(broadcaster);
```

**Check browser console** (F12):

- Should see: `SSE connected`
- Should NOT see: `SSE connection error`

### Events appearing but filtered out

**Clear all filters**:

- Empty the thread_id filter box
- Empty the topic filter box
- Empty the sender filter box

### High memory usage

**Reduce event buffer**:

```rust
// Default is 1000 events
let broadcaster = EventBroadcaster::new(100);  // Keep only 100
```

**Frontend keeps last 100 events automatically**

## Performance

- **Backend buffer**: 1000 events (configurable)
- **Frontend display**: Last 200 events (oldest trimmed automatically)
- **Update frequency**:
  - Events: Real-time (SSE push)
  - Topology snapshot: 10 seconds
  - Flow graph: 3 seconds
  - Metrics: 5 seconds (client-side rate derived from SSE when API returns placeholders)
- **Overhead**: ~5-10ms per event published (broadcast to channel)

## Next Steps

- See `core/src/dashboard/README.md` for architecture details
- See `core/examples/dashboard_demo.rs` for integration example
- See `loom-py/examples/trio.py` for Python multi-agent example
- See `docs/observability/QUICKSTART.md` for full observability stack

## Roadmap

**Current (React bundle v0.2)**:

- ✅ Real-time event stream (SSE) with per-agent filtering
- ✅ Agent communications feed (messages, tool invocations, outputs)
- ✅ Canvas-based agent network graph with animated message particles
- ✅ Metrics overview cards + QoS distribution (client-side enrichment)
- ✅ Vite-built React + shadcn UI served directly from `core` binary

**Coming Soon**:

- 🚧 Server-side metrics aggregator (replace placeholder `/api/metrics` values)
- 🚧 Event detail modal and search across payloads
- 🚧 Thread timeline / swimlane visualization
- 🚧 Export event log / flow data as JSON
- 🚧 Additional topology overlays (backpressure, error hotspots)
