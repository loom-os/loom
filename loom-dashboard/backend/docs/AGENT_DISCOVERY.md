# Agent Discovery - Usage Examples

## Overview

Issue 4 implements agent discovery functionality, allowing the Dashboard to list all registered agents and their capabilities. This is the foundation for multi-agent chat selection.

## Backend API

### Endpoint

```
GET /api/agents
```

### Response Format

```json
{
  "agents": [
    {
      "agent_id": "chat-assistant",
      "status": "active",
      "subscribed_topics": ["chat.input"],
      "capabilities": ["web_search", "file_write"],
      "metadata": {
        "type": "cognitive",
        "model": "deepseek-chat"
      },
      "last_heartbeat": 1734393600000
    }
  ],
  "count": 1,
  "timestamp": "2024-12-17T00:00:00Z"
}
```

## Frontend Hook

### Basic Usage

```tsx
import { useAgents } from "@/hooks/useAgents";

function MyComponent() {
  const { agents, loading, error } = useAgents();

  if (loading) return <div>Loading agents...</div>;
  if (error) return <div>Error: {error.message}</div>;

  return (
    <ul>
      {agents.map((agent) => (
        <li key={agent.agent_id}>
          {agent.agent_id} - {agent.status}
        </li>
      ))}
    </ul>
  );
}
```

### With Auto-refresh

```tsx
const { agents, refresh } = useAgents({
  refreshInterval: 5000, // Refresh every 5 seconds
  filterStatus: "active", // Only show active agents
});
```

### Filter Cognitive Agents

```tsx
const { agents, getCognitiveAgents } = useAgents();

const cognitiveAgents = getCognitiveAgents();
// Returns agents with "input" or "chat" topics, or metadata.type === "cognitive"
```

### Filter by Capability

```tsx
const { filterByCapability } = useAgents();

const webSearchAgents = filterByCapability("web_search");
```

### Custom Topic Filtering

```tsx
const { agents } = useAgents({
  filterTopicPattern: "input", // Only agents with topics containing "input"
});
```

## Testing

### Backend Tests

Run backend tests:

```bash
cd loom-dashboard
cargo test --package loom-dashboard --lib server::tests
```

Key test cases:

- ✅ Empty agent list
- ✅ Multiple agents with different statuses
- ✅ Agent metadata and capabilities serialization

### Manual Testing

1. Start the dashboard server:

```bash
cd loom-dashboard
cargo run --example basic_server
```

2. In another terminal, test the API:

```bash
curl http://localhost:3030/api/agents | jq
```

3. Register a test agent using loom-py:

```bash
cd apps/chat-assistant
loom run
```

4. Verify the agent appears in the API:

```bash
curl http://localhost:3030/api/agents | jq '.agents[] | {agent_id, status}'
```

## Integration with Chat (Coming in Issue 6)

The agent discovery API enables the frontend to:

1. **Fetch available agents** on page load
2. **Display agent selector** dropdown/cards
3. **Pass selected agent_id** to chat requests
4. **Monitor agent availability** in real-time

Example flow:

```tsx
function ChatPage() {
  const { getCognitiveAgents } = useAgents();
  const [selectedAgent, setSelectedAgent] = useState<string | null>(null);

  const cognitiveAgents = getCognitiveAgents();

  return (
    <div>
      <AgentSelector
        agents={cognitiveAgents}
        selected={selectedAgent}
        onChange={setSelectedAgent}
      />
      <ChatInterface targetAgentId={selectedAgent} />
    </div>
  );
}
```

## Architecture Notes

### Why REST instead of WebSocket?

Agent discovery uses REST (`GET /api/agents`) instead of WebSocket because:

- **Simpler client code**: No need to maintain WebSocket connection state
- **Cacheable**: Can be cached by browsers/proxies
- **Stateless**: Each request is independent
- **Standard HTTP**: Works with any HTTP client

For real-time updates, we'll add WebSocket events in Issue 9 (Agent Status Monitoring).

### Agent Identification Heuristics

To identify "chattable" cognitive agents, we use:

1. **Topic pattern matching**: Agents subscribed to topics like:

   - `chat.input`
   - `*.input`
   - Any topic containing "input" or "chat"

2. **Metadata markers**: Agents with:

   - `metadata.type === "cognitive"`
   - `metadata.role === "assistant"`

3. **Capability hints**: Agents providing tools like:
   - `llm_*` (LLM-related capabilities)
   - `chat_*`

This heuristic approach allows flexibility for different agent architectures.

## Next Steps

**Issue 5: Chat Routing** will implement:

- Dashboard backend as EventBus relay
- Chat request forwarding to selected agent
- Stream response aggregation
- Thread management

**Issue 6: Agent Selector UI** will implement:

- Agent cards/dropdown in ChatPage
- Agent status indicators (online/offline)
- Capability badges
- Last active timestamp display

## Troubleshooting

### No agents appear in the list

**Cause**: No agents are registered in AgentDirectory.

**Solution**:

1. Ensure loom Bridge is running
2. Start at least one agent (e.g., `cd apps/chat-assistant && loom run`)
3. Check agent logs for registration confirmation

### Agent status is "disconnected"

**Cause**: Agent stopped sending heartbeats.

**Solution**:

1. Check if agent process is still running
2. Verify Bridge connectivity
3. Restart the agent

### Agent missing capabilities

**Cause**: Agent didn't register tools with Bridge.

**Solution**:

1. Check agent configuration (tools list in loom.toml)
2. Verify tool registration in agent startup logs
3. Use `loom chat` to test if tools work directly

## References

- [Backend API Documentation](./API.md)
- [WebSocket Protocol](./WEBSOCKET.md)
- [Dashboard Architecture](../ARCHITECTURE.md)
- [Loom Core - AgentDirectory](../../core/src/agent/README.md)
