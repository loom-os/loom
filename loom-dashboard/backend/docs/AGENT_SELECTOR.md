# Issue 6: Frontend Agent Selector - Implementation Guide

## Overview

Issue 6 implements the frontend UI for selecting which cognitive agent to chat with. This builds on Issue 4's agent discovery API.

## Components

### 1. AgentSelector Component

**Location:** `frontend/src/components/AgentSelector.tsx`

**Features:**

- Grid layout of agent cards
- Real-time status indicators (active, idle, inactive, disconnected)
- Capability badges
- Topic subscriptions display
- Last seen timestamp
- Auto-select first active agent
- Auto-refresh every 10 seconds

**Props:**

```typescript
interface AgentSelectorProps {
  selectedAgentId?: string | null;
  onSelectAgent: (agentId: string) => void;
  cognitiveOnly?: boolean; // Filter cognitive agents only
  className?: string;
}
```

**Usage:**

```tsx
<AgentSelector
  selectedAgentId={selectedAgentId}
  onSelectAgent={(agentId) => {
    setSelectedAgentId(agentId);
    // Optionally close the selector
  }}
  cognitiveOnly={true}
/>
```

### 2. Updated ChatPage

**Location:** `frontend/src/pages/ChatPage.tsx`

**New Features:**

- Agent selector Sheet (slide-in from left)
- Selected agent indicator in status bar
- Agent ID included in chat requests
- Warning when no agent is selected

**Key Changes:**

```tsx
// State
const [selectedAgentId, setSelectedAgentId] = useState<string | null>(null);
const [agentSelectorOpen, setAgentSelectorOpen] = useState(false);

// Chat request now includes agent_id
send({
  type: 'chat_request',
  thread_id: threadIdRef.current,
  content,
  agent_id: selectedAgentId,  // ← New field
  settings: { ... },
});
```

## UI/UX Flow

### 1. Opening the Agent Selector

- Click the **Users icon** (👥) button in top-left corner
- Slide-in panel opens from the left
- Shows all available cognitive agents

### 2. Selecting an Agent

- Click on any agent card
- Card gets highlighted with primary ring
- "Selected" badge appears
- Panel auto-closes
- Selected agent appears in status bar

### 3. Chatting with Selected Agent

- Send messages normally
- Agent ID is included in request
- Messages show which agent responded (agentId badge)

### 4. No Agent Selected

- If user tries to send without selecting agent:
  - Warning message appears in chat
  - Agent selector automatically opens

## Visual Design

### Agent Card

```
┌─────────────────────────────────────┐
│ [Bot Icon] chat-assistant           │
│            ● Active                  │
│                          [Selected]  │
├─────────────────────────────────────┤
│ ⚡ Capabilities                      │
│   [web_search] [file_write] +2 more │
│                                      │
│ ● Topics                             │
│   [chat.input]                       │
│                                      │
│ 🕐 Last seen 2m ago                 │
└─────────────────────────────────────┘
```

### Status Indicators

- 🟢 **Active**: Green - Agent is ready
- 🟡 **Idle**: Yellow - Agent connected but idle
- ⚪ **Inactive**: Gray - No recent heartbeat
- 🔴 **Disconnected**: Red - Agent disconnected

### Status Bar (ChatPage)

```
[📡 Connected] [👥 chat-assistant]     Thread: thread-1234567890
```

## Agent Detection Heuristics

The `useAgents` hook's `getCognitiveAgents()` method identifies chattable agents:

1. **Topic-based**:

   - Subscribed to topics containing "input" or "chat"
   - Examples: `chat.input`, `market.input`, `*.input`

2. **Metadata-based**:
   - `metadata.type === "cognitive"`
   - `metadata.role === "assistant"`

## Integration with Backend (Issue 5)

When Issue 5 is complete, the flow will be:

```
User clicks send
    ↓
ChatPage includes agent_id in request
    ↓
WebSocket sends to Dashboard backend
    ↓
Dashboard routes to selected agent's input_topic
    ↓
Agent processes and streams response
    ↓
Dashboard forwards chunks to WebSocket
    ↓
ChatPage renders streaming response
```

## Testing

### Manual Testing

1. **Start Dashboard**:

   ```bash
   cd loom-dashboard/frontend
   npm run dev
   ```

2. **Start Backend** (in another terminal):

   ```bash
   cd loom-dashboard
   cargo run --example basic_server
   ```

3. **Start Test Agents** (in another terminal):

   ```bash
   cd apps/chat-assistant
   loom run
   ```

4. **Test Flow**:
   - Open http://localhost:5173/chat
   - Click Users icon (top-left)
   - Verify agents appear in selector
   - Click an agent card
   - Verify "Selected" badge appears
   - Verify agent name appears in status bar
   - Try sending a message
   - Check console for agent_id in request

### Empty State Testing

1. Start Dashboard without any agents
2. Open agent selector
3. Should see: "No agents available" message
4. Start an agent with `loom run`
5. Selector should auto-refresh and show the agent

### Status Testing

1. Start an agent
2. Verify status shows "Active" (green)
3. Stop the agent
4. Wait 10 seconds (auto-refresh)
5. Status should update to "Disconnected" (red)

## Known Limitations (To be addressed in Issue 5)

1. **No actual routing**: Messages aren't routed to selected agent yet
2. **Mock responses**: Backend still returns echo responses
3. **No streaming**: Streaming protocol not implemented yet
4. **No error handling**: Agent unavailability not handled

## Configuration

### Auto-refresh Interval

Modify in `AgentSelector.tsx`:

```tsx
const { agents } = useAgents({
  refreshInterval: 10000, // milliseconds (default: 10s)
});
```

### Agent Filtering

Show all agents (not just cognitive):

```tsx
<AgentSelector
  cognitiveOnly={false}  // Show all agents
  onSelectAgent={...}
/>
```

## Accessibility

- **Keyboard navigation**: Agent cards are focusable
- **Screen readers**: Cards have proper ARIA labels
- **Color contrast**: Status indicators meet WCAG AA
- **Tooltips**: Hover for additional information

## File Changes Summary

### New Files

- ✅ `frontend/src/components/AgentSelector.tsx` (290 lines)

### Modified Files

- ✅ `frontend/src/pages/ChatPage.tsx`
  - Added agent selector Sheet
  - Added selected agent state
  - Modified send handler to include agent_id
  - Added status bar indicator

### Dependencies

- Uses existing UI components (Sheet, Card, Badge, etc.)
- Uses `useAgents` hook from Issue 4
- No new npm packages required

## Next Steps

**Issue 5: Chat Routing** will complete the integration:

1. Backend WebSocket handler receives `agent_id`
2. Dashboard creates EventBus envelope
3. Routes to agent's `input_topic`
4. Subscribes to reply topic
5. Streams response back to frontend

After Issue 5, the entire flow will be functional end-to-end!

## Troubleshooting

### Agent selector shows "Loading..." forever

**Cause**: Backend not responding to `/api/agents`

**Solution**:

1. Check backend is running: `curl http://localhost:3030/api/agents`
2. Check browser console for CORS errors
3. Verify CORS is enabled: `LOOM_DASHBOARD_CORS_ENABLED=true`

### No agents appear (empty state)

**Cause**: No agents registered

**Solution**:

1. Start at least one agent: `cd apps/chat-assistant && loom run`
2. Verify agent is registered: `curl http://localhost:3030/api/agents | jq`
3. Check agent logs for registration confirmation

### Agent status always shows "Disconnected"

**Cause**: Agent not sending heartbeats

**Solution**:

1. Check agent is still running
2. Verify Bridge connectivity
3. Check agent logs for errors

### Selected agent doesn't appear in status bar

**Cause**: State not updating

**Solution**:

1. Check browser console for errors
2. Verify `onSelectAgent` callback is firing
3. Check React DevTools for state updates

## References

- [Issue 4: Agent Discovery API](./AGENT_DISCOVERY.md)
- [Issue 5: Chat Routing](./CHAT_ROUTING.md) (coming next)
- [useAgents Hook](../frontend/src/hooks/useAgents.ts)
- [AgentSelector Component](../frontend/src/components/AgentSelector.tsx)
