# Cognitive Loop Guide

Describes Loom's Python cognitive loop: ReAct-style reasoning, tool use via the Rust Bridge, context engineering, and memory patterns.

## Loop Structure

- Receive user intent or events
- Reason (plan next action)
- Act (call tools/capabilities)
- Observe results
- Iterate until objective is met

## Basic Pattern

```python
from loom.cognitive.agent import CognitiveAgent
from loom.llm.provider import LLMProvider

agent = CognitiveAgent(provider_name="deepseek")

async def run_once(user_input: str):
    thought = await agent.think(user_input)
    action = await agent.decide(thought)
    observation = await agent.act(action)
    return agent.summarize(thought, action, observation)
```

## Tool Use via Bridge

Tools are registered in the Rust Core and called through the Bridge:

```python
async def call_weather(ctx, city: str):
    result_bytes = await ctx.tool("weather:get", payload={"location": city})
    return result_bytes.decode()
```

### Available Tools

| Tool            | Description                | Arguments                                 |
| --------------- | -------------------------- | ----------------------------------------- |
| `weather:get`   | Get weather for a location | `{"location": "city"}`                    |
| `system:shell`  | Run allowed shell commands | `{"command": "ls"}` (ls, echo, cat, grep) |
| `fs:read_file`  | Read file contents         | `{"path": "relative/path"}`               |
| `fs:write_file` | Write content to file      | `{"path": "path", "content": "text"}`     |
| `fs:list_dir`   | List directory             | `{"path": "path"}` (optional)             |
| `fs:delete`     | Delete file or empty dir   | `{"path": "path"}`                        |

Tools execute in the Rust sandbox through the Loom Bridge, keeping system operations safe and isolated from the Python brain.

## Context Engineering

- Maintain a short system prompt defining role and constraints.
- Use thread IDs for conversation continuity.
- Rank and window messages to stay within token limits.

```python
from loom.context.window.manager import WindowManager
wm = WindowManager(max_tokens=4000)
# wm.add(message) ...
```

## Memory (Planned)

- Working memory: within current loop
- Short-term memory: session cache
- Long-term memory: persistent store (RocksDB)

## Error Handling & Backpressure

- Catch tool/bridge errors and retry selectively.
- Monitor event bus metrics via dashboard (top-level `docs/dashboard`).

## Deep Research Mode (Phase 2)

Enter research mode with the `/research` command in `loom chat`:

```
You ▶ /research AI agent frameworks
```

This triggers:

1. **Planning**: Creates a research plan with key questions
2. **Investigation**: Uses tools to gather information
3. **Synthesis**: Combines findings into a structured report
4. **Saving**: Writes report to `workspace/reports/`

### Research Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    /research "topic"                             │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  Phase 1: Plan                                          │   │
│  │  • Analyze topic                                        │   │
│  │  • Create 3-5 research questions                        │   │
│  └─────────────────────────────────────────────────────────┘   │
│                              │                                  │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  Phase 2: Investigate                                   │   │
│  │  • Use tools to gather data                             │   │
│  │  • Multiple iterations as needed                        │   │
│  └─────────────────────────────────────────────────────────┘   │
│                              │                                  │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  Phase 3: Synthesize                                    │   │
│  │  • Combine findings                                     │   │
│  │  • Structure as markdown report                         │   │
│  └─────────────────────────────────────────────────────────┘   │
│                              │                                  │
│                    workspace/reports/                           │
└─────────────────────────────────────────────────────────────────┘
```

### Future: Agent Spawning

Phase 2.2 will add proper agent spawning via events:

- `agent.spawn` event to create sub-agents
- `agent.result` event to collect results
- Context isolation per sub-agent

See `ROADMAP.md` for acceptance criteria.
