# Loom Dashboard

Real-time observability for Loom Core. The dashboard presents a live event stream, an animated flow map, and quick agent insights so you can see how threads move through your orchestration stack without leaving the terminal.

---

## TL;DR

- **Build once, serve everywhere** – run `npm run build` in `src/dashboard/frontend/` to emit assets the server embeds automatically.
- **React + Tailwind UI** – shadcn component kit, animated flow graph, responsive layout for day-long monitoring.
- **Flow-aware storage** – in-memory graph with automatic TTL and bounded topic lists to avoid runaway memory usage.
- **Safe SSE handling** – resilient reconnect logic that keeps a single EventSource per browser tab.

---

## Quick Start

```bash
cd core/src/dashboard/frontend
npm install
npm run build           # outputs hashed assets into ../static

cd ../../..
export LOOM_DASHBOARD_PORT=3030
cargo run --example dashboard_demo
# open http://127.0.0.1:3030
```

What you will see:

- **Event Flow** - D3 force-directed map of recent routes (rolling 60 s window).
- **Event Stream** - live log with per-agent quick filters, auto-scroll toggle, and export-friendly payload previews.
- **Agent Roster** - active agents, their topics, and capability hints.
- **Agent Timeline** - select any agent in the stream to reveal the full inbound/outbound history (no arbitrary 5-item cap).
- **Metrics** - request-rate placeholders ready for OpenTelemetry backends.

---

## Architecture Snapshot

| Layer                            | Purpose                                                                                               |
| -------------------------------- | ----------------------------------------------------------------------------------------------------- |
| `EventBus` -> `EventBroadcaster` | Publishes trimmed event payloads over SSE.                                                            |
| `FlowTracker`                    | Records `(source, target, topic)` edges, prunes after 60 s, caps each node to 20 topics.              |
| `TopologyBuilder`                | Reads `AgentDirectory` to surface live roster data.                                                   |
| Frontend (`static/`)             | Vite-built React bundle (shadcn UI) consuming `/api/*`, canvas flow animations, resilient SSE client. |

Event data stays on the server; the browser only renders JSON delivered via `/api/events/stream`, `/api/flow`, `/api/topology`, and `/api/metrics`.

---

## Integrate Into Your Service

```rust
let mut event_bus = EventBus::new().await?;
let directory = Arc::new(AgentDirectory::new());

let broadcaster = EventBroadcaster::new(1000);
let flow_tracker = Arc::new(FlowTracker::new());
event_bus.set_dashboard_broadcaster(broadcaster.clone());

let config = DashboardConfig::from_env();
let server = DashboardServer::new(config, broadcaster, directory.clone())
    .with_flow_tracker(flow_tracker.clone());

tokio::spawn(async move {
    server.serve().await.expect("dashboard");
});
```

Call `flow_tracker.record_flow("source", "target", "topic").await` whenever you forward work between agents, tools, or routers. The tracker keeps only the most recent minute of traffic and will let go of idle nodes automatically.

---

## Configuration

| Environment Variable  | Default     | Description                         |
| --------------------- | ----------- | ----------------------------------- |
| `LOOM_DASHBOARD`      | `false`     | Enable/disable dashboard bootstrap. |
| `LOOM_DASHBOARD_HOST` | `127.0.0.1` | Bind address for the HTTP server.   |
| `LOOM_DASHBOARD_PORT` | `3030`      | Listening port.                     |

Set `LOOM_DASHBOARD=true` in production or guard the server behind your own auth middleware.

---

## Operations & Observability Tips

- **SSE clients** - the React app keeps exactly one `EventSource`, closing it before reconnects. If you see multiple `events/stream` requests in DevTools, treat that as a bug.
- **Flow retention** - edges expire after 60 s, nodes after 120 s of inactivity. The browser graph polls every 2.5 s; adjust in `static/index.html` if you need longer windows.
- **Topic list bounds** - each node keeps the 20 most recent topics, guaranteeing predictable JSON payloads even when topics are dynamic (thread-scoped IDs, etc.).
- **Metrics endpoint** - returns placeholders today. Swap in your own struct or hook into OpenTelemetry exporters when ready.

---

## Extending the UI

The dashboard ships as a Vite-powered React application:

- Develop inside `core/src/dashboard/frontend` with `npm run dev`.
- Fetch additional APIs (e.g., `/api/metrics/detailed`) via React Query hooks inside `src/lib/dashboardApi.ts`.
- Customize the flow canvas or cards by editing the components under `src/components/`.
- Run `npm run build` to regenerate static assets; the Rust server embeds whatever lives in `core/src/dashboard/static/`.

---

## Roadmap Ideas

- Live latency charts (P95 / P99) once telemetry endpoints are plumbed.
- Flow playback mode to scrub through historical windows.
- Auth guard integration and configurable retention budgets.

Have a need not captured here? Open an issue or leave a note alongside your service integration.
