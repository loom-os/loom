# Dashboard API Documentation

## Overview

The Loom Dashboard backend provides REST and WebSocket APIs for monitoring and interacting with the Loom agent runtime.

Base URL: `http://localhost:3030` (configurable via `LOOM_DASHBOARD_PORT`)

---

## REST Endpoints

### Health Check

Check if the dashboard server is running.

**Endpoint:** `GET /health`

**Response:**

```json
{
  "status": "ok",
  "service": "loom-dashboard",
  "version": "0.1.0"
}
```

---

### Configuration

Get the current dashboard configuration.

**Endpoint:** `GET /api/config`

**Response:**

```json
{
  "host": "0.0.0.0",
  "port": 3030,
  "cors": {
    "enabled": true,
    "allowed_origins": ["http://localhost:5173"]
  },
  "frontend": {
    "serve_frontend": true
  }
}
```

---

### Available Agents

Get a list of all registered agents with their capabilities and status.

**Endpoint:** `GET /api/agents`

**Description:**
Returns all agents registered in the `AgentDirectory`. This allows clients to:

- Discover which cognitive agents are available for chat
- Monitor agent health and status
- Filter agents by capabilities or topics

**Response:**

```json
{
  "agents": [
    {
      "agent_id": "chat-assistant",
      "status": "active",
      "subscribed_topics": ["chat.input"],
      "capabilities": ["web_search", "file_write", "python_execute"],
      "metadata": {
        "type": "cognitive",
        "model": "deepseek-chat"
      },
      "last_heartbeat": 1734393600000
    },
    {
      "agent_id": "market-analyst",
      "status": "active",
      "subscribed_topics": ["market.input"],
      "capabilities": ["okx_api", "data_analysis"],
      "metadata": {
        "type": "cognitive"
      },
      "last_heartbeat": 1734393605000
    }
  ],
  "count": 2,
  "timestamp": "2024-12-17T00:00:00Z"
}
```

**Fields:**

| Field               | Type       | Description                                                     |
| ------------------- | ---------- | --------------------------------------------------------------- |
| `agent_id`          | `string`   | Unique identifier for the agent                                 |
| `status`            | `string`   | Current status: `active`, `idle`, `inactive`, or `disconnected` |
| `subscribed_topics` | `string[]` | EventBus topics the agent listens to                            |
| `capabilities`      | `string[]` | Tools/capabilities the agent provides                           |
| `metadata`          | `object`   | Additional agent information (key-value pairs)                  |
| `last_heartbeat`    | `number?`  | Timestamp of last heartbeat (milliseconds since epoch)          |

**Status Values:**

- `active`: Agent is connected and actively processing
- `idle`: Agent is connected but not currently processing
- `inactive`: Agent hasn't sent heartbeat recently (but still registered)
- `disconnected`: Agent has explicitly disconnected

**Identifying Cognitive Agents:**

To find agents suitable for chat interactions, look for:

1. Topics containing "input" or "chat" (e.g., `"chat.input"`, `"market.input"`)
2. Metadata with `"type": "cognitive"`

**Example Usage:**

```typescript
// Fetch all agents
const response = await fetch("http://localhost:3030/api/agents");
const data = await response.json();

// Filter cognitive agents
const cognitiveAgents = data.agents.filter(
  (agent) =>
    agent.subscribed_topics.some(
      (topic) => topic.includes("input") || topic.includes("chat")
    ) || agent.metadata.type === "cognitive"
);

// Find agent with specific capability
const searchableAgents = data.agents.filter((agent) =>
  agent.capabilities.includes("web_search")
);
```

---

## WebSocket API

### Connection

Establish a WebSocket connection for real-time bidirectional communication.

**Endpoint:** `ws://localhost:3030/ws`

**Message Protocol:**

All messages are JSON objects with a `type` field:

```typescript
type WsMessage =
  | {
      type: "chat_request";
      thread_id: string;
      content: string;
      agent_id?: string;
    }
  | {
      type: "chat_chunk";
      thread_id: string;
      content: string;
      content_type: string;
      sequence: number;
    }
  | {
      type: "chat_complete";
      thread_id: string;
      stats: { duration_ms: number; total_tokens: number };
    }
  | { type: "event_stream"; event_id: string; timestamp: string; topic: string }
  | { type: "error"; code: string; message: string };
```

See [WebSocket Protocol](./WEBSOCKET.md) for detailed documentation.

---

## Error Handling

All endpoints return appropriate HTTP status codes:

- `200 OK`: Successful request
- `400 Bad Request`: Invalid request parameters
- `404 Not Found`: Endpoint or resource not found
- `500 Internal Server Error`: Server error

Error responses include a message:

```json
{
  "error": "Description of what went wrong"
}
```

---

## Development

### Testing

Run backend tests:

```bash
cd loom-dashboard
cargo test --package loom-dashboard
```

### CORS Configuration

For local development, enable CORS:

```bash
export LOOM_DASHBOARD_CORS_ENABLED=true
export LOOM_DASHBOARD_CORS_ORIGINS=http://localhost:5173
```

---

## Integration Example

```typescript
// React component using the agents API
import { useAgents } from "@/hooks/useAgents";

function AgentSelector() {
  const { agents, loading, error, getCognitiveAgents } = useAgents({
    refreshInterval: 10000,
    filterStatus: "active",
  });

  if (loading) return <Spinner />;
  if (error) return <Error message={error.message} />;

  const cognitiveAgents = getCognitiveAgents();

  return (
    <select>
      {cognitiveAgents.map((agent) => (
        <option key={agent.agent_id} value={agent.agent_id}>
          {agent.agent_id} ({agent.capabilities.length} tools)
        </option>
      ))}
    </select>
  );
}
```

---

## Next Steps

- **Issue 5**: Implement chat routing through EventBus
- **Issue 6**: Add agent selector to frontend ChatPage
- **Issue 7**: Implement streaming chat with multiple agents
