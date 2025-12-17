# useObservability Hook

React hook for consuming real-time event stream data from the Loom Dashboard backend.

## Features

- ✅ Real-time event streaming via WebSocket
- ✅ Automatic reconnection with exponential backoff
- ✅ Event aggregation and windowing
- ✅ Derived agent communications
- ✅ Real-time metrics calculation
- ✅ Type-safe event handling

## Installation

The hook is part of the Loom Dashboard frontend. No additional installation required.

## Usage

### Basic Usage

```typescript
import { useObservability } from "@/hooks/useObservability";

function ObservabilityPage() {
  const { events, communications, metrics, isConnected } = useObservability();

  if (!isConnected) {
    return <div>Connecting to event stream...</div>;
  }

  return (
    <div>
      <h1>Events: {events.length}</h1>
      <h1>Metrics: {metrics.eventsPerSecond} events/sec</h1>
    </div>
  );
}
```

### Custom WebSocket URL

```typescript
const { events } = useObservability("ws://custom-host:8080/ws");
```

## API Reference

### Return Value

```typescript
interface UseObservabilityReturn {
  // Real-time events from EventBus
  events: ObservabilityEvent[];

  // Derived agent communications (tool calls, messages, output)
  communications: Communication[];

  // Real-time metrics
  metrics: ObservabilityMetrics;

  // Connection status
  isConnected: boolean;
  isConnecting: boolean;
}
```

### ObservabilityEvent

```typescript
interface ObservabilityEvent {
  id: string; // Unique event ID
  type: string; // Event type (inferred from topic)
  topic: string; // EventBus topic
  sender: string; // Agent ID
  threadId?: string; // Conversation/thread ID
  timestamp: number; // Unix timestamp (ms)
  qos: "Realtime" | "Batched" | "Background"; // QoS level
  payloadPreview: string; // Payload preview
}
```

### Communication

```typescript
interface Communication {
  id: string;
  timestamp: number;
  agent: string;
  type: "tool_call" | "output" | "message";
  target?: string; // For inter-agent messages
  content: string;
  tool?: string; // For tool_call type
  result?: string; // Tool call result
  threadId?: string;
}
```

### ObservabilityMetrics

```typescript
interface ObservabilityMetrics {
  eventsPerSecond: number; // Events/sec in last 10 seconds
  activeAgents: number; // Unique agents seen
  routingDecisions: number; // stream.request events
  averageLatency: number; // Estimated latency (ms)
  qosBreakdown: {
    realtime: number; // Percentage
    batched: number; // Percentage
    background: number; // Percentage
  };
}
```

## Event Types

Events are automatically categorized based on their topic:

| Topic Pattern    | Inferred Type    |
| ---------------- | ---------------- |
| `stream.request` | `user.question`  |
| `stream.chunk`   | `llm.generate`   |
| `agent.*`        | `agent.activity` |
| `tool.*`         | `tool.execute`   |
| Others           | Topic name       |

## Communication Derivation

The hook automatically converts events into communications:

### Tool Calls

Events with `tool.*` topics become tool call communications:

```typescript
{
  type: 'tool_call',
  tool: 'web_search',  // From topic: tool.web_search
  content: 'Tool execution',
  result: 'Search completed'
}
```

### Agent Output

Events from streaming become output communications:

```typescript
{
  type: 'output',
  content: 'Generated response'
}
```

### Inter-Agent Messages

Events with thread IDs become messages:

```typescript
{
  type: 'message',
  content: 'Inter-agent message',
  threadId: 'thread-123'
}
```

## Metrics Calculation

### Events Per Second

Calculated over a 10-second sliding window:

```typescript
eventsPerSecond = eventCount / 10;
```

### Active Agents

Unique agent IDs seen in recent events:

```typescript
activeAgents = new Set(events.map((e) => e.sender)).size;
```

### QoS Breakdown

Percentage distribution of QoS levels:

```typescript
qosBreakdown.realtime = (realtimeCount / totalEvents) * 100;
```

## Window Management

The hook maintains sliding windows for efficiency:

| Data Type      | Window Size | Purpose                  |
| -------------- | ----------- | ------------------------ |
| Events         | 50 items    | Display in UI            |
| Communications | 30 items    | Agent communication view |
| Metrics        | 10 seconds  | Performance calculation  |

Old items are automatically removed as new ones arrive.

## Connection Management

The hook uses `useWebSocket` internally with:

- **Automatic reconnection**: Exponential backoff on disconnect
- **Max attempts**: 5 reconnection attempts
- **Initial delay**: 1 second
- **Max delay**: 30 seconds

## Performance

### Memory Usage

- ~50 events × ~500 bytes = ~25 KB
- ~30 communications × ~300 bytes = ~9 KB
- Total: ~35 KB + UI rendering overhead

### Update Frequency

- **Event updates**: Immediate (as events arrive)
- **Metrics updates**: Every 1 second

### Network Traffic

- **Event rate**: Depends on system activity
- **Typical**: 5-20 events/sec during active use
- **Payload**: ~200 bytes per event (configurable on backend)

## Examples

### Display Connection Status

```typescript
function ConnectionStatus() {
  const { isConnected, isConnecting } = useObservability();

  if (isConnecting) {
    return <Badge variant="outline">Connecting...</Badge>;
  }

  return (
    <Badge variant={isConnected ? "success" : "error"}>
      {isConnected ? "● Live" : "● Disconnected"}
    </Badge>
  );
}
```

### Filter Events by Agent

```typescript
function AgentEvents({ agentId }: { agentId: string }) {
  const { events } = useObservability();

  const agentEvents = events.filter((e) => e.sender === agentId);

  return (
    <ul>
      {agentEvents.map((event) => (
        <li key={event.id}>
          {event.type}: {event.payloadPreview}
        </li>
      ))}
    </ul>
  );
}
```

### Show Real-time Metrics

```typescript
function MetricsDashboard() {
  const { metrics } = useObservability();

  return (
    <div>
      <div>Events/sec: {metrics.eventsPerSecond.toFixed(2)}</div>
      <div>Active Agents: {metrics.activeAgents}</div>
      <div>Avg Latency: {metrics.averageLatency}ms</div>
      <div>
        QoS: R{metrics.qosBreakdown.realtime}% / B{metrics.qosBreakdown.batched}
        % / BG{metrics.qosBreakdown.background}%
      </div>
    </div>
  );
}
```

### Export Events

```typescript
function ExportButton() {
  const { events } = useObservability();

  const handleExport = () => {
    const json = JSON.stringify(events, null, 2);
    const blob = new Blob([json], { type: "application/json" });
    const url = URL.createObjectURL(blob);

    const a = document.createElement("a");
    a.href = url;
    a.download = `events-${Date.now()}.json`;
    a.click();
  };

  return <button onClick={handleExport}>Export Events</button>;
}
```

## Troubleshooting

### No Events Appearing

1. Check connection status: `isConnected` should be `true`
2. Verify backend is running on `ws://localhost:3030/ws`
3. Check that events are being published to subscribed topics
4. Look at browser console for WebSocket errors

### Events Not Updating

1. Verify `useObservability` is called inside a React component
2. Check that component is mounted
3. Ensure WebSocket connection is stable

### High Memory Usage

1. Reduce window sizes (edit `useObservability.ts`)
2. Filter events before storing
3. Clear old data periodically

## Related Hooks

- [`useWebSocket`](./useWebSocket.ts) - Low-level WebSocket hook
- [`useChat`](./useChat.ts) - Chat-specific hook
- [`useAgents`](./useAgents.ts) - Agent topology hook

## Backend Configuration

See [Backend Observability Events Documentation](../../backend/docs/OBSERVABILITY_EVENTS.md) for:

- Configuring observable topics
- Payload visibility settings
- Security considerations
- Performance tuning
