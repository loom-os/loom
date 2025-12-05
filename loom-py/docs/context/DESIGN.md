# Context Engineering Design

> **"Don't communicate by sharing memory; share memory by communicating."**

This document defines the complete Context Engineering system for Loom, based on production learnings from Manus and adapted for our Brain/Hand architecture.

---

## Overview

Context Engineering is the **most critical capability** for production AI agents. Without it:

- Token costs explode (full history in every request)
- LLM performance degrades (noise drowns signal)
- Multi-agent fails (shared context = shared confusion)

Loom's Context Engineering has **6 core capabilities**:

```
┌─────────────────────────────────────────────────────────────────┐
│                   Context Engineering                            │
│                                                                  │
│   ┌──────────────┐  ┌──────────────┐  ┌──────────────┐         │
│   │  Reduction   │  │  Compaction  │  │  Isolation   │         │
│   │  (per-step)  │  │ (threshold)  │  │ (per-agent)  │         │
│   └──────┬───────┘  └──────┬───────┘  └──────┬───────┘         │
│          │                 │                 │                  │
│          └─────────────────┴─────────────────┘                  │
│                            │                                     │
│   ┌──────────────┐  ┌──────────────┐  ┌──────────────┐         │
│   │  Offloading  │  │ Hierarchical │  │   Memory     │         │
│   │  (data/tool) │  │   Actions    │  │   Tiers      │         │
│   └──────────────┘  └──────────────┘  └──────────────┘         │
└─────────────────────────────────────────────────────────────────┘
```

---

## 1. Reduction

> **Goal**: Every step becomes minimal but recoverable.

### Core Concept

Transform full tool results into **minimal observations**:

```
Before (full):
{
  "tool": "web:search",
  "args": {"query": "Bitcoin price", "count": 10},
  "output": "[10 full search results with titles, URLs, snippets...]"
}

After (reduced):
{
  "tool": "web:search",
  "args": {"query": "Bitcoin price"},  // minimal args
  "observation": "Found 10 results",   // minimal observation
  "output_ref": "workspace/outputs/search_001.json"  // full data offloaded
}
```

### Step Model

```python
@dataclass
class Step:
    """A single cognitive step with reduction applied."""

    id: str                          # Unique step ID (step_001)
    tool_name: str                   # e.g., "fs:read_file"
    minimal_args: dict               # Reduced arguments (no large content)
    observation: str                 # One-line result summary
    outcome_ref: Optional[str]       # Pointer to full output file
    timestamp_ms: int
    success: bool

@dataclass
class CompactStep:
    """Ultra-minimal representation for prompt inclusion."""

    id: str
    summary: str  # e.g., "Read config.json (1.2KB)"
```

### Minimal Observation Rules

Each tool has specific reduction rules:

| Tool               | Keep (minimal)      | Discard (offload) |
| ------------------ | ------------------- | ----------------- |
| `fs:read_file`     | path, size, hash    | file content      |
| `fs:write_file`    | path, bytes_written | content           |
| `web:search`       | query, result_count | full results      |
| `browser:navigate` | url, status         | page DOM/text     |
| `system:shell`     | command, exit_code  | stdout/stderr     |

### Implementation

```python
class StepReducer:
    """Reduces full tool results to minimal observations."""

    def reduce(self, tool_name: str, args: dict, result: dict) -> Step:
        """Apply tool-specific reduction rules."""

        if tool_name == "fs:read_file":
            return self._reduce_file_read(args, result)
        elif tool_name == "web:search":
            return self._reduce_search(args, result)
        # ... other tools

    def _reduce_file_read(self, args: dict, result: dict) -> Step:
        content = result.get("content", "")
        return Step(
            tool_name="fs:read_file",
            minimal_args={"path": args["path"]},
            observation=f"Read file ({len(content)} bytes)",
            outcome_ref=self._maybe_offload(content),
        )
```

---

## 2. Compaction

> **Goal**: Convert step history to ultra-compact form without LLM.

### Compaction vs Summarization

| Approach          | Method                | Pros                      | Cons                             |
| ----------------- | --------------------- | ------------------------- | -------------------------------- |
| **Summarization** | LLM generates summary | Natural language          | Expensive, error-prone, unstable |
| **Compaction**    | Rule-based transform  | Fast, stable, recoverable | Less flexible                    |

**Loom uses Compaction first, Summarization as fallback.**

### StepCompactor

```python
class StepCompactor:
    """Compacts steps to minimal tokens."""

    def compact(self, step: Step) -> CompactStep:
        """Convert Step to CompactStep."""

        templates = {
            "fs:read_file": "Read {path} ({size})",
            "fs:write_file": "Wrote {path} ({bytes} bytes)",
            "web:search": "Searched '{query}' → {count} results",
            "browser:navigate": "Visited {url}",
            "system:shell": "Ran `{command}` → exit {code}",
        }

        template = templates.get(step.tool_name, "{tool_name}({args})")
        summary = template.format(**step.minimal_args, **step.__dict__)

        return CompactStep(id=step.id, summary=summary)

    def compact_many(self, steps: list[Step], keep_recent: int = 3) -> str:
        """Compact a list of steps, keeping recent ones full."""

        if len(steps) <= keep_recent:
            return self._format_full(steps)

        old_steps = steps[:-keep_recent]
        recent_steps = steps[-keep_recent:]

        compact_section = "Previous steps:\n" + "\n".join(
            f"  • {self.compact(s).summary}" for s in old_steps
        )

        recent_section = "Recent steps:\n" + self._format_full(recent_steps)

        return f"{compact_section}\n\n{recent_section}"
```

### Compaction Triggers

Compaction triggers when:

1. `len(steps) > threshold` (default: 5)
2. `total_tokens > budget` (default: 2000)
3. After every tool call (incremental)

### Storage Strategy

```
workspace/
├── history/
│   ├── step_001.json      # Full step data
│   ├── step_001.compact   # Compact representation
│   ├── step_002.json
│   └── step_002.compact
└── outputs/
    ├── search_001.json    # Offloaded search results
    ├── file_002.txt       # Offloaded file content
    └── shell_003.log      # Offloaded shell output
```

---

## 3. Isolation

> **Goal**: Each agent has independent context; no shared prompt pollution.

### Multi-Agent Context Problem

```
❌ WRONG: Shared context
┌─────────────────────────────────────────┐
│  Main Agent                             │
│  ├── Step 1: Search "AI trends"         │
│  ├── Step 2: Read report.md             │
│  └── Sub-Agent (inherits all context!)  │  ← POLLUTED
│       ├── Main's Step 1                 │
│       ├── Main's Step 2                 │
│       └── Sub's Step 1: Search "ML"     │
└─────────────────────────────────────────┘

✅ CORRECT: Isolated context
┌─────────────────────────────────────────┐
│  Main Agent                             │
│  ├── Step 1: Search "AI trends"         │
│  ├── Step 2: Spawned sub-agent          │
│  └── Step 3: Received sub result        │
└─────────────────────────────────────────┘
        │
        │ EventBus (goal only)
        ▼
┌─────────────────────────────────────────┐
│  Sub-Agent (isolated)                   │
│  └── Step 1: Search "ML"                │  ← CLEAN
└─────────────────────────────────────────┘
```

### Implementation

```python
class IsolatedContext:
    """Per-agent isolated context."""

    def __init__(self, agent_id: str, workspace_root: Path):
        self.agent_id = agent_id
        self.workspace = workspace_root / agent_id
        self.memory = WorkingMemory()
        self.steps: list[Step] = []

    def spawn_child(self, child_id: str, goal: str) -> "IsolatedContext":
        """Create isolated child context with goal only."""
        child = IsolatedContext(
            agent_id=child_id,
            workspace_root=self.workspace.parent,
        )
        # Only pass goal, not parent's steps!
        child.memory.add("system", f"Goal: {goal}")
        return child
```

### Communication Protocol

Agents communicate via EventBus, not shared memory:

```python
# Parent spawns child
await ctx.emit("agent.spawn", {
    "child_id": "researcher-1",
    "goal": "Research Bitcoin price trends",
    # NO parent context included!
})

# Child returns result
await ctx.emit("agent.result", {
    "parent_id": "main-agent",
    "result": "Bitcoin is at $45,000...",
    # Only final result, not child's full history
})
```

---

## 4. Offloading

> **Goal**: Keep heavy data and complex logic outside LLM context.

### Three Types of Offloading

#### 4.1 Data Offloading

Large outputs → workspace files:

```python
class DataOffloader:
    """Offloads heavy data to files."""

    THRESHOLD = 1000  # bytes

    def maybe_offload(self, data: str, step_id: str) -> Optional[str]:
        """Offload if data exceeds threshold."""

        if len(data) < self.THRESHOLD:
            return None

        path = f"workspace/outputs/{step_id}.json"
        write_file(path, data)
        return path
```

Prompt shows:

```
Observation: Search completed. Results saved to workspace/outputs/search_001.json
             Use fs:read_file to retrieve if needed.
```

#### 4.2 Tool Offloading

Complex operations → CLI sandbox:

```
Level 1 (LLM-facing):     Level 2 (CLI):
─────────────────────     ─────────────
fs:read_file         ←→   cat, head, tail
fs:write_file        ←→   echo, tee
web:search           ←→   curl
                          git, python, node
```

LLM orchestrates; CLI executes.

#### 4.3 Logic Offloading

Business logic → external scripts:

```python
# Instead of LLM reasoning about complex analysis:
result = await ctx.tool("python:run_script", {
    "path": "scripts/analyze_prices.py",
    "args": {"symbol": "BTC", "days": 30}
})

# LLM just interprets the result
```

Benefits:

- Stable prompts (logic not in context)
- Cacheable (same script = same result)
- Testable (unit tests for scripts)

---

## 5. Hierarchical Action Space

> **Goal**: LLM sees fewer, higher-level tools.

### Three Levels

```
┌─────────────────────────────────────────────────────────────────┐
│  Level 1: Function Tools (LLM-facing)                           │
│  ─────────────────────────────────────                          │
│  fs:read_file, fs:write_file, web:search, browser:navigate      │
│  → Simple, well-defined, JSON Schema documented                 │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│  Level 2: Shell Utilities (CLI sandbox)                         │
│  ─────────────────────────────────────                          │
│  git, curl, python, node, grep, jq, ...                         │
│  → Approved commands, sandboxed execution                       │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│  Level 3: Script APIs (external packages)                       │
│  ─────────────────────────────────────                          │
│  scripts/analyze.py, scripts/report.py, ...                     │
│  → Complex logic encapsulated, LLM just orchestrates            │
└─────────────────────────────────────────────────────────────────┘
```

### Tool Registration

```python
@dataclass
class ToolDescriptor:
    name: str
    description: str
    parameters_schema: dict  # JSON Schema
    level: int  # 1, 2, or 3
    requires_approval: bool
```

LLM prompt only shows Level 1 tools by default. Level 2/3 are accessible but not enumerated.

---

## 6. Memory Architecture

> **Goal**: Right information at right time with right persistence.

### Three Tiers

```
┌─────────────────────────────────────────────────────────────────┐
│  Working Memory (Python)                                         │
│  ─────────────────────────                                       │
│  • Current task steps                                            │
│  • Compacted history                                             │
│  • Cleared after task completion                                 │
│  • In-memory only                                                │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│  Short-term Memory (Python + Files)                              │
│  ─────────────────────────────────                               │
│  • Session-scoped (~1 hour)                                      │
│  • Offloaded outputs                                             │
│  • workspace/history/, workspace/outputs/                        │
│  • Survives task switches, not restarts                          │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│  Long-term Memory (Rust Core - RocksDB)                          │
│  ─────────────────────────────────────                           │
│  • Cross-session persistence                                     │
│  • Semantic retrieval (embeddings)                               │
│  • Agent state, learned patterns                                 │
│  • Via Bridge: ctx.memory_save(), ctx.memory_query()             │
└─────────────────────────────────────────────────────────────────┘
```

### Retrieval

```python
# Working memory: direct access
steps = context.steps[-5:]

# Short-term: file read
content = await ctx.tool("fs:read_file", {"path": "outputs/search_001.json"})

# Long-term: Bridge RPC
memories = await ctx.memory_query({
    "query": "Bitcoin price analysis",
    "limit": 5,
})
```

---

## Implementation Plan

### P0: Core (Week 1-2)

```python
# New files
loom-py/src/loom/context/
├── step.py           # Step, CompactStep dataclasses
├── reducer.py        # StepReducer with tool rules
├── compactor.py      # StepCompactor
├── offloader.py      # DataOffloader
└── tools.py          # ToolDescriptor, fetch from Bridge
```

### P1: Multi-Agent (Week 3)

```python
# New files
loom-py/src/loom/context/
├── isolation.py      # IsolatedContext
└── spawn.py          # Agent spawn/result protocol
```

### P2: Advanced (Week 4+)

```python
# New files
loom-py/src/loom/context/
├── hierarchy.py      # Hierarchical tool levels
├── scripts.py        # python:run_script tool
└── semantic.py       # Embedding-based ranking
```

---

## Integration with Cognitive Loop

```python
class CognitiveAgent:
    def __init__(self, ...):
        self.reducer = StepReducer()
        self.compactor = StepCompactor()
        self.offloader = DataOffloader(self.workspace)

    async def _execute_tool(self, tool_call: ToolCall) -> Observation:
        # Execute
        result = await self.ctx.tool(tool_call.name, tool_call.arguments)

        # Reduce
        step = self.reducer.reduce(tool_call.name, tool_call.arguments, result)

        # Maybe offload
        if step.needs_offload:
            step.outcome_ref = self.offloader.offload(result, step.id)

        # Store
        self.steps.append(step)

        return step.to_observation()

    def _build_prompt(self, goal: str) -> str:
        # Compact old steps, keep recent full
        history = self.compactor.compact_many(self.steps, keep_recent=3)

        # Build with tool descriptors
        tools = self._format_tools(self.tool_descriptors)

        return f"{self.system_prompt}\n\n{tools}\n\n{history}\n\nGoal: {goal}"
```

---

## Success Metrics

| Metric                   | Before | Target |
| ------------------------ | ------ | ------ |
| Tokens per step          | ~500   | ~100   |
| Context size (10 steps)  | ~5000  | ~1500  |
| Tool error rate          | 15%    | <5%    |
| Multi-agent context leak | 100%   | 0%     |

---

_See also:_

- `REDUCTION.md` — Detailed reduction rules
- `ISOLATION.md` — Multi-agent isolation patterns
- `OFFLOADING.md` — Data and logic offloading strategies
