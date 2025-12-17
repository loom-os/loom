# Observability Event Streaming

This document describes the real-time event streaming system for the Loom Dashboard's Observability page.

## Overview

The observability event streaming system provides real-time visibility into events flowing through the Loom EventBus. Events are automatically forwarded to connected WebSocket clients for live monitoring and debugging.

## Architecture

```
EventBus                EventBusBridge              WebSocket              Frontend
   │                          │                         │                      │
   ├─ publish("agent.*") ────▶│                         │                      │
   ├─ publish("stream.*") ────▶│                         │                      │
   ├─ publish("dashboard") ───▶│                         │                      │
   │                          │                         │                      │
   │                          ├─ subscribe to topics    │                      │
   │                          │                         │                      │
   │                          ├─ convert Event ────────▶│                      │
   │                          │   to WsMessage          │                      │
   │                          │                         │                      │
   │                          │                         ├─ event_stream ──────▶│
   │                          │                         ├─ event_stream ──────▶│
   │                          │                         ├─ event_stream ──────▶│
   │                          │                         │                      │
   │                          │                         │                   Display
   │                          │                         │                   in UI
```

## Configuration

### Backend Configuration

Observable topics are configured via `ObservabilityConfig`:

```rust
use loom_dashboard::config::ObservabilityConfig;

let config = ObservabilityConfig {
    // Topics to subscribe to (supports wildcards)
    topics: vec![
        "dashboard".to_string(),
        "agent.*".to_string(),      // All agent events
        "stream.*".to_string(),     // All streaming events
        "tool.*".to_string(),       // All tool events
    ],

    // Whether to include payload content
    include_payload: true,

    // Maximum payload preview length (bytes)
    max_payload_preview: 200,
};
```

### Environment Variables

Configure topics via environment variable:

```bash
# Comma-separated list of topics
export LOOM_DASHBOARD_OBSERVABLE_TOPICS="dashboard,agent.*,stream.*,market.price.*"

# Start the dashboard
cargo run --bin loom-dashboard
```

### Default Topics

By default, the dashboard subscribes to:

- `dashboard` - Dashboard-specific events
- `agent.*` - All agent activity (wildcard)
- `stream.*` - All streaming events (wildcard)

## Message Format

### EventStream Message

When an event flows through the EventBus, it's converted to a WebSocket message:

```json
{
  "type": "event_stream",
  "event_id": "evt_abc123",
  "timestamp": "2024-01-15T10:30:00Z",
  "topic": "agent.planner",
  "sender": "planner-agent-1",
  "thread_id": "thread-456",
  "payload_preview": "Task breakdown complete... (1024 bytes)"
}
```

### Fields

| Field             | Type    | Description                       |
| ----------------- | ------- | --------------------------------- |
| `type`            | string  | Always `"event_stream"`           |
| `event_id`        | string  | Unique event identifier           |
| `timestamp`       | string  | ISO 8601 timestamp                |
| `topic`           | string  | Event topic from EventBus         |
| `sender`          | string? | Agent ID that published the event |
| `thread_id`       | string? | Thread/conversation ID if present |
| `payload_preview` | string  | Preview of event payload          |

## Frontend Integration

### Using the Hook

The `useObservability` hook subscribes to the event stream:

```typescript
import { useObservability } from "@/hooks/useObservability";

function ObservabilityPage() {
  const {
    events, // Array of recent events
    communications, // Derived agent communications
    metrics, // Real-time metrics
    isConnected, // Connection status
    isConnecting, // Connection state
  } = useObservability();

  return (
    <div>
      <EventFlowVisualization events={events} />
      <AgentCommunication communications={communications} />
      <MetricsOverview metrics={metrics} />
    </div>
  );
}
```

### Connection States

The hook manages WebSocket connection automatically:

| State                                   | Description                    |
| --------------------------------------- | ------------------------------ |
| `isConnecting=true`                     | Attempting to connect          |
| `isConnected=true`                      | Connected and receiving events |
| `isConnected=false, isConnecting=false` | Disconnected                   |

The hook will automatically reconnect with exponential backoff if the connection is lost.

## Topic Patterns

### Exact Match

Subscribe to a specific topic:

```rust
topics: vec!["dashboard".to_string()]
```

### Wildcard Pattern

Subscribe to all topics matching a prefix:

```rust
topics: vec![
    "agent.*".to_string(),      // agent.planner, agent.researcher, etc.
    "market.price.*".to_string() // market.price.BTC, market.price.ETH, etc.
]
```

### Subscribe to Everything

Subscribe to all events (use with caution in production):

```rust
topics: vec!["*".to_string()]
```

## Performance Considerations

### QoS and Backpressure

Events are subscribed with `QoSLevel::Realtime`:

- **Low latency**: Events delivered immediately
- **Drop on pressure**: If WebSocket clients can't keep up, events are dropped
- **No queuing**: Prevents memory buildup

### Event Window

The frontend maintains a sliding window:

- **Events**: Last 50 events
- **Communications**: Last 30 communications
- **Metrics window**: Last 10 seconds

### Payload Configuration

Control payload visibility:

```rust
config.include_payload = false;  // Only show byte count
config.max_payload_preview = 100; // Limit preview length
```

## Security Considerations

### Sensitive Data

When dealing with sensitive information:

1. **Disable payload preview**:

   ```rust
   config.include_payload = false;
   ```

2. **Filter sensitive topics**:

   ```rust
   // Don't subscribe to topics with sensitive data
   topics: vec!["dashboard".to_string()]
   ```

3. **Implement access control** (future):
   - Per-user topic subscriptions
   - Role-based event filtering

## Testing

### Integration Tests

Test event streaming end-to-end:

```rust
#[tokio::test]
async fn test_event_bridge_forwards_to_websocket() {
    let event_bus = Arc::new(EventBus::new().await.unwrap());
    let connection_manager = Arc::new(ConnectionManager::new());
    let config = ObservabilityConfig::default();

    let bridge = Arc::new(EventBusBridge::new(
        event_bus.clone(),
        connection_manager.clone(),
        config,
    ));
    bridge.clone().start().await.unwrap();

    // Add WebSocket connection
    let (tx, mut rx) = mpsc::unbounded_channel();
    connection_manager.add("test-conn".to_string(), tx);

    // Publish event
    event_bus.publish("test.topic", event).await.unwrap();

    // Verify received
    let message = rx.recv().await.unwrap();
    assert!(matches!(message, WsMessage::EventStream { .. }));
}
```

Run tests:

```bash
cd loom-dashboard/backend
cargo test event_streaming_test
```

## Troubleshooting

### No Events Appearing

1. **Check connection status**: Ensure WebSocket is connected
2. **Verify topics**: Events must match subscribed topics
3. **Check EventBus**: Ensure events are being published
4. **Check logs**: Look for bridge subscription confirmations

```bash
# Enable debug logs
RUST_LOG=loom_dashboard=debug cargo run
```

### High Memory Usage

1. **Reduce event window**: Lower `WINDOW_SIZE` in frontend
2. **Disable payload**: Set `include_payload = false`
3. **Filter topics**: Subscribe to fewer topics

### Events Not Matching

Remember wildcard patterns:

- `agent.*` matches `agent.planner`, not `agent`
- Pattern must be exact prefix followed by `.`

## Future Enhancements

Planned improvements:

- [ ] Per-connection topic subscriptions (client-side filtering)
- [ ] Event persistence for replay
- [ ] Advanced filtering (event type, sender, etc.)
- [ ] Event search and export
- [ ] Metrics aggregation and historical data

## Examples

### Monitor Specific Agent

```rust
// Backend: Subscribe only to one agent
config.topics = vec!["agent.planner".to_string()];
```

### Monitor All Market Events

```rust
// Backend: All market-related events
config.topics = vec!["market.*".to_string()];
```

### Debug Streaming Issues

```rust
// Backend: Monitor stream lifecycle
config.topics = vec![
    "stream.request".to_string(),
    "stream.chunk".to_string(),
    "stream.complete".to_string(),
    "stream.error".to_string(),
];
```

## Related Documentation

- [WebSocket Protocol](./CONTENT_TYPE_GUIDE.md)
- [Chat Routing](./CHAT_ROUTING.md)
- [Agent Discovery](./AGENT_DISCOVERY.md)
- [EventBus Documentation](../../core/src/messaging/README.md)
