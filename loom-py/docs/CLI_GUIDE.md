# Loom CLI Guide

## Overview

The Loom CLI provides interactive chat interface for cognitive agents with real-time visualization of reasoning and context engineering.

## Commands

### `loom chat`

Start an interactive chat session with a running agent.

```bash
loom chat
```

**Requirements**: A Loom runtime must be running (`loom run` or `loom up`)

### Chat Commands

Within the chat session:

| Command             | Description                                      |
| ------------------- | ------------------------------------------------ |
| `/help`             | Show available commands and tools                |
| `/clear`            | Clear conversation history                       |
| `/history`          | Show conversation history                        |
| `/verbose`          | Toggle verbose mode (show thinking steps)        |
| `/stream`           | Toggle streaming mode                            |
| `/research <topic>` | Deep research mode with multi-step investigation |
| `/quit`             | Exit chat session                                |

## Display Features

### Tool Execution Display

The CLI shows tool executions with context engineering optimizations:

#### Normal Output

```
🔧 Calling tool: weather:get
   ✅ Result:
      Temperature: 15°C
      Conditions: Sunny
```

#### Offloaded Output

When data is offloaded to files (large outputs):

```
🔧 Calling tool: web:search
   ✅ Result:
      📄 Data offloaded to: .loom/cache/search/websearch_123.json
      💡 Summary: Search completed with 5 results
```

#### Error Display

```
🔧 Calling tool: fs:read_file
   ❌ Error: File not found
```

### Context Engineering Metrics

At the end of each interaction, metrics are shown:

```
════════════════════════════════════════════════════════════════
🤖 Assistant:

[Response content]

────────────────────────────────────────────────────────────────
⚡ 8 iterations │ ⏱️  2341ms │ ✅ Success
📊 Context: 3 offloaded outputs
```

## Streaming Mode

In streaming mode, you see the LLM's thinking process in real-time:

```
💭 Thinking...
──────────────────────────────────────────────────
I need to search for pricing information...

🔧 Calling tool: web:search
   ✅ Result:
      📄 Data offloaded to: .loom/cache/search/result.json
      💡 Summary: Found 5 results

Now I can analyze the pricing...

════════════════════════════════════════════════════════════════
🤖 Assistant:

Based on my research, the pricing is...
```

## Verbose Mode

Toggle verbose mode to see detailed thinking steps:

```
/verbose

💭 Thinking Process:
  ┌─ Step 1 ───────────────────────────────────┐
  │
  │ 💭 Thought:
  │    I need to search for pricing
  │
  │ 🔧 Action: web:search
  │    Args: {'query': 'pricing', 'limit': 5}
  │
  │ ✅ Observation:
  │    Found 5 results
  └─────────────────────────────────────────────┘
```

## Research Mode

Deep research mode performs multi-step investigation:

```
/research AI agent frameworks

🔬 Deep Research Mode
Topic: AI agent frameworks
──────────────────────────────────────────────────
📚 Starting deep research on: AI agent frameworks
📋 Phase 1: Planning research approach...
   ✅ Research plan created
🔍 Phase 2: Investigating questions...
   ✅ Investigation complete (5 iterations)
📝 Phase 3: Synthesizing report...
💾 Phase 4: Saving report...
   ✅ Report saved to: workspace/reports/20241206_143025_AI_agent_frameworks.md

══════════════════════════════════════════════════════════════
📊 Research Complete

📄 Report saved: workspace/reports/20241206_143025_AI_agent_frameworks.md
Total iterations: 12

Summary:
[Report preview...]
```

## Permission System

Destructive operations require user approval:

```
──────────────────────────────────────────────────
⚠️  Permission Required
Tool: fs:write_file
Args: {'path': 'config.json', 'content': '...'}
Reason: Write to file 'config.json'
──────────────────────────────────────────────────
Allow this action? [y/N]: y
✅ Approved by user
```

Tools requiring approval:

- `fs:write_file` - File writing
- `fs:delete` - File/directory deletion
- `system:shell` - Shell commands (some)

## Configuration

Chat session configuration is read from project's `loom.toml`:

```toml
[agents.chat-assistant]
llm_provider = "deepseek"
thinking_strategy = "react"
max_iterations = 10
```

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│  CLI (loom/cli/chat.py)                                     │
│  ├─ ChatSession                                             │
│  │  ├─ User input handling                                  │
│  │  ├─ Display formatting                                   │
│  │  └─ Permission callbacks                                 │
│  │                                                           │
│  └─ Display Functions                                       │
│     ├─ print_stream_step_complete() ← Context engineering!  │
│     ├─ print_result()                                       │
│     └─ print_thinking_step()                                │
└─────────────────────────────────────────────────────────────┘
                        ↓
┌─────────────────────────────────────────────────────────────┐
│  CognitiveAgent (loom/cognitive/agent.py)                   │
│  ├─ ReAct Loop                                              │
│  ├─ Context Engineering                                     │
│  │  ├─ StepReducer                                          │
│  │  ├─ DataOffloader                                        │
│  │  └─ StepCompactor                                        │
│  └─ Tool Execution                                          │
└─────────────────────────────────────────────────────────────┘
                        ↓
┌─────────────────────────────────────────────────────────────┐
│  Rust Bridge (via gRPC)                                     │
│  ├─ Event Bus                                               │
│  ├─ Tool Registry                                           │
│  └─ Agent Directory                                         │
└─────────────────────────────────────────────────────────────┘
```

## Context Engineering Integration

The CLI is fully integrated with context engineering:

1. **Display Layer** (`print_stream_step_complete`):

   - Detects `step.reduced_step.outcome_ref`
   - Shows file path for offloaded data
   - Displays summary instead of full output

2. **Metrics** (`print_result`):

   - Counts offloaded outputs
   - Shows context efficiency

3. **Test Coverage**:
   - `tests/integration/test_context_engineering.py::TestCLIDisplay`
   - Validates attribute access
   - Verifies display format

## Troubleshooting

### Connection Failed

```
❌ Failed to connect: ...
Make sure Loom runtime is running (loom run or loom up)
```

**Solution**: Start runtime first:

```bash
cd apps/chat-assistant
loom run
```

### Tool Execution Errors

Check if the tool is available and agent has permission.

### AttributeError in Display

If you see `'Step' object has no attribute 'outcome'`:

- This was a bug fixed in v0.2.1
- `Step` uses `observation` not `outcome`
- Update to latest version

## See Also

- [Cognitive Guide](COGNITIVE_GUIDE.md) - Agent reasoning patterns
- [Context Engineering](context/DESIGN.md) - Token optimization
- [SDK Guide](SDK_GUIDE.md) - Building agents
