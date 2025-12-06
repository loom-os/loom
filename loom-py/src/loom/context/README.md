# Loom Context Engineering

> **Production-grade context management for AI agents.**

This module implements the Context Engineering system based on Manus learnings,
adapted for Loom's Brain/Hand architecture.

## Quick Links

- **Full Design**: `docs/context/DESIGN.md`
- **Reduction & Compaction**: `docs/context/REDUCTION.md`
- **Multi-Agent Isolation**: `docs/context/ISOLATION.md`
- **Data & Logic Offloading**: `docs/context/OFFLOADING.md`

## Capability Overview

```
Context Engineering
├── Reduction      — Per-step minimal observations
├── Compaction     — Rule-based history compression
├── Isolation      — Per-agent independent context
├── Offloading     — Data/tool/logic externalization
├── Hierarchy      — L1/L2/L3 action space
└── Memory Tiers   — Working/short-term/long-term
```

## Module Structure

```
context/
├── __init__.py              # Public exports
├── README.md                # This file
├── ranking.py               # ContextRanker, ScoredItem
├── window.py                # TokenWindowManager, TokenBudget
├── engineering/             # Context optimization
│   ├── __init__.py
│   ├── step.py              # Step, CompactStep dataclasses
│   ├── reducer.py           # StepReducer with per-tool rules
│   ├── compactor.py         # StepCompactor for history compression
│   └── offloader.py         # DataOffloader for large outputs
├── prompting/               # Prompt construction
│   ├── __init__.py
│   ├── builder.py           # ContextBuilder, ContextWindow
│   ├── few_shot.py          # FewShotExample, FewShotLibrary
│   └── tool_descriptor.py   # ToolDescriptor, ToolRegistry
└── memory/                  # Memory management
    ├── __init__.py
    ├── types.py             # MemoryItem, MemoryTier
    ├── working.py           # WorkingMemory
    └── store.py             # InMemoryStore
```

## Organization Principles

### 1. Flat is Better Than Nested (for simple modules)

- `ranking.py` and `window.py` are at the top level (114 and 156 lines respectively)
- No need for subdirectories when modules are small and focused

### 2. Logical Grouping (for related functionality)

- **engineering/**: Token optimization techniques (reducer, compactor, offloader, step)
- **prompting/**: Prompt construction utilities (builder, few_shot, tool_descriptor)
- **memory/**: Memory management (working, store, types)

### 3. Progressive Disclosure

- Import from top level: `from loom.context import StepReducer`
- Or from submodule: `from loom.context.engineering import StepReducer`
- Both work, use what makes sense for your code

```

## Usage Example

```python
from loom.context import (
    ContextBuilder,
    WorkingMemory,
    TokenWindowManager,
)

# Build context with token budget
builder = ContextBuilder(max_tokens=8192)
builder.set_system("You are a helpful assistant.")
builder.add_memory_items(working_memory.get_context())
context = builder.build()

# Manage token window
window = TokenWindowManager(max_tokens=8192)
window.allocate(system=1500, history=2000, context=1500, response=3192)
```

## Design Principles

1. **Reduction over Retention** — Keep minimal, discard recoverable
2. **Compaction over Summarization** — Rule-based, not LLM-based
3. **Isolation over Sharing** — Each agent owns its context
4. **Offloading over Embedding** — External storage for large data
5. **Hierarchy over Flatness** — Fewer tools visible to LLM

## Integration with Cognitive Loop

The Context Engineering module integrates with `CognitiveAgent`:

```python
class CognitiveAgent:
    def __init__(self, ...):
        self.reducer = StepReducer()
        self.compactor = StepCompactor()
        self.offloader = DataOffloader(workspace)

    async def _execute_tool(self, tool_call):
        result = await self.ctx.tool(...)
        step = self.reducer.reduce(tool_call, result)  # Reduce
        if step.needs_offload:
            step.ref = self.offloader.offload(result)  # Offload
        self.steps.append(step)

    def _build_prompt(self, goal):
        history = self.compactor.compact_many(self.steps)  # Compact
        return f"{system}\n{tools}\n{history}\nGoal: {goal}"
```

---

_See `docs/context/` for detailed specifications._
