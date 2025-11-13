# Dashboard Event Flow Visualization - Usage Guide

## Overview

The enhanced dashboard now includes an interactive event flow visualization that shows real-time event flow between agents and system components using D3.js force-directed graphs.

## What's New

### 1. Event Flow Graph Tab

- **Interactive Force-Directed Graph**: Visualize how events flow between components in real-time
- **Animated Links**: Active event flows (< 2 seconds old) are highlighted with blue animated lines
- **Color-Coded Nodes**: Each node type has a distinct color
- **Event Count Badges**: Shows total events processed by each node
- **Draggable Layout**: Click and drag nodes to customize the graph layout

### 2. Dual-Tab Interface

- **Event Flow Tab**: D3.js visualization (default view)
- **Event Stream Tab**: Traditional chronological event list

### 3. New API Endpoint

- `GET /api/flow`: Returns current flow graph data with nodes and edges

## Quick Test

```bash
# Terminal 1: Start the dashboard
cd core
export LOOM_DASHBOARD_PORT=3030
cargo run --example dashboard_demo

# Terminal 2: Open browser
open http://localhost:3030
```

## What You'll See

The dashboard_demo creates a continuous event flow simulation:

1. **Nodes appear** representing:

   - **EventBus** (purple center) - The message hub
   - **Agents** (blue) - planner, researcher, writer
   - **Router** (orange) - Routing decisions
   - **llm-provider** (green) - Language model interactions

2. **Links animate** showing:

   - EventBus → Agents (event delivery)
   - Agents → EventBus (agent publishing)
   - Router ↔ LLM (periodic interactions)

3. **Event counts** increment on node badges as events flow

4. **Auto-refresh** every 2 seconds to show latest flows

## Node Type Legend

| Color     | Type     | Description                   |
| --------- | -------- | ----------------------------- |
| 🔵 Blue   | Agent    | User-defined event processors |
| 🟣 Purple | EventBus | Central message bus           |
| 🟠 Orange | Router   | Routing decisions             |
| 🟢 Green  | LLM      | Language model interactions   |
| 🔴 Red    | Tool     | Tool invocations              |
| 🔵 Cyan   | Storage  | Data persistence              |

## Integration Example

To use FlowTracker in your own application:

```rust
use loom_core::dashboard::FlowTracker;
use std::sync::Arc;

let flow_tracker = Arc::new(FlowTracker::new());

// Record when EventBus delivers event to agent
flow_tracker.record_flow("EventBus", "my-agent", "my.topic").await;

// Record when agent processes and publishes result
flow_tracker.record_flow("my-agent", "EventBus", "result.topic").await;

// Record LLM interactions
flow_tracker.record_flow("Router", "openai-llm", "llm.request").await;
flow_tracker.record_flow("openai-llm", "Router", "llm.response").await;
```

## Performance Characteristics

- **Flow Retention**: 60 seconds (auto-cleanup of old flows)
- **Active Flow Threshold**: 2 seconds (animated links)
- **Display Threshold**: 30 seconds (visible flows)
- **Update Interval**: 2 seconds
- **Memory Impact**: Minimal (HashMap with time-based cleanup)

## Troubleshooting

### No flows showing?

- Ensure `FlowTracker::record_flow()` is being called in your code
- Check that demo is running: look for "Starting event flow simulation..." log

### Nodes overlapping?

- Drag nodes to separate them
- The force simulation will auto-arrange after a few seconds

### Performance issues?

- Reduce update interval if needed (currently 2s)
- Flows auto-cleanup after 60 seconds to prevent memory growth

## Next Enhancements

Planned features:

- Click on nodes to see details
- Filter flows by topic
- Show event payloads on hover
- Thread timeline view
- Export graph as image
- Integrate with actual EventBus for automatic flow tracking

## Architecture

```
FlowTracker (Rust)
    ↓
REST API (/api/flow)
    ↓
D3.js Force Simulation (Browser)
    ↓
SVG Visualization
```

The flow tracking is:

1. **Recorded** by calling `flow_tracker.record_flow()`
2. **Stored** in-memory with timestamps
3. **Cleaned** automatically (60s retention)
4. **Queried** via `/api/flow` endpoint
5. **Visualized** using D3.js force-directed graph

## Tips

- **Zoom**: Not yet implemented, but nodes are draggable
- **Best view**: Flow graph works best with 3-10 nodes
- **Performance**: Handles up to 50 nodes smoothly
- **Mobile**: Works on mobile but desktop experience is better

Enjoy exploring event flows! 🚀
