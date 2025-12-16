# Content Type Rendering Guide

## Visual Examples

### 1. Text Messages (contentType: 'text')

```
┌──────────────────────────────────────┐
│ 🤖 agent-name                        │
│                                      │
│ **Regular markdown text**            │
│ - Lists work                         │
│ - With bullets                       │
│                                      │
│ `inline code` and links too!         │
│                                      │
│ 2:30 PM                              │
└──────────────────────────────────────┘
```

**Styling**: Muted background, full Markdown support, code highlighting

---

### 2. Thinking Steps (contentType: 'thinking')

```
┌──────────────────────────────────────┐
│ 🤖 agent-name   [🧠 Thinking]       │
│                                      │
│ I need to search for information     │
│ about the user's question. First,    │
│ let me break down the query...       │
│                                      │
│ 2:30 PM                              │
└──────────────────────────────────────┘
```

**Styling**: Purple border/background, italic text, brain icon

---

### 3. Tool Calls (contentType: 'tool_call')

```
┌──────────────────────────────────────┐
│ 🤖 agent-name   [🔧 Tool Call]      │
│                                      │
│ 🔧 web_search                        │
│ ┌────────────────────────────────┐   │
│ │ {                              │   │
│ │   "query": "Rust async",       │   │
│ │   "max_results": 5             │   │
│ │ }                              │   │
│ └────────────────────────────────┘   │
│                                      │
│ 2:30 PM                              │
└──────────────────────────────────────┘
```

**Styling**: Cyan border/background, wrench icon, JSON syntax highlighting

---

### 4. Tool Results (contentType: 'tool_result')

```
┌──────────────────────────────────────┐
│ 🤖 agent-name   [✅ Tool Result]    │
│                                      │
│ ✅ Result:                           │
│ ┌────────────────────────────────┐   │
│ │ Found 5 results about Rust     │   │
│ │ async programming:             │   │
│ │ 1. tokio.rs - Async runtime    │   │
│ │ 2. async-std - Alternative     │   │
│ │ ...                            │   │
│ └────────────────────────────────┘   │
│                                      │
│ 2:30 PM                              │
└──────────────────────────────────────┘
```

**Styling**: Green border/background, checkmark icon, auto-truncation

---

### 5. Errors (contentType: 'error')

```
┌──────────────────────────────────────┐
│ 🤖 agent-name   [⚠️ Error]          │
│                                      │
│ ❌ Tool execution failed:            │
│ Connection timeout                   │
│                                      │
│ 2:30 PM                              │
└──────────────────────────────────────┘
```

**Styling**: Red border/background, alert icon, error formatting

---

## Streaming States

### Active Streaming

```
┌──────────────────────────────────────┐
│ 🤖 agent-name   • • •                │
│                                      │
│ The quick brown fox jumps over t     │
│                                      │
└──────────────────────────────────────┘
```

**Animation**: Three pulsing dots (staggered 150ms delays)

### Completed Message

```
┌──────────────────────────────────────┐
│ 🤖 agent-name                        │
│                                      │
│ The quick brown fox jumps over the   │
│ lazy dog.                            │
│                                      │
│ 2:30 PM                              │
└──────────────────────────────────────┘
```

**State**: No pulsing, timestamp visible

---

## Code Highlighting

### Inline Code

```
Use the `useState` hook for state management.
```

**Styling**: Dark background, rounded corners, monospace font

### Code Blocks

````
```python
def fibonacci(n):
    if n <= 1:
        return n
    return fibonacci(n-1) + fibonacci(n-2)
```
````

**Features**:

- Syntax highlighting via highlight.js
- GitHub Dark theme
- Language auto-detection
- Formatted with proper indentation

---

## Multi-Agent Sessions

### Status Bar

```
┌──────────────────────────────────────────────────┐
│ [📡 Connected]  [👥 research-agent ▼]  3 agents used │
└──────────────────────────────────────────────────┘
```

### Agent Switch Notification

```
┌──────────────────────────────────────┐
│          System Message              │
│ 🔄 Switched from agent-a to agent-b  │
└──────────────────────────────────────┘
```

---

## ReAct Pattern Example

Full conversation flow showing content types:

```
User: "Search for Rust async programming tutorials"

┌─ THINKING ───────────────────────────┐
│ 🧠 I need to search the web for      │
│    Rust async tutorials. Let me use  │
│    the web_search tool.              │
└──────────────────────────────────────┘

┌─ TOOL CALL ──────────────────────────┐
│ 🔧 web_search                        │
│ { "query": "Rust async programming   │
│    tutorials", "max_results": 5 }    │
└──────────────────────────────────────┘

┌─ TOOL RESULT ────────────────────────┐
│ ✅ Found 5 results:                  │
│ 1. Tokio Tutorial                    │
│ 2. Async Book                        │
│ ...                                  │
└──────────────────────────────────────┘

┌─ FINAL ANSWER ──────────────────────┐
│ I found several excellent resources: │
│                                      │
│ **1. Tokio Tutorial**                │
│ The official async runtime guide     │
│                                      │
│ **2. Async Programming in Rust**     │
│ Comprehensive book covering...       │
└──────────────────────────────────────┘
```

---

## Component Mapping

| Content Type  | Component       | Icon | Color  |
| ------------- | --------------- | ---- | ------ |
| `text`        | Default         | None | Gray   |
| `thinking`    | ThinkingContent | 🧠   | Purple |
| `tool_call`   | ToolCallCard    | 🔧   | Cyan   |
| `tool_result` | ToolResultPanel | ✅   | Green  |
| `error`       | ErrorPanel      | ⚠️   | Red    |

---

## CSS Classes

```css
/* Content Type Styles */
.thinking-content {
  color: rgb(192, 132, 252); /* purple-400 */
  background: rgba(168, 85, 247, 0.1); /* purple-500/10 */
  border: 1px solid rgba(168, 85, 247, 0.3); /* purple-500/30 */
}

.tool-call-content {
  color: rgb(34, 211, 238); /* cyan-400 */
  background: rgba(6, 182, 212, 0.1); /* cyan-500/10 */
  border: 1px solid rgba(6, 182, 212, 0.3); /* cyan-500/30 */
}

.tool-result-content {
  color: rgb(74, 222, 128); /* green-400 */
  background: rgba(34, 197, 94, 0.1); /* green-500/10 */
  border: 1px solid rgba(34, 197, 94, 0.3); /* green-500/30 */
}

.error-content {
  color: rgb(248, 113, 113); /* red-400 */
  background: rgba(239, 68, 68, 0.1); /* red-500/10 */
  border: 1px solid rgba(239, 68, 68, 0.3); /* red-500/30 */
}
```

---

## Usage Example

```typescript
// Backend sends content with type metadata
{
  "type": "chat_chunk",
  "content": "I need to search...",
  "metadata": {
    "stream.content_type": "thinking"
  }
}

// Frontend detects and routes
const contentType = parseContentType(metadata);
// Result: 'thinking'

// Renders with ThinkingContent component
<MessageBubble message={message} />
// Displays purple panel with brain icon
```
