# Dashboard Quick Start Guide

Real-time event flow visualization for Loom multi-agent systems.

## Quick Demo (30 seconds)

```bash
# Terminal 1: Start Dashboard with demo events
cd core
export LOOM_DASHBOARD_PORT=3030
cargo run --example dashboard_demo

# Terminal 2: Open in browser
open http://localhost:3030
```

You should see:

- **Event Stream**: Real-time events flowing through the system
- **Agent Topology**: 3 registered agents (planner, researcher, writer)
- **Metrics**: Events/sec counter

## Usage Scenarios

### Scenario 1: Debug Python Multi-Agent System (trio.py)

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

### Agent Topology

**What you see:**

```
┌──────────────────────────┐
│ 3 Agents registered      │
├──────────────────────────┤
│ planner                  │
│ researcher               │
│ writer                   │
└──────────────────────────┘
```

**Updates**: Auto-refreshes every 5 seconds

### Metrics Cards

```
┌─────────────┬─────────────┐
│ Events/sec  │ Active      │
│     12      │  Agents     │
│             │      3      │
└─────────────┴─────────────┘
```

## API Endpoints

### `GET /`

Main Dashboard HTML page

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
- **Frontend display**: Last 100 events
- **Update frequency**:
  - Events: Real-time (SSE push)
  - Topology: 5 seconds
  - Metrics: 1 second
- **Overhead**: ~5-10ms per event published (broadcast to channel)

## Next Steps

- See `core/src/dashboard/README.md` for architecture details
- See `core/examples/dashboard_demo.rs` for integration example
- See `loom-py/examples/trio.py` for Python multi-agent example
- See `docs/observability/QUICKSTART.md` for full observability stack

## Roadmap

**Current (MVP v0.1)**:

- ✅ Real-time event stream (SSE)
- ✅ Basic agent topology (list view)
- ✅ Key metrics cards
- ✅ Event filtering
- ✅ Zero-build frontend

**Coming Soon**:

- 🚧 D3.js force-directed topology graph
- 🚧 Thread timeline / Gantt chart
- 🚧 Prometheus metrics integration
- 🚧 Event detail modal
- 🚧 Export to JSON
- 🚧 Search and advanced filtering
