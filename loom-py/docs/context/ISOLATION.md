# Multi-Agent Context Isolation

This document details the context isolation strategy for Loom's multi-agent scenarios.

---

## The Problem

Without isolation, multi-agent systems fail:

```
❌ Shared Context Problem:

Main Agent context:
├── User goal: "Research AI trends"
├── Step 1: Searched "AI 2024"
├── Step 2: Read 5 articles
├── Step 3: Spawned Researcher-1
│   └── Researcher-1 inherits ALL above context
│       └── Step 4: Searches "ML frameworks"
│       └── Step 5: BUT context already polluted with Main's steps
│
└── Result: Researcher confused, duplicates work, wrong focus
```

The fundamental issue: **parent context leaks into child**.

---

## The Solution: Isolation by Design

> **"Don't communicate by sharing memory; share memory by communicating."**

```
✅ Isolated Context:

Main Agent:
├── Own workspace: workspace/main/
├── Own memory: steps[], working_memory
└── Communicates via EventBus only

Researcher-1:
├── Own workspace: workspace/researcher-1/
├── Own memory: starts EMPTY
├── Receives: ONLY the goal string
└── Returns: ONLY the result string

Communication:
Main → EventBus → Researcher-1: "Research ML frameworks"
Researcher-1 → EventBus → Main: "Found: PyTorch, TensorFlow..."
```

---

## Architecture

### IsolatedContext Class

```python
@dataclass
class IsolatedContext:
    """Per-agent isolated context."""

    agent_id: str
    workspace: Path
    parent_id: Optional[str] = None

    # Isolated state
    steps: list[Step] = field(default_factory=list)
    working_memory: WorkingMemory = field(default_factory=WorkingMemory)
    compactor: StepCompactor = field(default_factory=StepCompactor)
    offloader: DataOffloader = None

    def __post_init__(self):
        # Create isolated workspace
        self.workspace.mkdir(parents=True, exist_ok=True)
        (self.workspace / "history").mkdir(exist_ok=True)
        (self.workspace / "outputs").mkdir(exist_ok=True)
        self.offloader = DataOffloader(self.workspace)

    def spawn_child(self, child_id: str, goal: str) -> "IsolatedContext":
        """Create child with ONLY the goal. No context inheritance."""

        child_workspace = self.workspace.parent / child_id
        child = IsolatedContext(
            agent_id=child_id,
            workspace=child_workspace,
            parent_id=self.agent_id,
        )

        # Child gets ONLY the goal
        child.working_memory.add("system", f"Your task: {goal}")

        # Parent records the spawn
        self.steps.append(Step(
            id=self._next_step_id(),
            tool_name="agent:spawn",
            minimal_args={"child_id": child_id},
            observation=f"Spawned {child_id} with goal: {goal[:50]}...",
            outcome_ref=None,
            success=True,
        ))

        return child
```

### Workspace Structure

```
workspace/
├── main-agent/
│   ├── history/
│   │   ├── step_001.json
│   │   └── step_002.json
│   └── outputs/
│       └── search_001.json
│
├── researcher-1/          # Completely isolated
│   ├── history/
│   │   └── step_001.json  # Fresh start
│   └── outputs/
│       └── page_001.html
│
└── researcher-2/          # Completely isolated
    ├── history/
    └── outputs/
```

---

## Communication Protocol

### Spawning an Agent

```python
# In Main Agent's cognitive loop

# 1. Create spawn event
spawn_request = {
    "child_id": f"researcher-{uuid4().hex[:8]}",
    "goal": "Research ML frameworks and summarize top 3",
    "timeout_ms": 60000,
}

# 2. Emit via EventBus (NOT direct context sharing)
await ctx.emit("agent.spawn", type="spawn_request", payload=spawn_request)

# 3. Record in own history
self.steps.append(Step(
    tool_name="agent:spawn",
    observation=f"Spawned researcher to: {spawn_request['goal'][:50]}",
))

# 4. Wait for result (async)
result = await ctx.wait_for("agent.result", correlation_id=spawn_request["child_id"])
```

### Returning Results

```python
# In Child Agent's cognitive loop

# 1. Complete the task
final_answer = "Top 3 ML frameworks: 1. PyTorch 2. TensorFlow 3. JAX"

# 2. Emit result (NOT full history)
await ctx.emit("agent.result", type="spawn_result", payload={
    "parent_id": self.parent_id,
    "child_id": self.agent_id,
    "result": final_answer,  # ONLY the answer
    "success": True,
})

# Child's full history stays in child's workspace, not sent to parent
```

### What Gets Shared

| Data            | Shared? | Notes                   |
| --------------- | ------- | ----------------------- |
| Goal string     | ✅      | Parent → Child          |
| Final result    | ✅      | Child → Parent          |
| Step history    | ❌      | Never crosses boundary  |
| Working memory  | ❌      | Per-agent only          |
| Workspace files | ❌      | Isolated directories    |
| Tool outputs    | ❌      | In agent's own outputs/ |

---

## Goal-Only Prompting

### What Child Receives

```
❌ WRONG - Context inheritance:

System: You are a helpful assistant.

Previous context:
- User asked about AI trends
- Searched "AI 2024 trends"
- Read article about GPT-5
- ...100 more lines of parent's history...

Your task: Research ML frameworks

→ Child is confused by irrelevant context
```

```
✅ CORRECT - Goal-only:

System: You are a research assistant with access to web search and file tools.

Your task: Research ML frameworks and summarize the top 3 options.

Available tools: web:search, fs:read_file, fs:write_file

→ Child has clean, focused context
```

### Implementation

```python
class IsolatedContext:
    def create_child_prompt(self, goal: str, tools: list[str]) -> str:
        """Create minimal prompt for child agent."""

        # NO parent history
        # NO parent's tool outputs
        # ONLY goal + available tools

        return f"""You are a focused research assistant.

Your task: {goal}

Available tools:
{self._format_tools(tools)}

Complete the task and provide a clear summary."""
```

---

## Aggregation Pattern

For multi-child scenarios (e.g., research with multiple researchers):

```python
class ResearchOrchestrator:
    """Coordinates multiple isolated research agents."""

    async def research(self, topic: str, num_researchers: int = 3) -> str:
        # 1. Spawn multiple isolated researchers
        children = []
        for i in range(num_researchers):
            child = self.context.spawn_child(
                child_id=f"researcher-{i}",
                goal=f"Research aspect {i+1} of: {topic}"
            )
            children.append(child)

        # 2. Wait for all results (parallel)
        results = await asyncio.gather(*[
            self._wait_for_child(c.agent_id) for c in children
        ])

        # 3. Aggregate in parent's context
        combined = "\n\n".join([
            f"Researcher {i+1}: {r}" for i, r in enumerate(results)
        ])

        # 4. Parent synthesizes (with ITS context, not children's)
        return await self.cognitive.run(f"Synthesize these findings:\n{combined}")
```

### Aggregation Rules

1. **Wait for all or timeout** - Don't mix complete/incomplete results
2. **Deduplicate** - Children may find same sources
3. **Preserve attribution** - Track which child found what
4. **Parent synthesizes** - Final synthesis uses parent's context only

---

## Resource Limits

Each isolated agent has limits:

```python
@dataclass
class AgentLimits:
    max_steps: int = 20
    max_tokens_per_step: int = 2000
    max_output_files: int = 10
    max_output_size_bytes: int = 1_000_000  # 1MB
    timeout_ms: int = 60_000

class IsolatedContext:
    def __init__(self, ..., limits: AgentLimits = None):
        self.limits = limits or AgentLimits()

    def check_limits(self):
        if len(self.steps) >= self.limits.max_steps:
            raise AgentLimitExceeded("max_steps")

        output_size = sum(
            f.stat().st_size for f in (self.workspace / "outputs").iterdir()
        )
        if output_size >= self.limits.max_output_size_bytes:
            raise AgentLimitExceeded("max_output_size")
```

---

## EventBus Integration

### Event Types

```protobuf
// In loom-proto/proto/agent.proto

message SpawnRequest {
    string parent_id = 1;
    string child_id = 2;
    string goal = 3;
    repeated string tools = 4;
    int64 timeout_ms = 5;
}

message SpawnResult {
    string parent_id = 1;
    string child_id = 2;
    bool success = 3;
    string result = 4;
    string error = 5;
}
```

### Rust Core Handling

```rust
// Bridge receives spawn request, creates new agent context
async fn handle_spawn(&self, req: SpawnRequest) {
    // 1. Create isolated agent in directory
    self.agent_directory.register_agent(AgentInfo {
        agent_id: req.child_id.clone(),
        parent_id: Some(req.parent_id.clone()),
        // No shared context!
    });

    // 2. Forward to Python runtime
    self.event_bus.publish("agent.spawn.execute", req).await;
}
```

---

## Testing Isolation

```python
def test_context_isolation():
    """Verify child context is truly isolated."""

    # Parent does some work
    parent = IsolatedContext(agent_id="parent", workspace=Path("/tmp/parent"))
    parent.steps.append(Step(tool_name="web:search", ...))
    parent.working_memory.add("user", "Parent's query")

    # Spawn child
    child = parent.spawn_child("child", "Research something")

    # Verify isolation
    assert len(child.steps) == 0  # No inherited steps
    assert len(child.working_memory) == 1  # Only the goal
    assert child.workspace != parent.workspace  # Different directories

    # Child cannot access parent's files
    assert not (child.workspace / "outputs" / "search_001.json").exists()

def test_result_only_communication():
    """Verify only result crosses boundary."""

    parent = IsolatedContext(...)
    child = parent.spawn_child("child", "Research X")

    # Child does work
    child.steps.append(Step(...))
    child.steps.append(Step(...))
    child.steps.append(Step(...))

    # Child returns result
    result = "Found: X, Y, Z"

    # Parent receives ONLY result
    parent_received = result  # NOT child.steps

    assert "Step" not in parent_received
    assert len(parent.steps) == 1  # Only spawn step, not child's steps
```

---

## Best Practices

### Do

- ✅ Give child a **clear, specific goal**
- ✅ Limit available tools to what's needed
- ✅ Set appropriate timeouts
- ✅ Handle child failures gracefully
- ✅ Aggregate results before synthesis

### Don't

- ❌ Share working memory between agents
- ❌ Let child access parent's output files
- ❌ Pass "context" along with goal
- ❌ Expect child to know parent's history
- ❌ Mix child's steps into parent's prompt

---

_See also: `DESIGN.md` for overall architecture_
