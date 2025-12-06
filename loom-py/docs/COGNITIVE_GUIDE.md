# Cognitive Loop Guide

Describes Loom's Python cognitive loop: ReAct-style reasoning, tool use via the Rust Bridge, context engineering, and memory patterns.

## Architecture Overview

The cognitive module is organized into three focused components:

```
┌─────────────────────────────────────────────────────────────────┐
│                      CognitiveAgent                              │
│  • High-level coordination                                      │
│  • Configuration management                                     │
│  • Public API                                                   │
├─────────────────────────────────────────────────────────────────┤
│                    StrategyExecutor                              │
│  • Single-shot: One LLM call                                    │
│  • ReAct: Iterative reasoning + action                          │
│  • Chain-of-Thought: Step-by-step reasoning                     │
├─────────────────────────────────────────────────────────────────┤
│                      ToolExecutor                                │
│  • Tool execution via Rust bridge                               │
│  • Human-in-the-loop approval                                   │
│  • Result processing & data offloading                          │
└─────────────────────────────────────────────────────────────────┘
```

## Loop Structure

- Receive user intent or events
- Reason (plan next action)
- Act (call tools/capabilities)
- Observe results
- Iterate until objective is met

## Basic Pattern

```python
from loom import Agent
from loom.cognitive import CognitiveAgent, CognitiveConfig
from loom.llm import LLMProvider

# Initialize agent with Rust context
agent = Agent(agent_id="assistant", topics=["tasks"])
await agent.start()

# Create cognitive agent with LLM
cognitive = CognitiveAgent(
    ctx=agent._ctx,
    llm=LLMProvider.from_name(agent._ctx, "deepseek"),
    config=CognitiveConfig(
        system_prompt="You are a helpful assistant...",
        max_iterations=5,
    ),
    available_tools=["fs:read_file", "fs:write_file", "system:shell"],
)

# Run cognitive loop
result = await cognitive.run("What files are in the current directory?")
print(result.answer)
```

## Tool Use via Bridge

Tools are registered in the Rust Core and called through the Bridge. The `ToolExecutor` handles execution with optional human-in-the-loop approval for destructive operations.

```python
# Tools are automatically executed via ToolExecutor
# Destructive operations (fs:write_file, fs:delete) require approval

# Example: Custom permission callback
async def permission_callback(tool_name, args, error_msg):
    """Called when approval is needed for destructive operations."""
    print(f"⚠️  {tool_name} requires approval: {error_msg}")
    response = input("Approve? (y/n): ")
    return response.lower() == 'y'

cognitive = CognitiveAgent(
    ctx=agent._ctx,
    llm=llm,
    permission_callback=permission_callback,
)
```

### Available Tools

| Tool            | Description                | Arguments                                 | Approval Required |
| --------------- | -------------------------- | ----------------------------------------- | ----------------- |
| `weather:get`   | Get weather for a location | `{"location": "city"}`                    | No                |
| `system:shell`  | Run allowed shell commands | `{"command": "ls"}` (ls, echo, cat, grep) | No                |
| `fs:read_file`  | Read file contents         | `{"path": "relative/path"}`               | No                |
| `fs:write_file` | Write content to file      | `{"path": "path", "content": "text"}`     | **Yes**           |
| `fs:list_dir`   | List directory             | `{"path": "path"}` (optional)             | No                |
| `fs:delete`     | Delete file or empty dir   | `{"path": "path"}`                        | **Yes**           |

Tools execute in the Rust sandbox through the Loom Bridge, keeping system operations safe and isolated from the Python brain.

### Streaming Support

Stream the cognitive process in real-time:

```python
async for item in cognitive.run_stream("Analyze this codebase"):
    if isinstance(item, str):
        # LLM response chunks
        print(item, end="", flush=True)
    elif isinstance(item, ThoughtStep):
        # Complete thought/action/observation step
        print(f"\n🧠 Thought: {item.reasoning}")
        if item.tool_call:
            print(f"🔧 Action: {item.tool_call.name}")
            print(f"👁️  Observation: {item.observation.output}")
    elif isinstance(item, CognitiveResult):
        # Final result
        print(f"\n✅ Answer: {item.answer}")
```

## Thinking Strategies

The `StrategyExecutor` implements three reasoning patterns:

### 1. ReAct (Reasoning + Acting)

Default strategy. Iteratively reasons about the problem and takes actions.

```python
config = CognitiveConfig(
    thinking_strategy=ThinkingStrategy.REACT,
    max_iterations=10,
)
```

**Flow:**

1. **Thought**: Reason about what to do next
2. **Action**: Call a tool or provide final answer
3. **Observation**: Process tool results
4. Repeat until solved or max iterations

### 2. Chain-of-Thought (CoT)

Step-by-step reasoning without tool use.

```python
config = CognitiveConfig(
    thinking_strategy=ThinkingStrategy.CHAIN_OF_THOUGHT,
)
```

Best for: Mathematical reasoning, logical deduction, planning

### 3. Single-Shot

One LLM call, no iteration or tools.

```python
config = CognitiveConfig(
    thinking_strategy=ThinkingStrategy.SINGLE_SHOT,
)
```

Best for: Simple questions, summarization, quick responses

## Context Engineering

The cognitive agent includes advanced context management to handle long conversations and large tool outputs:

### Data Offloading

Large tool outputs are automatically saved to files and referenced instead of kept in context:

```python
# Automatic offloading for outputs > 2KB or > 50 lines
cognitive = CognitiveAgent(
    ctx=agent._ctx,
    llm=llm,
    workspace_path="/path/to/workspace",  # Where offloaded data is saved
)

# When a tool returns large output:
# Instead of: "Observation: [5000 lines of code...]"
# Uses: "Observation: (Data saved to .loom/offload/file_read/example.py)"
```

### Step Reduction

Tool execution steps are intelligently summarized to conserve tokens:

```python
# Before reduction:
{
    "tool": "fs:read_file",
    "args": {"path": "long_file.py"},
    "result": "[Full 1000-line file content...]"
}

# After reduction:
{
    "tool": "fs:read_file",
    "args": {"path": "long_file.py"},
    "summary": "Read 1000 lines from long_file.py",
    "outcome_ref": ".loom/offload/file_read/long_file.py"
}
```

### Step Compaction

Historical steps are compacted when building prompts to stay within token limits:

```python
# Recent steps: Full detail
# Older steps: Compacted to summaries
# Ancient steps: Dropped from context

# This happens automatically in ReAct iterations
```

### Working Memory

Maintains conversation history within the current task:

```python
# Access memory directly if needed
cognitive.memory.add("user", "What's the weather?")
cognitive.memory.add("assistant", "Let me check...")

messages = cognitive.memory.get_messages()
cognitive.memory.clear()  # Reset for new task
```

## Tool Registry

Register custom tools with detailed descriptors for better LLM understanding:

```python
from loom.context import ToolParameter

cognitive.register_tool(
    name="custom:analyze",
    description="Analyze code quality and suggest improvements",
    parameters=[
        ToolParameter(
            name="file_path",
            type="string",
            required=True,
            description="Path to the file to analyze",
        ),
        ToolParameter(
            name="checks",
            type="array",
            required=False,
            description="List of checks to run: ['style', 'performance', 'security']",
        ),
    ],
    examples=[
        '{"file_path": "src/main.py", "checks": ["style", "security"]}',
        '{"file_path": "lib/utils.py"}',
    ],
    category="code_analysis",
)
```

## Configuration Options

```python
config = CognitiveConfig(
    # System prompt for the agent
    system_prompt="You are an expert programmer...",

    # Thinking strategy
    thinking_strategy=ThinkingStrategy.REACT,

    # Max iterations for ReAct
    max_iterations=10,

    # LLM temperature
    temperature=0.7,
)
```

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
