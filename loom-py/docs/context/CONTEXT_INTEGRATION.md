# Context Engineering Integration Guide

## Overview

This guide explains how context engineering components integrate across the Loom stack, from tool execution to LLM prompts and UI display.

## The Complete Pipeline

```
Tool Execution
      ↓
┌─────────────────────────────────────────────────────────────┐
│ 1. StepReducer: Extract minimal representation             │
│    - Tool name + key args                                   │
│    - One-line observation                                   │
│    - Success/error status                                   │
└─────────────────────────────────────────────────────────────┘
      ↓
┌─────────────────────────────────────────────────────────────┐
│ 2. DataOffloader: Handle large outputs                     │
│    - Threshold: >2KB or >50 lines                          │
│    - Save to .loom/cache/{category}/                       │
│    - Return: preview + file_path                           │
└─────────────────────────────────────────────────────────────┘
      ↓
┌─────────────────────────────────────────────────────────────┐
│ 3. Observation: Attach reduced_step                        │
│    - observation.reduced_step = Step(...)                  │
│    - Step.outcome_ref = file_path (if offloaded)           │
└─────────────────────────────────────────────────────────────┘
      ↓
┌─────────────────────────────────────────────────────────────┐
│ 4. ThoughtStep: Store in result.steps[]                    │
│    - thought_step.reduced_step = observation.reduced_step  │
│    - Carries full chain of context                         │
└─────────────────────────────────────────────────────────────┘
      ↓
┌─────────────────────────────────────────────────────────────┐
│ 5. StepCompactor: Compress history (if >5 steps)           │
│    - Recent 3 steps: full detail                           │
│    - Older steps: grouped summaries                         │
│    - Compact format: "• 5 file operations"                  │
└─────────────────────────────────────────────────────────────┘
      ↓
┌─────────────────────────────────────────────────────────────┐
│ 6. Prompt Builder: Construct LLM input                     │
│    - Compacted history (if enabled)                        │
│    - Recent steps with offload refs                        │
│    - "Observation: (See {file_path})"                      │
└─────────────────────────────────────────────────────────────┘
      ↓
┌─────────────────────────────────────────────────────────────┐
│ 7. CLI Display: Show to user                               │
│    - Check step.reduced_step.outcome_ref                   │
│    - Display: file path + summary                          │
│    - Metrics: count offloaded outputs                      │
└─────────────────────────────────────────────────────────────┘
```

## Code Path Walkthrough

### 1. Tool Execution in CognitiveAgent

```python
# loom-py/src/loom/cognitive/agent.py:_execute_tool()

async def _execute_tool(self, tool_call: ToolCall) -> Observation:
    # Execute tool via Rust Bridge
    result = await self.ctx.tool(tool_call.name, payload=tool_call.arguments)

    # Process through context engineering
    processed_output, reduced_step = self._process_tool_result(
        tool_call=tool_call,
        raw_output=result,
        success=True,
    )

    # Create observation with reduced_step attached
    return Observation(
        tool_name=tool_call.name,
        success=True,
        output=processed_output,  # Preview if offloaded
        reduced_step=reduced_step,  # ← Key integration point!
    )
```

### 2. Processing Tool Result

```python
# loom-py/src/loom/cognitive/agent.py:_process_tool_result()

def _process_tool_result(self, tool_call, raw_output, success):
    # Step 1: Check if should offload
    offload_result = self.data_offloader.offload(
        content=raw_output,
        category=self._get_offload_category(tool_call.name),
        identifier=self._get_offload_identifier(tool_call),
    )

    # Use preview if offloaded
    output_for_observation = (
        offload_result.content if offload_result.offloaded
        else raw_output
    )

    # Step 2: Reduce to Step
    step = self.step_reducer.reduce(
        tool_name=tool_call.name,
        args=tool_call.arguments,
        result=output_for_observation,
        success=True,
    )

    # Step 3: Attach offload reference
    if offload_result.offloaded:
        step.outcome_ref = offload_result.file_path

    return output_for_observation, step
```

### 3. ReAct Loop Integration

```python
# loom-py/src/loom/cognitive/agent.py:_run_react()

for iteration in range(self.config.max_iterations):
    # Build prompt with compactor
    prompt = build_react_prompt(
        goal,
        result.steps,
        compactor=self.step_compactor,  # ← Pass compactor
        use_compaction=True,
    )

    # Get LLM response
    response = await self.llm.generate(prompt, system)
    parsed = parse_react_response(response)

    if parsed["type"] == "tool_call":
        # Execute tool
        observation = await self._execute_tool(tool_call)

        # Attach reduced_step to ThoughtStep
        step = ThoughtStep(
            step=iteration + 1,
            reasoning=parsed["thought"],
            tool_call=tool_call,
            observation=observation,
            reduced_step=observation.reduced_step,  # ← Propagate
        )

        result.steps.append(step)
```

### 4. Prompt Construction with Compaction

```python
# loom-py/src/loom/cognitive/loop.py:build_react_prompt()

def build_react_prompt(goal, steps, compactor, use_compaction=True):
    if use_compaction and compactor and len(steps) > 5:
        # Extract reduced steps
        reduced_steps = [s.reduced_step for s in steps if s.reduced_step]

        if reduced_steps:
            # Compact the history
            history = compactor.compact(reduced_steps)

            # Show compacted steps
            if history.compact_steps:
                parts.append("Previous actions (summarized):")
                for compact_step in history.compact_steps:
                    parts.append(f"  {compact_step}")

            # Show recent steps with offload refs
            for step in steps[-3:]:  # Last 3 steps
                if step.reduced_step and step.reduced_step.outcome_ref:
                    # Show file reference instead of content
                    parts.append(
                        f"Observation: (See {step.reduced_step.outcome_ref})"
                    )
                else:
                    parts.append(f"Observation: {step.observation.output}")
```

### 5. CLI Display

```python
# loom-py/src/loom/cli/chat.py:print_stream_step_complete()

def print_stream_step_complete(step):
    if step.tool_call and step.observation:
        # Check if data was offloaded
        if step.reduced_step and step.reduced_step.outcome_ref:
            # Show offload reference
            print(f"📄 Data offloaded to: {step.reduced_step.outcome_ref}")
            print(f"💡 Summary: {step.reduced_step.observation[:100]}")
        else:
            # Show normal output
            print(f"✅ Result:")
            for line in step.observation.output.split("\n")[:8]:
                print(f"   {line}")
```

## Key Integration Points

### 1. Observation → ThoughtStep Propagation

```python
# CRITICAL: Must propagate reduced_step from Observation to ThoughtStep

observation = await self._execute_tool(tool_call)
step = ThoughtStep(
    ...,
    observation=observation,
    reduced_step=observation.reduced_step,  # ← Must attach!
)
```

### 2. Step.observation vs Step.outcome

```python
# CORRECT: Step has 'observation' attribute
step.observation  # ✅ One-line summary

# WRONG: Step does NOT have 'outcome'
step.outcome  # ❌ AttributeError!
```

### 3. Compactor Requires Reduced Steps

```python
# Prompt builder needs reduced steps for compaction
reduced_steps = [s.reduced_step for s in steps if s.reduced_step]

if reduced_steps:
    history = compactor.compact(reduced_steps)
    # Build compacted prompt...
```

### 4. CLI Checks outcome_ref

```python
# CLI must check for offload reference
if step.reduced_step and step.reduced_step.outcome_ref:
    # Show file path
else:
    # Show output
```

## Testing Integration

### Unit Tests

```python
# tests/unit/test_step.py
def test_step_has_observation_not_outcome():
    step = Step(...)
    assert hasattr(step, "observation")  # ✅
    assert not hasattr(step, "outcome")  # ❌

# tests/unit/test_cognitive.py
def test_observation_carries_reduced_step():
    reduced = Step(...)
    obs = Observation(..., reduced_step=reduced)
    assert obs.reduced_step is not None
```

### Integration Tests

```python
# tests/integration/test_context_engineering.py

def test_display_offloaded_step(capsys):
    """Verify CLI displays offload references correctly."""
    reduced_step = Step(..., outcome_ref=".loom/cache/result.json")
    step = ThoughtStep(..., reduced_step=reduced_step)

    print_stream_step_complete(step)
    captured = capsys.readouterr()

    assert "Data offloaded" in captured.out
    assert ".loom/cache/result.json" in captured.out

def test_full_pipeline_with_offloading(tmp_path):
    """Test complete flow: reducer -> offloader -> compactor -> prompt."""
    reducer = StepReducer()
    offloader = DataOffloader(tmp_path, ...)
    compactor = StepCompactor()

    # Offload large output
    offload_result = offloader.offload(large_content, ...)

    # Reduce to Step
    reduced_step = reducer.reduce(...)
    reduced_step.outcome_ref = offload_result.file_path

    # Create ThoughtStep
    thought_step = ThoughtStep(..., reduced_step=reduced_step)

    # Build prompt
    prompt = build_react_prompt(..., compactor=compactor)

    # Verify preview is shown, not full output
    assert len(prompt) < len(large_content)
```
## Performance Impact

### Token Savings

**Without Context Engineering**:

```
Prompt size: ~8,000 tokens (10 tool calls with full outputs)
```

**With Context Engineering**:

```
- Offloading: 3 large outputs → files
- Compaction: 7 old steps → 2 summaries
- Recent 3 steps: full detail

Prompt size: ~2,500 tokens (68% reduction)
```

### Measured Metrics

From production usage:

- **Offload rate**: 40% of tool outputs (web search, file reads)
- **Compression ratio**: 3.2x for compacted steps
- **Token reduction**: 29.4% average across conversations
- **No impact on task completion**: Identical iteration counts

## See Also

- [Context Engineering Design](context/DESIGN.md)
- [Reduction Strategies](context/REDUCTION.md)
- [Offloading Guide](context/OFFLOADING.md)
- [CLI Guide](CLI_GUIDE.md)
- [Cognitive Guide](COGNITIVE_GUIDE.md)
