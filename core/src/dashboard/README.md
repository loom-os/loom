# Dashboard — Real-time Event Flow Visualization

Interactive dashboard for monitoring and visualizing how events move through the Loom system. Includes a real-time animated graph (D3.js) that shows event flow between agents, components, and the EventBus, alongside a live event stream, agent topology, and key metrics.

## Quick Start

### 1) Run the demo

```bash
cd core
export LOOM_DASHBOARD_PORT=3030
cargo run --example dashboard_demo
```

### 2) Open the UI

```bash
open http://localhost:3030
```

You’ll see:

- Event Flow Graph: Interactive D3.js visualization of real-time flows
- Event Stream: Chronological list of events published to the EventBus
- Agent Topology: Registered agents and their topic subscriptions
- Key Metrics: Events/sec, Active Agents

## Features

### Implemented

- Event Flow Visualization (new)

  - D3.js force-directed graph with animated active links
  - Color-coded nodes: Agent, EventBus, Router, LLM, Tool, Storage
  - Per-node event count badge; draggable layout
  - Auto-updates every 2 seconds; shows recent flows (last 30s)

- Real-time Event Stream (SSE)

  - Timestamp, event_id, topic, sender, thread_id, correlation_id, payload preview
  - Filter by thread_id / topic / sender
  - Pause/resume auto-scroll; keeps last 100 events

- Agent Topology

  - Lists registered agents and subscribed topics
  - Auto-refresh every 5 seconds

- Key Metrics

  - Events/sec, Active Agents (MVP placeholders)

- Zero-dependency Frontend
  - Pure HTML/CSS/JS + D3.js (CDN); no build step; dark theme

### Roadmap

- Advanced visualizations: thread timeline (Gantt), event correlation
- More metrics: tool invocations/sec, P99 latency, Prometheus integration
- Interactions: node/event details, search, JSON export, topic filters

## Important: Metrics are placeholders

Today the Metrics endpoint returns static zeros as placeholders. Live metrics (events/sec, active agents, tool invocations, etc.) will be wired up via OpenTelemetry/Prometheus in a follow-up. See docs/dashboard/METRICS_INTEGRATION.md for the plan and tracking checklist.

## API Endpoints

### GET /

Returns the Dashboard HTML page.

### GET /api/events/stream

Server-Sent Events (SSE) endpoint pushing real-time events.

Example payload:

```json
{
  "timestamp": "2025-11-13T10:30:00Z",
  "event_type": "event_published",
  "event_id": "event-123",
  "topic": "agent.task",
  "sender": "planner",
  "thread_id": "thread-456",
  "correlation_id": "corr-789",
  "payload_preview": "Task 1 payload..."
}
```

### GET /api/topology

Returns a snapshot of the current agent topology.

Example payload:

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
    { "from_topic": "agent.task", "to_agent": "planner", "event_count": 0 }
  ],
  "timestamp": "2025-11-13T10:30:00Z"
}
```

### GET /api/flow

Returns the current event flow graph for visualization.

Example payload:

```json
{
  "nodes": [
    {
      "id": "EventBus",
      "node_type": "eventbus",
      "event_count": 42,
      "topics": ["agent.task"]
    },
    {
      "id": "planner",
      "node_type": "agent",
      "event_count": 15,
      "topics": ["agent.task"]
    }
  ],
  "flows": [
    {
      "source": "EventBus",
      "target": "planner",
      "topic": "agent.task",
      "count": 15,
      "last_event_ms": 1699876543210
    }
  ],
  "timestamp": "2025-11-13T10:30:00Z"
}
```

### GET /api/metrics

Returns a snapshot of key metrics (MVP).

Example payload:

```json
{
  "events_per_sec": 0,
  "active_agents": 3,
  "active_subscriptions": 0,
  "tool_invocations_per_sec": 0
}
```

## Environment Variables

| Variable            | Default   | Description                 |
| ------------------- | --------- | --------------------------- |
| LOOM_DASHBOARD      | false     | Whether to enable Dashboard |
| LOOM_DASHBOARD_PORT | 3030      | Dashboard HTTP port         |
| LOOM_DASHBOARD_HOST | 127.0.0.1 | Dashboard bind address      |

## Integrate in your app

```rust
use loom_core::{
    dashboard::{DashboardConfig, DashboardServer, EventBroadcaster, FlowTracker},
    event::EventBus,
    directory::AgentDirectory,
};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut event_bus = EventBus::new().await?;
    let agent_directory = Arc::new(AgentDirectory::new());

    // Enable Dashboard
    let broadcaster = EventBroadcaster::new(1000);
    let flow_tracker = Arc::new(FlowTracker::new());
    event_bus.set_dashboard_broadcaster(broadcaster.clone());

    // Start server
    let config = DashboardConfig::from_env();
    let dashboard = DashboardServer::new(config, broadcaster, agent_directory)
        .with_flow_tracker(flow_tracker.clone());

    tokio::spawn(async move {
        dashboard.serve().await.unwrap();
    });

    // Optionally record flows where appropriate
    // flow_tracker.record_flow("EventBus", "planner", "agent.task").await;

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
                      │  SSE
                      ▼
              ┌───────────────┐       ┌──────────────┐
              │ Dashboard     │◄──────│ FlowTracker  │
              │   (Axum)      │       │              │
              └────────┬───────┘       └──────────────┘
                       │ HTTP
                       ▼
                  ┌─────────┐
                  │ Browser │  D3.js
                  └─────────┘
```

## Event Flow Visualization

Node types:

- Agent (blue) — user-defined processors
- EventBus (purple) — central bus
- Router (orange) — routing decisions
- LLM (green) — language models
- Tool (red) — tool invocations
- Storage (cyan) — persistence

Behavior:

- Gray links = recent flows (≤ 30s)
- Blue animated links = active flows (≤ 2s)
- Badges = total events processed per node
- Drag nodes to rearrange; force layout will stabilize

## Performance

- Event buffer (SSE): 1000 (configurable)
- Stream view: last 100 events kept
- Flow retention: 60s (auto-cleanup)
- Update cadence: flow 2s, topology 5s, metrics 1s

## Next Steps

- Integrate FlowTracker with EventBus for automatic flow tracking
- Click-to-details for nodes/links; event search
- Thread timeline view; Prometheus metrics; export tools

# Dashboard - Real-time Event Flow Visualization## Dashboard MVP

Interactive dashboard for monitoring and visualizing event flow in the Loom system. Features real-time animated graph showing how events flow between agents, components, and the EventBus.A simple real-time event-stream visualization UI for viewing event flow within the Loom system.

## Quick Start

### 1. Start the Dashboard demo

`bash

cd core

export LOOM_DASHBOARD_PORT=3030

cargo run --example dashboard_demo

````

### 2. Open your browser

```bash

open http://localhost:3030

````

You will see:

- **Event Flow Graph**: Interactive D3.js visualization showing real-time event flow between components- Real-time event stream: events published to the EventBus

- **Event Stream**: Chronological list of all events published to the EventBus- Agent topology: list of registered Agents

- **Agent Topology**: List of registered agents and their subscriptions- Key metrics: event rate, number of active Agents

- **Key Metrics**: Event rate, active agent count

## Features

### ✅ Implemented

- Real-time event stream (SSE)

- **Event Flow Visualization (NEW!)**

  - Displays events in chronological order

  - Interactive force-directed graph using D3.js - Shows: timestamp, event_id, topic, sender, thread_id, correlation_id, payload

  - Animated links showing active event flow - Filter by thread_id / topic / sender

  - Color-coded nodes by type (Agent, EventBus, Router, LLM, Tool, Storage) - Pause / resume automatic scrolling

  - Event count badges on each node - Keeps the most recent 100 events

  - Draggable nodes for custom layout

  - Auto-updates every 2 seconds- Agent topology

  - Shows recent flows (last 30 seconds)

  - Displays the list of registered Agents

- **Real-time Event Stream (SSE)** - Shows subscribed topics

  - Auto-refresh (every 5 seconds)

  - Chronological event display

  - Shows: timestamp, event_id, topic, sender, thread_id, correlation_id, payload- Key metrics

  - Filter by thread_id / topic / sender

  - Pause / resume auto-scrolling - Events/sec

  - Retains last 100 events - Active Agents

- **Agent Topology**- Zero-dependency frontend

  - Pure HTML/CSS/JS (no build step)

  - Displays registered agents - Responsive design

  - Shows subscribed topics - Dark theme

  - Auto-refresh (every 5 seconds)

### 🚧 To be implemented

- **Key Metrics**

- Advanced visualizations

  - Events/sec

  - Active Agents - D3.js topology (force-directed graph)

  - Thread timeline (Gantt chart)

- **Zero-dependency Frontend** - Event relationship visualization

  - Pure HTML/CSS/JS + D3.js (from CDN)

  - No build step required- More metrics

  - Responsive design

  - Dark theme - Tool invocations/sec

  - P99 latency

### 🚧 To Be Implemented - Read real-time metrics from Prometheus

- **Advanced Visualizations**- Interactive features

  - Click an event to view details

  - Thread timeline (Gantt chart) - Event search

  - Event correlation visualization - Export event log as JSON

  - Tool invocation tracking

## API Endpoints

- **More Metrics**

### `GET /`

- Tool invocations/sec

- P99 latencyReturns the Dashboard HTML page

- Read real-time metrics from Prometheus

### `GET /api/events/stream`

- **Interactive Features**

  - Click event/node for detailsServer-Sent Events (SSE) endpoint that pushes real-time events

  - Event search

  - Export event log as JSONResponse format:

  - Filter flows by topic

````json

## API Endpoints{

  "timestamp": "2025-11-12T10:30:00Z",

### `GET /`  "event_type": "event_published",

  "event_id": "event-123",

Returns the Dashboard HTML page  "topic": "agent.task",

  "sender": "planner",

### `GET /api/events/stream`  "thread_id": "thread-456",

  "correlation_id": "corr-789",

**Server-Sent Events (SSE)** endpoint that pushes real-time events  "payload_preview": "Task 1 payload..."

}

Response format:```



```json### `GET /api/topology`

{

  "timestamp": "2025-11-13T10:30:00Z",Returns a snapshot of the current Agent topology

  "event_type": "event_published",

  "event_id": "event-123",Response format:

  "topic": "agent.task",

  "sender": "planner",```json

  "thread_id": "thread-456",{

  "correlation_id": "corr-789",  "agents": [

  "payload_preview": "Task 1 payload..."    {

}      "id": "planner",

```      "topics": ["agent.task"],

      "capabilities": ["plan.create"]

### `GET /api/topology`    }

  ],

Returns a snapshot of the current agent topology  "edges": [

    {

Response format:      "from_topic": "agent.task",

      "to_agent": "planner",

```json      "event_count": 0

{    }

  "agents": [  ],

    {  "timestamp": "2025-11-12T10:30:00Z"

      "id": "planner",}

      "topics": ["agent.task"],```

      "capabilities": ["plan.create"]

    }### `GET /api/metrics`

  ],

  "edges": [Returns a snapshot of key metrics

    {

      "from_topic": "agent.task",Response format:

      "to_agent": "planner",

      "event_count": 0```json

    }{

  ],  "events_per_sec": 0,

  "timestamp": "2025-11-13T10:30:00Z"  "active_agents": 3,

}  "active_subscriptions": 0,

```  "tool_invocations_per_sec": 0

}

### `GET /api/flow` (NEW!)```



Returns current event flow graph## Environment Variables



Response format:| Variable                | Default  | Description                         |

| ----------------------- | -------- | ----------------------------------- |

```json| `LOOM_DASHBOARD`        | `false`  | Whether to enable the Dashboard     |

{| `LOOM_DASHBOARD_PORT`   | `3030`   | Dashboard HTTP port                 |

  "nodes": [| `LOOM_DASHBOARD_HOST`   | `127.0.0.1` | Dashboard bind address           |

    {

      "id": "EventBus",## Integrating into your application

      "node_type": "eventbus",

      "event_count": 42,```rust

      "topics": ["agent.task", "agent.research"]use loom_core::{

    },    dashboard::{DashboardConfig, DashboardServer, EventBroadcaster},

    {    event::EventBus,

      "id": "planner",    directory::AgentDirectory,

      "node_type": "agent",};

      "event_count": 15,

      "topics": ["agent.task"]#[tokio::main]

    }async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {

  ],    // Create core components

  "flows": [    let mut event_bus = EventBus::new().await?;

    {    let agent_directory = Arc::new(AgentDirectory::new());

      "source": "EventBus",

      "target": "planner",    // Enable Dashboard

      "topic": "agent.task",    let broadcaster = EventBroadcaster::new(1000);

      "count": 15,    event_bus.set_dashboard_broadcaster(broadcaster.clone());

      "last_event_ms": 1699876543210

    }    // Start Dashboard server

  ],    let config = DashboardConfig::from_env();

  "timestamp": "2025-11-13T10:30:00Z"    let dashboard = DashboardServer::new(config, broadcaster, agent_directory);

}

```    tokio::spawn(async move {

        dashboard.serve().await.unwrap();

### `GET /api/metrics`    });



Returns a snapshot of key metrics    // ... your application code ...



Response format:    Ok(())

}

```json```

{

  "events_per_sec": 0,## Architecture

  "active_agents": 3,

  "active_subscriptions": 0,```

  "tool_invocations_per_sec": 0┌─────────────┐

}│  EventBus   │

```│             │

│  publish()  ├──────┐

## Environment Variables└─────────────┘      │

                     │ broadcast

| Variable                | Default     | Description                        |                     ▼

| ----------------------- | ----------- | ---------------------------------- |             ┌───────────────────┐

| `LOOM_DASHBOARD`        | `false`     | Whether to enable the Dashboard    |             │ EventBroadcaster  │

| `LOOM_DASHBOARD_PORT`   | `3030`      | Dashboard HTTP port                |             │  (tokio channel)  │

| `LOOM_DASHBOARD_HOST`   | `127.0.0.1` | Dashboard bind address             |             └────────┬──────────┘

                      │

## Integrating into your application                      │ SSE

                      ▼

```rust              ┌───────────────┐

use loom_core::{              │ DashboardServer│

    dashboard::{DashboardConfig, DashboardServer, EventBroadcaster, FlowTracker},              │   (Axum)       │

    event::EventBus,              └────────┬───────┘

    directory::AgentDirectory,                       │

};                       │ HTTP

use std::sync::Arc;                       ▼

                  ┌─────────┐

#[tokio::main]                  │ Browser │

async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {                  │   UI    │

    // Create core components                  └─────────┘

    let mut event_bus = EventBus::new().await?;```

    let agent_directory = Arc::new(AgentDirectory::new());

## Performance

    // Enable Dashboard

    let broadcaster = EventBroadcaster::new(1000);- Event buffer: 1000 events (configurable)

    let flow_tracker = Arc::new(FlowTracker::new());- Frontend limit: displays the most recent 100 events

    - Update frequencies:

    event_bus.set_dashboard_broadcaster(broadcaster.clone());  - Event stream: real-time (SSE push)

  - Topology: every 5 seconds

    // Start Dashboard server  - Metrics: every 1 second

    let config = DashboardConfig::from_env();

    let dashboard = DashboardServer::new(config, broadcaster, agent_directory)## Next steps

        .with_flow_tracker(flow_tracker.clone());

- [ ] Update ROADMAP

    tokio::spawn(async move {- [ ] Test integration with trio.py

        dashboard.serve().await.unwrap();- [ ] Add D3.js topology visualization

    });- [ ] Integrate Prometheus metrics

- [ ] Add Thread timeline view

    // Record event flows manually when needed

    // flow_tracker.record_flow("source", "target", "topic").await;Completion summary: Translation of README.md is done. Next steps I can take on request: (1) commit the translated README into the repo, (2) produce a side-by-side diff, or (3) refine wording for a specific audience (developer vs. product). Which would you prefer?


    // ... your application code ...

    Ok(())
}
````

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
              ┌───────────────┐       ┌──────────────┐
              │ DashboardServer│◄──────│ FlowTracker  │
              │   (Axum)       │       │              │
              └────────┬───────┘       └──────────────┘
                       │
                       │ HTTP
                       ▼
                  ┌─────────┐
                  │ Browser │
                  │   UI    │
                  │  D3.js  │
                  └─────────┘
```

## Event Flow Visualization

The flow graph provides real-time visualization of event flow between components:

### Node Types

- **Agent** (Blue): User-defined agents that process events
- **EventBus** (Purple): Central message bus
- **Router** (Orange): Routing decisions
- **LLM** (Green): Language model interactions
- **Tool** (Red): Tool invocations
- **Storage** (Cyan): Data persistence

### Flow Animation

- **Gray links**: Historical flows (within last 30 seconds)
- **Blue animated links**: Active flows (within last 2 seconds)
- **Event count badges**: Total events processed by each node
- **Link thickness**: Visual indicator of flow activity

### Interaction

- **Drag nodes**: Rearrange the graph layout
- **Hover**: See node/link details (future feature)
- **Auto-layout**: Force-directed layout automatically positions nodes

## Performance

- **Event buffer**: 1000 events (configurable)
- **Frontend limit**: Displays last 100 events in stream view
- **Flow retention**: 60 seconds (old flows auto-cleaned)
- **Update frequencies**:
  - Event stream: Real-time (SSE push)
  - Flow graph: Every 2 seconds
  - Topology: Every 5 seconds
  - Metrics: Every 1 second

## Development Tips

### Recording Event Flows

To accurately reflect event flow in your application, call `FlowTracker::record_flow()` when events move between components:

```rust
// When EventBus delivers to agent
flow_tracker.record_flow("EventBus", "planner", "agent.task").await;

// When agent publishes back
flow_tracker.record_flow("planner", "EventBus", "agent.result").await;

// When router forwards to LLM
flow_tracker.record_flow("Router", "llm-provider", "llm.request").await;
```

### Custom Node Types

The FlowTracker automatically infers node types from node IDs:

- Contains "llm" or "LLM" → LLM node
- Contains "tool" → Tool node
- Contains "storage" → Storage node
- Exactly "EventBus" → EventBus node
- Exactly "Router" → Router node
- Everything else → Agent node

## Next Steps

- [ ] Integrate FlowTracker into EventBus for automatic flow tracking
- [ ] Add click handlers for node/event details
- [ ] Implement event search
- [ ] Add thread timeline visualization
- [ ] Integrate Prometheus metrics
- [ ] Add export functionality
