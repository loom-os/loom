# Cognitive Module

LLM-powered reasoning for intelligent agents using the Perceive-Think-Act pattern.

## Overview

```
Event ──▶ [PERCEIVE] ──▶ [THINK] ──▶ [ACT] ──▶ Actions
              │            │          │
              ▼            ▼          ▼
         AgentContext   LLM+Router  ToolRegistry
```

## Module Structure

```
cognitive/
├── llm/                # LLM subsystem
│   ├── client.rs         # HTTP client for LLM APIs
│   ├── router.rs         # Model routing (local/cloud)
│   ├── provider.rs       # Provider abstraction
│   ├── adapter.rs        # LLM as Tool adapter
│   └── tool_orchestrator.rs  # Tool call parsing
│
├── simple_loop.rs      # Main CognitiveLoop implementation
├── loop_trait.rs       # CognitiveLoop trait definition
├── config.rs           # CognitiveConfig + ThinkingStrategy
├── thought.rs          # Plan, ToolCall, Observation types
├── agent_adapter.rs    # CognitiveAgent (adapts to AgentBehavior)
└── working_memory.rs   # DEPRECATED: Use context/agent_context.rs
```

## Quick Start

```rust
use loom_core::cognitive::{SimpleCognitiveLoop, CognitiveAgent, CognitiveConfig};
use loom_core::context::AgentContext;

// Create cognitive loop with context
let config = CognitiveConfig::react();
let ctx = AgentContext::with_defaults("session-1", "agent-1");
let loop_impl = SimpleCognitiveLoop::new(config, llm, tools)
    .with_context(ctx);

// Wrap as AgentBehavior
let behavior = CognitiveAgent::new(loop_impl);

// Use with AgentRuntime
runtime.create_agent(agent_config, Box::new(behavior)).await?;
```

## CognitiveConfig

| Option | Default | Description |
|--------|---------|-------------|
| `thinking_strategy` | SingleShot | SingleShot, ReAct, or ChainOfThought |
| `max_iterations` | 5 | Max think-act cycles (ReAct) |
| `enable_reflection` | false | Self-evaluation after acting |
| `memory_window_size` | 20 | Items in working memory |
| `tool_timeout_ms` | 30,000 | Tool execution timeout |
| `refine_after_tools` | true | Refinement LLM call after tools |
| `max_tools_exposed` | 32 | Max tools to expose to LLM |

**Presets:**

```rust
CognitiveConfig::single_shot()     // One LLM call, no tools
CognitiveConfig::react()           // ReAct pattern with tools
CognitiveConfig::chain_of_thought() // Step-by-step reasoning
```

## ThinkingStrategy

| Strategy | Description | Use Case |
|----------|-------------|----------|
| SingleShot | One LLM call | Simple Q&A, no tools |
| ReAct | Iterative reasoning + tools | Complex tasks with tool use |
| ChainOfThought | Multi-step reasoning | Complex reasoning without tools |

## CognitiveLoop Trait

```rust
#[async_trait]
pub trait CognitiveLoop: Send + Sync {
    /// Process event and build context
    async fn perceive(&mut self, event: Event, state: &AgentState) -> Result<Perception>;

    /// Reason about perception, create plan
    async fn think(&mut self, perception: &Perception) -> Result<Plan>;

    /// Execute plan, produce results
    async fn act(&mut self, plan: &Plan, state: &mut AgentState) -> Result<ExecutionResult>;

    /// Optional self-evaluation
    async fn reflect(&mut self, result: &ExecutionResult) -> Result<Option<Thought>> {
        Ok(None)
    }

    /// Access working memory (deprecated)
    fn working_memory(&self) -> &WorkingMemory;
}
```

## SimpleCognitiveLoop

Default implementation with:

- **Perceive**: Records event in AgentContext, builds Perception
- **Think**: Calls LLM with context, parses response for tool calls or final answer
- **Act**: Executes tool calls via ToolRegistry, records results in AgentContext

```rust
let loop_impl = SimpleCognitiveLoop::new(config, llm, tools)
    .with_context(AgentContext::with_defaults("s1", "a1"))
    .with_correlation_id("trace-123");
```

## LLM Subsystem

### LlmClient

```rust
let client = LlmClient::from_env()?;  // Uses LLM_* env vars
let response = client.generate(&prompt_bundle, Some(4096)).await?;
```

### ModelRouter

Routes requests based on:
- Privacy policy (local-only, sensitive, public)
- Model capabilities
- Cost and latency constraints

```rust
let router = ModelRouter::new().await?;
let decision = router.route(&context, &task).await?;
match decision.route {
    Route::Local => { /* use local model */ }
    Route::Cloud => { /* use cloud API */ }
    Route::Hybrid => { /* split sensitive/non-sensitive */ }
}
```

## Thought Types

```rust
pub struct Plan {
    pub goal: String,
    pub steps: Vec<ThoughtStep>,
    pub complete: bool,
}

pub struct ThoughtStep {
    pub step_number: usize,
    pub reasoning: String,
    pub tool_call: Option<ToolCall>,
    pub observation: Option<Observation>,
}

pub struct ToolCall {
    pub id: Option<String>,
    pub name: String,
    pub arguments: Value,
}

pub struct Observation {
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
}
```

## Context Integration

SimpleCognitiveLoop automatically records to AgentContext:

| Phase | Records |
|-------|---------|
| perceive() | Incoming events |
| act() | Tool calls and results |

Future: think() will record LLM prompts and responses.

## Migration from WorkingMemory

`working_memory.rs` is **deprecated**. Use `AgentContext` instead.

See `context/README.md` for migration guide.

## Testing

```bash
cargo test -p loom-core cognitive::
```
