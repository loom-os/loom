# Chat Routing Implementation

This document describes the implementation of Issue 5: Chat Routing from WebSocket to EventBus agents.

## Overview

The chat routing system enables the dashboard to forward chat requests from WebSocket clients to specific agents via the EventBus, and stream back responses in real-time.

## Architecture

```text
┌─────────────────┐
│ WebSocket Client│
│   (Browser)     │
└────────┬────────┘
         │ ChatRequest(agent_id, content)
         ▼
┌─────────────────┐
│  ChatRouter     │
│                 │
│  - Session Mgmt │
│  - Topic Sub    │
│  - Stream Relay │
└────────┬────────┘
         │ stream.request
         ▼
┌─────────────────┐         ┌─────────────────┐
│   EventBus      │◀────────│  Target Agent   │
│                 │ chunks  │                 │
└────────┬────────┘         └─────────────────┘
         │ stream.chunk, stream.complete
         ▼
┌─────────────────┐
│  ChatRouter     │
│  (forward)      │
└────────┬────────┘
         │ ChatChunk messages
         ▼
┌─────────────────┐
│ WebSocket Client│
└─────────────────┘
```

## Implementation Details

### 1. ChatRouter (`backend/src/ws/router.rs`)

The `ChatRouter` struct manages active chat sessions and routes messages between WebSocket clients and EventBus agents.

**Key Components:**

- `event_bus`: EventBus instance for pub/sub
- `agent_directory`: Directory to lookup agent metadata
- `connection_manager`: Manager for WebSocket connections
- `sessions`: Concurrent map of active chat sessions

**Session Lifecycle:**

1. **Request** - Client sends `ChatRequest` with `agent_id` and `content`
2. **Validation** - Router validates agent exists and is active
3. **Subscribe** - Router subscribes to unique reply topic: `agent.dashboard.replies.{correlation_id}`
4. **Publish** - Router publishes `stream.request` to agent's input topic
5. **Stream** - Router spawns task to receive chunks with 120s timeout
6. **Forward** - Router forwards each `stream.chunk` to WebSocket connection
7. **Complete** - Router receives `stream.complete`, sends stats, cleans up
8. **Cleanup** - Router unsubscribes from EventBus and removes session

### 2. Message Protocol

#### WsMessage::ChatRequest (Client → Server)

```json
{
  "type": "chat_request",
  "thread_id": "conversation-uuid",
  "content": "user message",
  "agent_id": "target-agent-id",
  "settings": {
    /* optional */
  }
}
```

#### stream.request (Router → Agent via EventBus)

```json
{
  "type": "stream.request",
  "from": "dashboard",
  "to": "agent-id",
  "correlation_id": "unique-uuid",
  "content": "user message",
  "metadata": {
    "reply_topic": "agent.dashboard.replies.{correlation_id}",
    "thread_id": "conversation-uuid"
  }
}
```

#### stream.chunk (Agent → Router via EventBus)

```json
{
  "type": "stream.chunk",
  "content": "response text",
  "metadata": {
    "stream.sequence": "0",
    "stream.content_type": "text"
  }
}
```

#### WsMessage::ChatChunk (Router → Client)

```json
{
  "type": "chat_chunk",
  "thread_id": "conversation-uuid",
  "content": "response text",
  "content_type": "text",
  "sequence": 0
}
```

#### stream.complete (Agent → Router via EventBus)

```json
{
  "type": "stream.complete",
  "metadata": {
    "stream.duration_ms": "1500",
    "stream.total_tokens": "250",
    "stream.tool_calls": "2"
  }
}
```

#### WsMessage::ChatComplete (Router → Client)

```json
{
  "type": "chat_complete",
  "thread_id": "conversation-uuid",
  "stats": {
    "duration_ms": 1500,
    "total_tokens": 250,
    "tool_calls": 2
  }
}
```

### 3. Error Handling

**Agent Not Found**

- Router checks `AgentDirectory` before routing
- Sends `WsMessage::Error` to client if agent doesn't exist

**Agent Inactive**

- Router checks agent status (Active, Idle, Disconnected)
- Sends error if agent is not Active

**Stream Timeout**

- Router spawns task with 120-second timeout
- Sends timeout error if agent doesn't respond in time
- Automatically cleans up session

**Connection Lost**

- If WebSocket connection closes during stream, session is cleaned up
- EventBus subscription is removed

### 4. Concurrency & Thread Safety

- `DashMap` for lock-free concurrent session storage
- `Arc` for shared ownership across async tasks
- Each stream handled in separate tokio task
- No blocking operations in request path

### 5. Integration Points

**DashboardState** (`backend/src/server.rs`)

```rust
pub struct DashboardState {
    pub event_bus: Arc<EventBus>,
    pub agent_directory: Arc<AgentDirectory>,
    pub connection_manager: Arc<ConnectionManager>,
    pub chat_router: Arc<ChatRouter>,  // New field
}
```

**WebSocket Handler** (`backend/src/ws/handler.rs`)

```rust
async fn handle_chat_request(...) -> Result<...> {
    state.chat_router.handle_chat_request(
        connection_id.to_string(),
        thread_id,
        content,
        Some(agent_id),
    ).await?;
    Ok(())
}
```

## Testing

All tests passing (31 total):

- **Unit Tests** (21): Message serialization, connection management, routing logic
- **Integration Tests** (5): Server startup, config, multi-server
- **WebSocket Tests** (5): Connection, chat, error handling, ping/pong

## Usage Example

### Frontend (TypeScript)

```typescript
// User selects agent from AgentSelector
const selectedAgent = "cognitive-agent-1";

// Send chat request via WebSocket
socket.send(
  JSON.stringify({
    type: "chat_request",
    thread_id: "conv-123",
    content: "Explain quantum computing",
    agent_id: selectedAgent,
  })
);

// Receive streaming response
socket.onmessage = (event) => {
  const msg = JSON.parse(event.data);
  if (msg.type === "chat_chunk") {
    appendToChat(msg.content);
  } else if (msg.type === "chat_complete") {
    console.log("Tokens:", msg.stats.total_tokens);
  }
};
```

### Backend Flow

1. WebSocket handler receives `ChatRequest`
2. Handler calls `ChatRouter.handle_chat_request()`
3. Router validates agent and creates session
4. Router subscribes to `agent.dashboard.replies.{id}`
5. Router publishes `stream.request` to agent's input topic
6. Agent processes and sends chunks to reply topic
7. Router forwards chunks to WebSocket client
8. Agent sends `stream.complete`
9. Router sends stats and cleans up

## Performance Considerations

- **Low Latency**: Direct EventBus access (no gRPC hop)
- **Memory Efficient**: Sessions cleaned up after completion or timeout
- **Concurrent**: Multiple streams can run simultaneously
- **Non-blocking**: All I/O is async

## Future Enhancements

1. **Session Persistence**: Store active sessions for dashboard restarts
2. **Rate Limiting**: Limit requests per client
3. **Analytics**: Track message counts, latencies, errors
4. **Agent Load Balancing**: Route to least-busy agent if multiple available
5. **Message Priority**: QoS-based prioritization
6. **Retry Logic**: Auto-retry on temporary failures

## Related Files

- `backend/src/ws/router.rs` - Core routing logic
- `backend/src/ws/handler.rs` - WebSocket message handling
- `backend/src/ws/message.rs` - Message protocol definitions
- `backend/src/server.rs` - Server initialization and state
- `frontend/src/pages/ChatPage.tsx` - Chat UI integration
- `frontend/src/hooks/useAgents.ts` - Agent discovery
- `frontend/src/components/AgentSelector.tsx` - Agent selection UI
