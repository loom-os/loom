## Cognitive Runtime & Agent Pattern

**Status**: ✅ Implemented — The cognitive layer is implemented in `core/src/cognitive/`.

---

### Goals

- **Lift agents from purely reactive callbacks to an explicit perceive–think–act loop** without breaking the existing `AgentRuntime` and `AgentBehavior` abstractions.
- **Keep the core runtime simple**: EventBus + AgentRuntime remain general-purpose infrastructure; cognitive behavior is an opt-in pattern.
- **Make cognition observable**: planning steps, policy decisions, and context usage should be inspectable from logs, traces, and the Dashboard.
- **Reuse existing building blocks**: `AgentContext`, `ModelRouter`, `ToolRegistry`, and collaboration primitives.

---

### Module Structure

The cognitive module is organized as follows:

```
core/src/cognitive/
├── mod.rs              # Public exports
├── config.rs           # CognitiveConfig with builder pattern
├── loop_trait.rs       # CognitiveLoop trait and core types
├── thought.rs          # ThoughtStep, Plan, ToolCall, Observation
├── memory_buffer.rs    # Simple in-process memory buffer
├── agent_adapter.rs    # CognitiveAgent bridging to AgentBehavior
├── simple_loop.rs      # SimpleCognitiveLoop with ReAct pattern
├── llm/                # LLM client, router, providers
│   ├── client.rs       # HTTP client for LLM APIs
│   ├── router.rs       # Model routing based on policies
│   └── providers/      # Provider-specific adapters
└── README.md           # Module documentation
```

---

### Core Interfaces

#### CognitiveLoop Trait

The core of the design is the **loop trait** with five methods:

```rust
#[async_trait]
pub trait CognitiveLoop: Send + Sync {
    /// Perceive phase: process incoming event and build context
    async fn perceive(&mut self, event: Event, state: &AgentState) -> Result<Perception>;

    /// Think phase: reason about the perception and create a plan
    async fn think(&mut self, perception: &Perception) -> Result<Plan>;

    /// Act phase: execute the plan and produce results
    async fn act(&mut self, plan: &Plan, state: &mut AgentState) -> Result<ExecutionResult>;

    /// Reflect phase: learn from execution (optional)
    async fn reflect(&mut self, perception: &Perception, plan: &Plan, result: &ExecutionResult) -> Result<Option<String>>;

    /// Access the memory buffer
    fn memory_buffer(&self) -> &MemoryBuffer;
    fn memory_buffer_mut(&mut self) -> &mut MemoryBuffer;

    /// Run a complete cognitive cycle
    async fn run_cycle(&mut self, event: Event, state: &mut AgentState) -> Result<ExecutionResult>;
}
```

#### Key Types

```rust
/// Result of perceiving an event
pub struct Perception {
    pub event: Event,
    pub goal: Option<String>,
    pub context: Vec<String>,
    pub available_tools: Vec<String>,
    pub priority: i32,
}

/// Result of executing a plan
pub struct ExecutionResult {
    pub actions: Vec<Action>,
    pub response: Option<String>,
    pub goal_achieved: bool,
    pub error: Option<String>,
}
```

#### CognitiveAgent Adapter

The adapter bridges `CognitiveLoop` with the existing `AgentBehavior`:

```rust
pub struct CognitiveAgent<L: CognitiveLoop> {
    loop_impl: L,
    initialized: bool,
}

#[async_trait]
impl<L: CognitiveLoop + 'static> AgentBehavior for CognitiveAgent<L> {
    async fn on_event(&mut self, event: Event, state: &mut AgentState) -> Result<Vec<Action>> {
        let result = self.loop_impl.run_cycle(event, state).await?;
        Ok(result.into_actions())
    }
}
```

---

### Configuration

`CognitiveConfig` provides flexible configuration via builder pattern:

```rust
let config = CognitiveConfig::react()
    .with_system_prompt("You are a helpful assistant")
    .with_max_iterations(10)
    .with_reflection()
    .with_memory_window(50);
```

**Thinking Strategies:**

- `SingleShot` — One LLM call, no tools
- `ReAct` — Interleaved reasoning and acting (recommended)
- `ChainOfThought` — Multi-step reasoning before acting

---

### Memory Buffer

`MemoryBuffer` provides simple in-process context management:

```rust
let mut buffer = MemoryBuffer::new(50); // capacity

// Add items
buffer.add_user_message("Hello");
buffer.add_agent_response("Hi there!");
buffer.add_observation("weather", "Sunny, 72°F");

// Get recent items
let recent = buffer.recent(5);

// Convert to context string for prompts
let context = buffer.to_context_string();
```

For persistent and advanced context management, use `AgentContext` from the context module.

---

### AgentContext Integration

`SimpleCognitiveLoop` integrates with `AgentContext` for intelligent context management:

```rust
use loom_core::cognitive::SimpleCognitiveLoop;
use loom_core::context::AgentContext;

let config = CognitiveConfig::react();
let llm = Arc::new(LlmClient::from_env()?);
let tools = Arc::new(ToolRegistry::new());
let context = AgentContext::with_defaults("session-1", "agent-1");

let loop_impl = SimpleCognitiveLoop::new(config, llm, tools)
    .with_context(context);  // Enables automatic context recording
```

When `AgentContext` is set:
- Incoming events are recorded during `perceive()`
- Tool calls and results are recorded during `act()`
- Context can be retrieved for LLM prompts

---

### Thought & Planning

The module provides structured types for reasoning:

```rust
// Create a plan
let mut plan = Plan::with_goal("analyze-task".to_string());

// Add reasoning steps
plan.add_step(ThoughtStep::reasoning(1, "User wants data analysis"));

// Add tool calls
let tool_call = ToolCall::new("sql_query", json!({"query": "SELECT * FROM data"}));
plan.add_step(ThoughtStep::with_tool(2, "Query the database", tool_call));

// Mark complete
plan.complete_with_answer("Here are the results...");
```

---

### SimpleCognitiveLoop

The default implementation with ReAct pattern support:

```rust
use loom_core::cognitive::{SimpleCognitiveLoop, CognitiveConfig, CognitiveAgent};
use loom_core::tools::ToolRegistry;

// Create cognitive loop
let config = CognitiveConfig::react().with_max_iterations(5);
let loop_impl = SimpleCognitiveLoop::new(config, llm_client, tool_registry);

// Wrap in CognitiveAgent for AgentRuntime compatibility
let behavior = CognitiveAgent::new(loop_impl);

// Register with AgentRuntime
runtime.create_agent(agent_config, Box::new(behavior)).await?;
```

The `SimpleCognitiveLoop` includes:

- ReAct-style reasoning with explicit Thought/Action/Observation traces
- Tool call parsing from LLM output
- Configurable iteration limits
- Optional AgentContext integration for context recording
- OpenTelemetry tracing integration
- Optional reflection phase

---

### Observability

The cognitive module integrates with OpenTelemetry:

- Tracing spans for `perceive`, `think`, `act`, and `reflect` phases
- Thread/correlation IDs from `Envelope` metadata
- Tool call and result tracking
- Planning decision logging

---

### Usage Example

```rust
use loom_core::cognitive::{
    CognitiveAgent, CognitiveConfig, SimpleCognitiveLoop,
    ThinkingStrategy,
};
use loom_core::context::AgentContext;

// 1. Create configuration
let config = CognitiveConfig::react()
    .with_system_prompt("You are a helpful assistant")
    .with_max_iterations(5)
    .with_reflection();

// 2. Create cognitive loop with LLM and tools
let loop_impl = SimpleCognitiveLoop::new(config, llm_client, tool_registry)
    .with_context(AgentContext::with_defaults("session-1", "agent-1"));

// 3. Wrap in CognitiveAgent for AgentRuntime compatibility
let agent = CognitiveAgent::new(loop_impl);

// 4. Register with AgentRuntime
runtime.create_agent(agent_config, Box::new(agent)).await?;
```

---

### Implementation Status

| Component                | Status    | Notes                               |
| ------------------------ | --------- | ----------------------------------- |
| `CognitiveLoop` trait    | ✅ Done   | Core abstraction                    |
| `CognitiveAgent` adapter | ✅ Done   | Bridges to `AgentBehavior`          |
| `SimpleCognitiveLoop`    | ✅ Done   | ReAct pattern implementation        |
| `CognitiveConfig`        | ✅ Done   | Builder pattern with strategies     |
| `MemoryBuffer`           | ✅ Done   | Simple in-process memory            |
| `AgentContext` integration | ✅ Done | Optional context recording          |
| `ThoughtStep` / `Plan`   | ✅ Done   | Structured reasoning types          |
| LLM subsystem            | ✅ Done   | Client, router, providers           |
| Unit tests               | ✅ Done   | Tests in `cognitive_test.rs`        |
| Documentation            | ✅ Done   | README.md + this design doc         |

---

### Related Modules

- **Context Engineering** (`context/`): `AgentContext` for intelligent context management
- **Tools** (`tools/`): `ToolRegistry` for tool execution
- **Agent Runtime** (`agent/`): `AgentBehavior` and lifecycle management
