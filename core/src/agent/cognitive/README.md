# Cognitive Loop Module

The cognitive loop module provides a structured **Perceive → Think → Act** pattern for building intelligent agents with LLM-powered reasoning.

## Overview

This module sits on top of the existing `AgentRuntime` and `AgentBehavior` abstractions, providing an opt-in cognitive architecture. Agents can continue to use the simple `AgentBehavior` trait for reactive behavior, or adopt the `CognitiveLoop` pattern for more sophisticated reasoning.

```
                    ┌─────────────────────────────────────────────────┐
                    │               COGNITIVE LOOP                     │
                    │                                                  │
   Event ──────▶   │  ┌──────────┐   ┌──────────┐   ┌──────────┐     │
                    │  │ PERCEIVE │──▶│  THINK   │──▶│   ACT    │──────────▶ Actions
                    │  │          │   │          │   │          │     │
                    │  │ Context  │   │ LLM +    │   │ Execute  │     │
                    │  │ Builder  │   │ Planning │   │ Tools    │     │
                    │  └──────────┘   └──────────┘   └──────────┘     │
                    │        │              │              │          │
                    │        ▼              ▼              ▼          │
                    │  ┌──────────────────────────────────────────┐   │
                    │  │              WORKING MEMORY               │   │
                    │  └──────────────────────────────────────────┘   │
                    │                       │                         │
                    │                       ▼                         │
                    │              ┌──────────────┐                   │
                    │              │   REFLECT    │ (optional)        │
                    │              └──────────────┘                   │
                    └─────────────────────────────────────────────────┘
```

## Key Components

### CognitiveConfig

Configuration for cognitive agents:

```rust
use loom_core::agent::cognitive::{CognitiveConfig, ThinkingStrategy};

// Quick presets
let config = CognitiveConfig::single_shot();  // One LLM call, no tools
let config = CognitiveConfig::react();         // ReAct pattern with tools
let config = CognitiveConfig::chain_of_thought(); // Step-by-step reasoning

// Custom configuration
let config = CognitiveConfig::react()
    .with_system_prompt("You are a helpful trading assistant")
    .with_max_iterations(10)
    .with_reflection()
    .with_memory_window(50);
```

**Configuration Options:**

| Option               | Default      | Description                                |
| -------------------- | ------------ | ------------------------------------------ |
| `thinking_strategy`  | `SingleShot` | `SingleShot`, `ReAct`, or `ChainOfThought` |
| `max_iterations`     | 5            | Max think-act cycles (for ReAct)           |
| `enable_reflection`  | false        | Enable self-evaluation after acting        |
| `memory_window_size` | 20           | Items to keep in working memory            |
| `tool_timeout_ms`    | 30,000       | Timeout for each tool invocation           |
| `refine_after_tools` | true         | Do a refinement LLM call after tools       |
| `max_tools_exposed`  | 32           | Max tools to expose to the LLM             |

### CognitiveLoop Trait

The core trait defining the cognitive cycle:

```rust
#[async_trait]
pub trait CognitiveLoop: Send + Sync {
    /// Perceive: Process incoming event and build context
    async fn perceive(&mut self, event: Event, state: &AgentState) -> Result<Perception>;

    /// Think: Reason about the perception and create a plan
    async fn think(&mut self, perception: &Perception) -> Result<Plan>;

    /// Act: Execute the plan and produce results
    async fn act(&mut self, plan: &Plan, state: &mut AgentState) -> Result<ExecutionResult>;

    /// Reflect: Optional self-evaluation (default: no-op)
    async fn reflect(&mut self, ...) -> Result<Option<String>>;

    /// Access working memory
    fn working_memory(&self) -> &WorkingMemory;
    fn working_memory_mut(&mut self) -> &mut WorkingMemory;

    /// Run the complete cognitive cycle (provided default implementation)
    async fn run_cycle(&mut self, event: Event, state: &mut AgentState) -> Result<ExecutionResult>;
}
```

### SimpleCognitiveLoop

A ready-to-use implementation that integrates with `LlmClient` and `ActionBroker`:

```rust
use loom_core::agent::cognitive::{SimpleCognitiveLoop, CognitiveAgent, CognitiveConfig};
use loom_core::llm::LlmClient;
use loom_core::ActionBroker;
use std::sync::Arc;

// Create the cognitive loop
let config = CognitiveConfig::react();
let llm = Arc::new(LlmClient::from_env()?);
let broker = Arc::new(ActionBroker::new());

let loop_impl = SimpleCognitiveLoop::new(config, llm, broker)
    .with_correlation_id("session-123");

// Wrap as AgentBehavior
let behavior = CognitiveAgent::new(loop_impl);

// Use with AgentRuntime
let agent_config = AgentConfig {
    agent_id: "cognitive-agent-1".to_string(),
    agent_type: "cognitive".to_string(),
    subscribed_topics: vec!["user.messages".to_string()],
    capabilities: vec![],
    parameters: Default::default(),
};

runtime.create_agent(agent_config, Box::new(behavior)).await?;
```

### WorkingMemory

Short-term memory for the cognitive cycle:

```rust
use loom_core::agent::cognitive::{WorkingMemory, MemoryItem, MemoryItemType};

let mut memory = WorkingMemory::new(20); // capacity of 20 items

// Add different types of items
memory.add_user_message("What's the weather?");
memory.add_agent_response("Let me check...");
memory.add_observation("weather.get", "Sunny, 25°C");

// Search memory
let results = memory.search("weather");

// Task state (key-value pairs)
memory.set_state("current_step", "2");
let step = memory.get_state("current_step");

// Get context for LLM
let context = memory.to_context_string();
```

### Plan and Thought Structures

Structured representation of reasoning:

```rust
use loom_core::agent::cognitive::{Plan, ThoughtStep, ToolCall, Observation};
use serde_json::json;

// Build a plan
let mut plan = Plan::with_goal("Get weather in Tokyo");

// Add a tool call step
let tool = ToolCall::new("weather.get", json!({"city": "Tokyo"}));
plan.add_step(ThoughtStep::with_tool(1, "I should check the weather API", tool));

// Execute and add observation
// ... (observation added by act phase)

// Check status
if plan.has_pending_tools() {
    // Execute pending tool calls
}

// Complete the plan
plan.complete_with_answer("The weather in Tokyo is sunny, 25°C");
```

## Thinking Strategies

### SingleShot

One LLM call, no tool use. Best for simple Q&A:

```
Event → Perceive → Think (1 LLM call) → Act → Done
```

### ReAct (Reason + Act)

Iterative reasoning with tool use:

```
Event → Perceive → Think → Act (tool) → Observe → Think → Act → ... → Done
```

The LLM is prompted to output:

- **Thought**: Reasoning about what to do
- **Action**: Tool call as JSON `{"tool": "name", "args": {...}}`
- **FINAL ANSWER**: When done reasoning

### ChainOfThought

Step-by-step reasoning without explicit tool use:

```
Event → Perceive → Think (step 1) → Think (step 2) → ... → Act → Done
```

## Custom Cognitive Loops

Implement your own cognitive loop for custom behavior:

```rust
use loom_core::agent::cognitive::{
    CognitiveLoop, Perception, Plan, ExecutionResult, WorkingMemory
};

struct MyCustomLoop {
    memory: WorkingMemory,
    // ... your custom fields
}

#[async_trait]
impl CognitiveLoop for MyCustomLoop {
    async fn perceive(&mut self, event: Event, state: &AgentState) -> Result<Perception> {
        // Custom perception logic
        // e.g., parse structured data, query external systems
        Ok(Perception::from_event(event))
    }

    async fn think(&mut self, perception: &Perception) -> Result<Plan> {
        // Custom planning logic
        // e.g., rule-based, hierarchical planning, custom LLM integration
        Ok(Plan::final_answer(
            perception.goal.clone().unwrap_or_default(),
            "My custom answer",
        ))
    }

    async fn act(&mut self, plan: &Plan, state: &mut AgentState) -> Result<ExecutionResult> {
        // Custom action execution
        // e.g., API calls, database updates, event publishing
        Ok(ExecutionResult::with_response("Done"))
    }

    fn working_memory(&self) -> &WorkingMemory { &self.memory }
    fn working_memory_mut(&mut self) -> &mut WorkingMemory { &mut self.memory }
}
```

## Integration with Existing Systems

### With AgentRuntime

The `CognitiveAgent` adapter implements `AgentBehavior`, so cognitive agents work seamlessly with the existing runtime:

```rust
// Create cognitive agent
let loop_impl = SimpleCognitiveLoop::new(config, llm, broker);
let behavior = CognitiveAgent::new(loop_impl);

// Use with AgentRuntime just like any other agent
let agent_id = runtime.create_agent(config, Box::new(behavior)).await?;

// Dynamic subscription works
runtime.subscribe_agent(&agent_id, "new.topic".to_string()).await?;

// Deletion works
runtime.delete_agent(&agent_id).await?;
```

### With ToolOrchestrator

`SimpleCognitiveLoop` uses `ActionBroker` directly. For more sophisticated tool orchestration, you can integrate with `ToolOrchestrator`:

```rust
use loom_core::llm::ToolOrchestrator;

// In your custom CognitiveLoop implementation:
let orchestrator = ToolOrchestrator::new(llm.clone(), broker.clone());
let result = orchestrator.run(&bundle, Some(budget), options, correlation_id).await?;
```

### With Memory Systems

Working memory is ephemeral. For persistent memory, integrate with the context module:

```rust
use loom_core::context::{ContextBuilder, MemoryReader, MemoryWriter, InMemoryMemory};

// Use InMemoryMemory for episodic memory
let memory = InMemoryMemory::new();

// In perceive phase, query long-term memory
let context = memory.retrieve(query, k, None).await?;

// In act phase, store important information
memory.append_event(session_id, event).await?;
```

## Observability

The cognitive loop emits tracing spans for each phase:

```
cognitive_cycle
├── cognitive.perceive
│   └── event_id, event_type, goal, context_items, tools
├── cognitive.think
│   └── strategy, iterations, steps, has_pending_tools
├── cognitive.act
│   └── pending_tools, goal_achieved, tool executions
└── cognitive.reflect (optional)
    └── reflection text
```

Enable with standard Loom telemetry:

```rust
use loom_core::telemetry::init_telemetry;

init_telemetry("my-cognitive-agent")?;
```

## Best Practices

1. **Choose the right strategy**: Use `SingleShot` for simple tasks, `ReAct` for complex multi-step reasoning with tools.

2. **Tune iterations**: Set `max_iterations` based on task complexity. Too low may truncate reasoning; too high wastes tokens.

3. **Use reflection sparingly**: Enable `enable_reflection` only for critical agents where self-evaluation is valuable.

4. **Manage memory size**: Balance `memory_window_size` between context richness and token limits.

5. **Customize system prompts**: Use `with_system_prompt()` to give agents specific personas or constraints.

6. **Implement custom loops**: For specialized behavior, implement `CognitiveLoop` directly rather than using `SimpleCognitiveLoop`.

## Example: Market Analysis Agent

```rust
use loom_core::agent::cognitive::{SimpleCognitiveLoop, CognitiveAgent, CognitiveConfig};

// Configure for market analysis
let config = CognitiveConfig::react()
    .with_system_prompt(
        "You are a market analyst. Analyze market data and provide trading recommendations. \
         Use available tools to fetch real-time data. Be precise and data-driven."
    )
    .with_max_iterations(5)
    .with_reflection(); // Enable self-evaluation of recommendations

let loop_impl = SimpleCognitiveLoop::new(config, llm, broker);
let behavior = CognitiveAgent::new(loop_impl);

// The agent can now:
// 1. Perceive: Parse incoming market events
// 2. Think: Reason about market conditions, call analysis tools
// 3. Act: Generate trading recommendations
// 4. Reflect: Evaluate the quality of its analysis
```

## API Reference

See the module documentation:

- `loom_core::agent::cognitive::CognitiveConfig`
- `loom_core::agent::cognitive::CognitiveLoop`
- `loom_core::agent::cognitive::CognitiveAgent`
- `loom_core::agent::cognitive::SimpleCognitiveLoop`
- `loom_core::agent::cognitive::WorkingMemory`
- `loom_core::agent::cognitive::Plan`
- `loom_core::agent::cognitive::ThoughtStep`
- `loom_core::agent::cognitive::ToolCall`
- `loom_core::agent::cognitive::Observation`
