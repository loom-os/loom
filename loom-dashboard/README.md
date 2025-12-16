# Loom Dashboard

Web-based UI for Loom agent runtime, providing real-time observability and interactive agent control.

## Features

- 🔍 **Real-time Observability**

  - Event stream visualization
  - Agent topology graph
  - Performance metrics
  - Distributed tracing

- 💬 **Interactive Chat**

  - Conversational interface with agents
  - Streaming responses
  - Multi-agent coordination visibility
  - Tool call monitoring

- 🎛️ **Agent Management**
  - Discover available agents via REST API
  - Monitor running agents status
  - View agent capabilities and topics
  - Filter cognitive agents for interaction
  - Real-time heartbeat monitoring

## Implementation Status

### ✅ Completed (Issues 1-6)

- Backend crate skeleton with axum server
- WebSocket bidirectional communication
- EventBus bridge for real-time updates
- **Agent Discovery API** (`GET /api/agents`)
- Frontend WebSocket hook (`useWebSocket`)
- Frontend Agent Discovery hook (`useAgents`)
- **AgentSelector Component** with status indicators
- **ChatPage Integration** with agent selection UI
- Comprehensive test coverage

### 🚧 In Progress

- Issue 5: Chat routing implementation (EventBus integration)
- Issue 7: Streaming chat with multiple agents
- Issue 8: Permission request handling

See [backend/docs/](./backend/docs/) for detailed implementation guides.

## Architecture

```
┌──────────────────────────────────────────────────────────────────┐
│                    Browser (React + Vite)                         │
│  - Chat interface                                                 │
│  - Observability views                                            │
│  - Real-time updates                                              │
└─────────────────────────────┬────────────────────────────────────┘
                              │ WebSocket + REST
                              ▼
┌──────────────────────────────────────────────────────────────────┐
│              loom-dashboard Backend (Rust)                        │
│  - Axum HTTP/WebSocket server                                    │
│  - Message routing to EventBus                                   │
│  - Real-time event streaming                                     │
└─────────────────────────────┬────────────────────────────────────┘
                              │ Direct access
                              ▼
┌──────────────────────────────────────────────────────────────────┐
│                      loom-core                                    │
│  - EventBus                                                       │
│  - AgentDirectory                                                 │
│  - FlowTracker                                                    │
└──────────────────────────────────────────────────────────────────┘
```

## Quick Start

### Prerequisites

- Rust 1.70+
- Node.js 18+ (for frontend development)
- Running Loom Core instance

### Development Mode

```bash
# Terminal 1: Start frontend with hot reload
cd frontend
npm install
npm run dev  # Runs on http://localhost:5173

# Terminal 2: Start backend
cd backend
export LOOM_DASHBOARD_DEV=true
cargo run
```

The frontend will automatically proxy API/WebSocket requests to the backend on `localhost:3030`.

### Production Build

```bash
# Build frontend
cd frontend
npm run build

# Build backend (optionally with embedded frontend)
cd backend
cargo build --release --features embed-frontend

# Run
./target/release/loom-dashboard
```

## Configuration

Configure via environment variables:

```bash
# Server
export LOOM_DASHBOARD_HOST=127.0.0.1  # Bind address
export LOOM_DASHBOARD_PORT=3030       # Server port

# WebSocket
export LOOM_DASHBOARD_WS_MAX_CONNECTIONS=1000  # Max concurrent connections

# CORS (for development)
export LOOM_DASHBOARD_CORS_ENABLED=true
export LOOM_DASHBOARD_CORS_ORIGINS=http://localhost:5173

# Development mode
export LOOM_DASHBOARD_DEV=true  # Enable debug endpoints, relaxed CORS
```

## Project Structure

```
loom-dashboard/
├── Cargo.toml              # Workspace configuration
├── README.md               # This file
│
├── backend/                # Rust backend
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs         # Library entry point
│       ├── config.rs      # Configuration management
│       └── server.rs      # HTTP/WebSocket server
│
└── frontend/              # React frontend
    ├── package.json
    ├── vite.config.ts
    └── src/
        ├── App.tsx
        ├── components/    # UI components
        ├── pages/         # Page components
        ├── hooks/         # React hooks
        └── lib/           # Utilities and API client
```

## WebSocket Protocol

### Message Types

#### Chat Messages

```typescript
// Client → Server: Send chat message
{
  "type": "chat.request",
  "thread_id": "thread-123",
  "content": "What's the weather in Tokyo?",
  "settings": {
    "model": "gpt-4o",
    "temperature": 0.7
  }
}

// Server → Client: Streaming chunk
{
  "type": "chat.chunk",
  "thread_id": "thread-123",
  "content": "The weather in Tokyo is ",
  "content_type": "text",
  "sequence": 0
}

// Server → Client: Completion
{
  "type": "chat.complete",
  "thread_id": "thread-123",
  "stats": {
    "duration_ms": 1234,
    "total_tokens": 156,
    "tool_calls": 1
  }
}
```

#### Observability Messages

```typescript
// Server → Client: Event stream
{
  "type": "event.stream",
  "event_id": "evt-456",
  "timestamp": "2025-12-16T10:30:00Z",
  "topic": "chat.input",
  "sender": "chat-client-abc",
  "payload_preview": "What's the weather..."
}

// Server → Client: Topology update
{
  "type": "topology.update",
  "agents": [
    {
      "id": "chat-assistant",
      "topics": ["chat.input"],
      "capabilities": ["chat", "research"]
    }
  ]
}

// Server → Client: Metrics update
{
  "type": "metrics.update",
  "events_per_sec": 12.5,
  "active_agents": 3,
  "avg_latency_ms": 45
}
```

## API Endpoints

### REST API

- `GET /health` - Health check
- `GET /api/config` - Current configuration
- `GET /api/topology` - Agent topology snapshot
- `GET /api/metrics` - Performance metrics

### WebSocket

- `GET /ws` - WebSocket endpoint for bidirectional communication

## Development

### Backend Development

```bash
cd backend

# Run tests
cargo test

# Run with logging
RUST_LOG=debug cargo run

# Check documentation
cargo doc --open
```

### Frontend Development

```bash
cd frontend

# Start dev server
npm run dev

# Type checking
npm run type-check

# Linting
npm run lint

# Build for production
npm run build
```

## Testing

### Backend Tests

```bash
cd backend
cargo test
```

### Frontend Tests

```bash
cd frontend
npm test
```

### Integration Tests

```bash
# Start backend
cd backend && cargo run &

# Start frontend
cd frontend && npm run dev &

# Run E2E tests
npm run test:e2e
```

## Roadmap

- [x] Basic server infrastructure
- [ ] WebSocket implementation (Issue 2)
- [ ] Chat functionality (Issues 4-6)
- [ ] Observability features (Issues 7-9)
- [ ] Agent management UI
- [ ] Long-term memory integration
- [ ] Desktop app packaging (Tauri)

## Contributing

See [CONTRIBUTING.md](../../CONTRIBUTING.md) for development guidelines.

## License

Apache-2.0
