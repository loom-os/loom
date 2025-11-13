## Dashboard MVP

A simple real-time event-stream visualization UI for viewing event flow within the Loom system.

## Quick Start

### 1. Start the Dashboard demo

```bash
cd core
export LOOM_DASHBOARD_PORT=3030
cargo run --example dashboard_demo
```

### 2. Open your browser

```bash
open http://localhost:3030
```

You will see:

- Real-time event stream: events published to the EventBus
- Agent topology: list of registered Agents
- Key metrics: event rate, number of active Agents

## Features

### ✅ Implemented

- Real-time event stream (SSE)

  - Displays events in chronological order
  - Shows: timestamp, event_id, topic, sender, thread_id, correlation_id, payload
  - Filter by thread_id / topic / sender
  - Pause / resume automatic scrolling
  - Keeps the most recent 100 events

- Agent topology

  - Displays the list of registered Agents
  - Shows subscribed topics
  - Auto-refresh (every 5 seconds)

- Key metrics

  - Events/sec
  - Active Agents

- Zero-dependency frontend
  - Pure HTML/CSS/JS (no build step)
  - Responsive design
  - Dark theme

### 🚧 To be implemented

- Advanced visualizations

  - D3.js topology (force-directed graph)
  - Thread timeline (Gantt chart)
  - Event relationship visualization

- More metrics

  - Tool invocations/sec
  - P99 latency
  - Read real-time metrics from Prometheus

- Interactive features
  - Click an event to view details
  - Event search
  - Export event log as JSON

## API Endpoints

### `GET /`

Returns the Dashboard HTML page

### `GET /api/events/stream`

Server-Sent Events (SSE) endpoint that pushes real-time events

Response format:

```json
{
  "timestamp": "2025-11-12T10:30:00Z",
  "event_type": "event_published",
  "event_id": "event-123",
  "topic": "agent.task",
  "sender": "planner",
  "thread_id": "thread-456",
  "correlation_id": "corr-789",
  "payload_preview": "Task 1 payload..."
}
```

### `GET /api/topology`

Returns a snapshot of the current Agent topology

Response format:

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

Returns a snapshot of key metrics

Response format:

```json
{
  "events_per_sec": 0,
  "active_agents": 3,
  "active_subscriptions": 0,
  "tool_invocations_per_sec": 0
}
```

## Environment Variables

| Variable                | Default  | Description                         |
| ----------------------- | -------- | ----------------------------------- |
| `LOOM_DASHBOARD`        | `false`  | Whether to enable the Dashboard     |
| `LOOM_DASHBOARD_PORT`   | `3030`   | Dashboard HTTP port                 |
| `LOOM_DASHBOARD_HOST`   | `127.0.0.1` | Dashboard bind address           |

## Integrating into your application

```rust
use loom_core::{
    dashboard::{DashboardConfig, DashboardServer, EventBroadcaster},
    event::EventBus,
    directory::AgentDirectory,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Create core components
    let mut event_bus = EventBus::new().await?;
    let agent_directory = Arc::new(AgentDirectory::new());

    // Enable Dashboard
    let broadcaster = EventBroadcaster::new(1000);
    event_bus.set_dashboard_broadcaster(broadcaster.clone());

    // Start Dashboard server
    let config = DashboardConfig::from_env();
    let dashboard = DashboardServer::new(config, broadcaster, agent_directory);

    tokio::spawn(async move {
        dashboard.serve().await.unwrap();
    });

    // ... your application code ...

    Ok(())
}
```

## Architecture

```
┌─────────────┐
│  EventBus   │
│             │
│  publish()  ├──────┐
└─────────────┘      │
                     │ broadcast
                     ▼
             ┌───────────────────┐
             │ EventBroadcaster  │
             │  (tokio channel)  │
             └────────┬──────────┘
                      │
                      │ SSE
                      ▼
              ┌───────────────┐
              │ DashboardServer│
              │   (Axum)       │
              └────────┬───────┘
                       │
                       │ HTTP
                       ▼
                  ┌─────────┐
                  │ Browser │
                  │   UI    │
                  └─────────┘
```

## Performance

- Event buffer: 1000 events (configurable)
- Frontend limit: displays the most recent 100 events
- Update frequencies:
  - Event stream: real-time (SSE push)
  - Topology: every 5 seconds
  - Metrics: every 1 second

## Next steps

- [ ] Update ROADMAP
- [ ] Test integration with trio.py
- [ ] Add D3.js topology visualization
- [ ] Integrate Prometheus metrics
- [ ] Add Thread timeline view

Completion summary: Translation of README.md is done. Next steps I can take on request: (1) commit the translated README into the repo, (2) produce a side-by-side diff, or (3) refine wording for a specific audience (developer vs. product). Which would you prefer?
