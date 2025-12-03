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

```python
async def call_weather(ctx, city: str):
    result_bytes = await ctx.tool("weather.get", payload={"city": city})
    return result_bytes.decode()
```

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

## Deep Research (Phase 2)

- Enter `/research` mode
- Spawn sub-agents via events (`agent.spawn`)
- Collect results (`agent.result`)
- Save final report to `workspace/reports/`

See `ROADMAP.md` for acceptance criteria.
