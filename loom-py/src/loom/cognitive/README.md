# Cognitive Module

Autonomous perceive-think-act cognitive loop for LLM-powered agents.

## Overview

The Cognitive module implements **autonomous agent reasoning** using the cognitive loop pattern:

1. **Perceive**: Gather context from events, memory, and environment
2. **Think**: Use LLM to reason and decide on actions
3. **Act**: Execute tools and produce outputs

This is the **"Brain"** in Loom's Brain/Hand separation.

## Architecture

```
┌──────────────────────────────────────────────────────────┐
│              CognitiveAgent (agent.py)                   │
│                                                           │
│  ┌─────────────┐  ┌──────────────┐  ┌───────────────┐  │
│  │  Perceive   │→ │    Think     │→ │     Act       │  │
│  │  (Context)  │  │  (LLM Call)  │  │  (Tools)      │  │
│  └─────────────┘  └──────────────┘  └───────────────┘  │
│         ↓                 ↓                  ↓           │
│  ┌─────────────────────────────────────────────────┐    │
│  │         Context Engineering Pipeline            │    │
│  │  • StepReducer: Summarize tool outputs          │    │
│  │  • StepCompactor: Compress history              │    │
│  │  • DataOffloader: Save large outputs            │    │
│  └─────────────────────────────────────────────────┘    │
│         ↓                 ↓                  ↓           │
│  ┌──────────────────────────────────────────────────┐   │
│  │            ToolExecutor (executor.py)            │   │
│  │  • Human-in-the-loop approval                    │   │
│  │  • Security validation                           │   │
│  │  • Result processing                             │   │
│  └──────────────────────────────────────────────────┘   │
│         ↓                 ↓                  ↓           │
│  ┌──────────────────────────────────────────────────┐   │
│  │       StrategyExecutor (strategies.py)           │   │
│  │  • ReAct: Iterative Thought→Action→Observation   │   │
│  │  • CoT: Chain of Thought reasoning               │   │
│  │  • Single-shot: One LLM call                     │   │
│  └──────────────────────────────────────────────────┘   │
└──────────────────────────────────────────────────────────┘
         ↓                               ↓
    LLMProvider                     EventContext
    (direct HTTP)                   (tool calls)
```

## Refactored Structure (v0.3.0)

Previously: Single 935-line `agent.py` file
Now: Three focused modules (1057 lines total)

- **`agent.py`** (289 lines): Main coordinator
- **`executor.py`** (447 lines): Tool execution & approval
- **`strategies.py`** (321 lines): Thinking strategies

**Benefits:**

- ✅ Clear separation of concerns
- ✅ Easier testing of individual components
- ✅ Simplified extensibility

## Key Components

### CognitiveAgent (`agent.py`)

Main coordinator class:

```python
from loom import Agent
from loom.cognitive import CognitiveAgent, CognitiveConfig, ThinkingStrategy
from loom.llm import LLMProvider

# Setup agent
agent = Agent(agent_id="researcher", topics=["tasks"])
await agent.start()

# Create cognitive agent
cognitive = CognitiveAgent(
    ctx=agent.ctx,  # EventContext for tool calls
    llm=LLMProvider.from_name(agent.ctx, "deepseek"),
    config=CognitiveConfig(
        system_prompt="You are a research assistant...",
        max_iterations=10,
        thinking_strategy=ThinkingStrategy.REACT,
    ),
    available_tools=["web:search", "fs:write_file"],
    workspace_path="/workspace",
)

# Run cognitive loop
result = await cognitive.run("Research AI frameworks")
print(result.answer)
```

**Key Methods:**

- `run(goal)`: Execute cognitive loop (blocking)
- `run_stream(goal)`: Execute with streaming output
- `register_tool()`: Add tool to registry

### ToolExecutor (`executor.py`)

Handles tool execution with approval and security:

```python
# Internal usage (by CognitiveAgent)
executor = ToolExecutor(
    ctx=agent.ctx,
    step_reducer=StepReducer(),
    data_offloader=DataOffloader(workspace),
    permission_callback=approval_callback,
)

observation = await executor.execute_tool(tool_call)
```

**Features:**

- **Human-in-the-loop**: Approval for destructive tools (fs:write, fs:delete)
- **Security**: Path traversal prevention, input validation
- **Context Engineering**: Automatic reduction and offloading
- **Error Handling**: Retry logic, timeout management

### StrategyExecutor (`strategies.py`)

Implements thinking strategies:

```python
# Internal usage (by CognitiveAgent)
strategy = StrategyExecutor(
    llm=llm_provider,
    tool_executor=tool_executor,
    memory=WorkingMemory(),
    config=config,
)

result = await strategy.run_react(goal)  # ReAct pattern
result = await strategy.run_cot(goal)    # Chain of Thought
result = await strategy.run_single_shot(goal)  # Single call
```

**Strategies:**

1. **ReAct**: Iterative reasoning with tool use
2. **CoT**: Multi-step reasoning without tools
3. **Single-shot**: One LLM call, no iteration

## Thinking Strategies

### ReAct (Recommended)

**Pattern**: Thought → Action → Observation (repeat)

```python
config = CognitiveConfig(thinking_strategy=ThinkingStrategy.REACT)
```

**Example:**

```
Iteration 1:
  Thought: I need to search for information
  Action: web:search(query="Loom framework")
  Observation: Found 10 results...

Iteration 2:
  Thought: Let me fetch the first article
  Action: web:fetch(url="https://...")
  Observation: Article content...

Iteration 3:
  Thought: Now I have enough information
  Final Answer: Loom is...
```

### Chain of Thought (CoT)

**Pattern**: Reason step-by-step without tools

```python
config = CognitiveConfig(thinking_strategy=ThinkingStrategy.CHAIN_OF_THOUGHT)
```

**Example:**

```
Step 1: First, let's break down the problem
Step 2: Consider the constraints
Step 3: Evaluate options
Final Answer: Based on the reasoning...
```

### Single-shot

**Pattern**: One LLM call, direct answer

```python
config = CognitiveConfig(thinking_strategy=ThinkingStrategy.SINGLE_SHOT)
```

**Example:**

```
Question: What is 2+2?
Answer: 4
```

## Context Engineering Integration

The cognitive module fully integrates with context engineering:

```python
# Automatic pipeline inside CognitiveAgent
from loom.context import StepReducer, StepCompactor, DataOffloader

# 1. Reduce: Summarize tool outputs
reducer = StepReducer()
step = reducer.reduce(tool_call, result)
# "Read 1000-line file" → "Read config.py (1000 lines)"

# 2. Offload: Save large outputs
offloader = DataOffloader(workspace)
if step.needs_offload:
    step.ref = offloader.offload(result)
    # → .loom-context/offload-123.txt

# 3. Compact: Compress history
compactor = StepCompactor()
history = compactor.compact_many(steps)
# 20 steps → 8 compact representations
```

**Benefits:**

- Token savings: 40-60% reduction
- Stays within context limits
- Preserves key information

## Streaming Support

Stream LLM reasoning in real-time:

```python
async for item in cognitive.run_stream(goal):
    if isinstance(item, str):
        # LLM chunk
        print(item, end="", flush=True)
    elif isinstance(item, ThoughtStep):
        # Completed thought/action/observation
        print(f"\n🔧 {item.tool_call.name}")
        print(f"✅ {item.observation.output}")
    elif isinstance(item, CognitiveResult):
        # Final result
        print(f"\n💡 {item.answer}")
```

**Use Cases:**

- CLI chat interface
- Web UI with live updates
- Progress monitoring

## Human-in-the-Loop

Approve destructive operations:

```python
async def approval_callback(tool_name, args, error_msg):
    """Ask user for approval."""
    print(f"⚠️  {tool_name} requires approval")
    print(f"Arguments: {args}")
    response = input("Approve? [y/N]: ")
    return response.lower() == 'y'

cognitive = CognitiveAgent(
    ctx=agent.ctx,
    llm=llm,
    permission_callback=approval_callback,
)
```

**Protected Tools:**

- `fs:write_file` - Can overwrite files
- `fs:delete` - Can delete files

## Tool Registry

Register tools for enhanced descriptions:

```python
from loom.context import ToolRegistry

registry = ToolRegistry()
registry.register(
    name="web:search",
    description="Search the web for information",
    parameters=[
        {"name": "query", "type": "string", "description": "Search query"}
    ],
    examples=["web:search(query='AI frameworks')"],
)

cognitive = CognitiveAgent(
    ctx=agent.ctx,
    llm=llm,
    tool_registry=registry,
)
```

## Configuration

### CognitiveConfig

```python
from loom.cognitive import CognitiveConfig, ThinkingStrategy

config = CognitiveConfig(
    system_prompt="You are a helpful assistant",
    max_iterations=10,           # Max ReAct iterations
    thinking_strategy=ThinkingStrategy.REACT,
    require_final_answer=True,   # Must end with "FINAL ANSWER:"
    reflection_enabled=False,    # Self-reflection (future)
)
```

### WorkingMemory

Short-term memory for conversation:

```python
from loom.cognitive import WorkingMemory

memory = WorkingMemory(max_items=50)
memory.add("user", "What is Loom?")
memory.add("assistant", "Loom is an AI OS...")

# Get recent context
context = memory.get_context(max_items=10)

# Convert to messages
messages = memory.to_messages()
```

## Testing

```bash
# Run cognitive tests
pytest tests/unit/test_cognitive.py -v

# 24 tests covering:
# - ThoughtStep integration
# - CognitiveAgent.run()
# - CognitiveAgent.run_stream()
# - Cognitive types
# - Config validation
# - Memory management
```

## Examples

### Research Agent

```python
cognitive = CognitiveAgent(
    ctx=agent.ctx,
    llm=LLMProvider.from_name(agent.ctx, "deepseek"),
    config=CognitiveConfig(
        system_prompt="You are a research assistant.",
        max_iterations=10,
    ),
    available_tools=["web:search", "web:fetch", "fs:write_file"],
)

result = await cognitive.run(
    "Research the latest AI frameworks and create a summary"
)
```

### Code Analyzer

```python
cognitive = CognitiveAgent(
    ctx=agent.ctx,
    llm=LLMProvider.from_name(agent.ctx, "deepseek"),
    config=CognitiveConfig(
        system_prompt="You are a code analysis expert.",
        max_iterations=5,
    ),
    available_tools=["fs:read_file", "fs:list_dir"],
    workspace_path="/path/to/project",
)

result = await cognitive.run(
    "Analyze the project structure and identify main components"
)
```

### Trading Agent

```python
cognitive = CognitiveAgent(
    ctx=agent.ctx,
    llm=LLMProvider.from_name(agent.ctx, "deepseek"),
    config=CognitiveConfig(
        system_prompt="You are a trading strategist.",
        max_iterations=3,
    ),
    available_tools=["market:get_price", "market:place_order"],
)

result = await cognitive.run(
    "Analyze BTC price and suggest a trading strategy"
)

# Save plan to Core memory
await agent.ctx.save_plan(
    symbol="BTC",
    action="BUY",
    confidence=result.confidence,
    reasoning=result.answer,
)
```

## Performance

### Token Optimization

With context engineering:

- **Before**: 8000 tokens (exceeds 4K limit)
- **After**: 2000 tokens (75% reduction)

### Latency

- **Non-streaming**: 5-10s for full response
- **Streaming**: 0.1-0.5s first token, real-time chunks

### Cost

DeepSeek pricing (example):

- Input: $0.001 per 1K tokens
- Output: $0.002 per 1K tokens

Typical cognitive run:

- Input: 2000 tokens = $0.002
- Output: 500 tokens = $0.001
- **Total: $0.003 per run**

## Troubleshooting

### Max Iterations Exceeded

```
Problem: Agent hits max_iterations without final answer
Solution:
1. Increase max_iterations: CognitiveConfig(max_iterations=20)
2. Simplify goal/task
3. Check tool availability
```

### Tool Call Failures

```
Problem: Tools timing out or failing
Solution:
1. Increase timeout: ctx.tool(..., timeout_ms=30000)
2. Check Bridge is running: loom up
3. Verify tool is registered
```

### Context Window Overflow

```
Problem: "Token limit exceeded"
Solution:
1. Enable offloading (automatic)
2. Reduce max_iterations
3. Use more aggressive compaction
```

## Migration from v0.2.x

### Before (Single agent.py)

```python
from loom.cognitive import CognitiveAgent

# All logic in one file
```

### After (v0.3.0+)

```python
from loom.cognitive import CognitiveAgent

# Same API! Refactoring is internal only
# ToolExecutor and StrategyExecutor are internal
```

**No breaking changes** - public API unchanged.

## Related Documentation

- **[COGNITIVE_GUIDE.md](../../../docs/COGNITIVE_GUIDE.md)**: Detailed patterns
- **[ARCHITECTURE.md](../../../docs/cognitive/ARCHITECTURE.md)**: Internal design
- **[Context Engineering](../context/README.md)**: Token optimization
- **[LLM Module](../llm/README.md)**: Provider configuration

---

**Key Insight**: The cognitive module transforms LLMs from passive question-answerers into **autonomous problem-solvers** that can perceive, reason, and act in complex environments.
