## Cognitive Runtime & Agent Pattern

**Status**: ✅ Implemented — The cognitive layer is fully implemented in `core/src/agent/cognitive/`.

---

### Goals

- **Lift agents from purely reactive callbacks to an explicit perceive–think–act loop** without breaking the existing `AgentRuntime` and `AgentBehavior` abstractions.
- **Keep the core runtime simple**: EventBus + AgentRuntime remain general-purpose infrastructure; cognitive behavior is an opt-in pattern.
- **Make cognition observable**: planning steps, policy decisions, and memory usage should be inspectable from logs, traces, and the Dashboard.
- **Reuse existing building blocks**: `ContextBuilder`, `MemoryReader/Writer`, `ModelRouter`, `ActionBroker`, and collaboration primitives.

---

### Module Structure

The cognitive module is organized as follows:

```
core/src/agent/cognitive/
├── mod.rs              # Public exports
├── config.rs           # CognitiveConfig with builder pattern
├── loop_trait.rs       # CognitiveLoop trait and core types
├── thought.rs          # ThoughtStep, Plan, ToolCall, Observation
├── working_memory.rs   # WorkingMemory for in-loop context
├── agent_adapter.rs    # CognitiveAgent bridging to AgentBehavior
├── simple_loop.rs      # SimpleCognitiveLoop with ReAct pattern
└── README.md           # Module documentation
```

---

### Core Interfaces

#### CognitiveLoop Trait

The core of the design is the **loop trait** with five methods:

```rust
#[async_trait::async_trait]
pub trait CognitiveLoop: Send + Sync {
    /// Gather and interpret incoming information
    async fn perceive(&mut self, perception: Perception) -> Result<()>;

    /// Reason about the current situation and form a plan
    async fn think(&mut self) -> Result<Plan>;

    /// Execute the plan and produce actions
    async fn act(&mut self, plan: &Plan) -> Result<ExecutionResult>;

    /// Learn from the execution result (optional)
    async fn reflect(&mut self, result: &ExecutionResult) -> Result<()>;

    /// Run a complete cognitive cycle
    async fn run_cycle(&mut self, perception: Perception) -> Result<ExecutionResult>;
}
```

#### Key Types

```rust
/// Input to the cognitive loop
pub struct Perception {
    pub event: Option<Event>,
    pub context: HashMap<String, String>,
    pub timestamp: Instant,
}

/// Result of executing a plan
pub struct ExecutionResult {
    pub actions: Vec<Action>,
    pub tool_results: Vec<ToolResult>,
    pub success: bool,
    pub error: Option<String>,
}
```

#### CognitiveAgent Adapter

The adapter bridges `CognitiveLoop` with the existing `AgentBehavior`:

```rust
pub struct CognitiveAgent<L: CognitiveLoop> {
    cognitive_loop: L,
    config: CognitiveConfig,
}

#[async_trait::async_trait]
impl<L: CognitiveLoop + 'static> AgentBehavior for CognitiveAgent<L> {
    async fn on_event(
        &mut self,
        event: Event,
        _state: &mut AgentState,
    ) -> Result<Vec<Action>> {
        let perception = Perception::from_event(event);
        let result = self.cognitive_loop.run_cycle(perception).await?;
        Ok(result.actions)
    }
    // ...
}
```

---

### Configuration

`CognitiveConfig` provides flexible configuration via builder pattern:

```rust
let config = CognitiveConfig::builder()
    .agent_id("analyst-agent")
    .thinking_strategy(ThinkingStrategy::ReAct)  // or ChainOfThought, SingleShot
    .max_iterations(10)
    .iteration_timeout(Duration::from_secs(30))
    .enable_reflection(true)
    .memory_capacity(1000)
    .build();
```

**Thinking Strategies:**

- `ReAct` — Interleaved reasoning and acting (default, recommended)
- `ChainOfThought` — Complete reasoning before acting
- `SingleShot` — No explicit reasoning trace

---

### Working Memory

`WorkingMemory` provides in-loop context management:

```rust
let mut memory = WorkingMemory::new(100); // capacity

// Store and retrieve items
memory.store("user_intent", json!({"goal": "analyze data"}));
let intent = memory.retrieve("user_intent");

// Task state tracking
memory.set_task_state(TaskState::InProgress);

// Search by key pattern
let results = memory.search("user");
```

---

### Thought & Planning

The module provides structured types for reasoning:

```rust
// Create a plan
let mut plan = Plan::new("analyze-task");

// Add reasoning steps
plan.add_step(ThoughtStep::reasoning(1, "User wants data analysis"));
plan.add_step(ThoughtStep::action(2, "query_database"));

// Add tool calls
plan.add_tool_call(ToolCall {
    name: "sql_query".to_string(),
    arguments: json!({"query": "SELECT * FROM data"}),
    id: Some("call-1".to_string()),
});
```

---

### SimpleCognitiveLoop

The default implementation with ReAct pattern support:

```rust
// Create with LLM client
let cognitive_loop = SimpleCognitiveLoop::new(llm_client, config);

// Or with action broker for tool execution
let cognitive_loop = SimpleCognitiveLoop::with_action_broker(
    llm_client,
    action_broker,
    config,
);

// Run a cognitive cycle
let perception = Perception::from_event(event);
let result = cognitive_loop.run_cycle(perception).await?;
```

The `SimpleCognitiveLoop` includes:

- ReAct-style reasoning with explicit Thought/Action/Observation traces
- Tool call parsing from LLM output
- Configurable iteration limits and timeouts
- OpenTelemetry tracing integration
- Optional reflection phase for learning

---

### Observability

The cognitive module integrates with OpenTelemetry:

- Tracing spans for `perceive`, `think`, `act`, and `reflect` phases
- Thread/correlation IDs from `Envelope` metadata
- Tool call and result tracking
- Planning decision logging

Dashboard integration (future work):

- Cognitive agents marked in topology graph
- Current phase and recent plan steps in agent detail view

---

### Usage Example

```rust
use loom_core::agent::cognitive::{
    CognitiveAgent, CognitiveConfig, SimpleCognitiveLoop,
    ThinkingStrategy, Perception,
};

// 1. Create configuration
let config = CognitiveConfig::builder()
    .agent_id("my-cognitive-agent")
    .thinking_strategy(ThinkingStrategy::ReAct)
    .max_iterations(5)
    .build();

// 2. Create cognitive loop with LLM
let cognitive_loop = SimpleCognitiveLoop::new(llm_client, config.clone());

// 3. Wrap in CognitiveAgent for AgentRuntime compatibility
let agent = CognitiveAgent::new(cognitive_loop, config);

// 4. Register with AgentRuntime as usual
runtime.register_behavior("cognitive-agent", agent).await?;
```

---

### Implementation Status

| Component                | Status    | Notes                               |
| ------------------------ | --------- | ----------------------------------- |
| `CognitiveLoop` trait    | ✅ Done   | Core abstraction with 5 methods     |
| `CognitiveAgent` adapter | ✅ Done   | Bridges to `AgentBehavior`          |
| `SimpleCognitiveLoop`    | ✅ Done   | ReAct pattern implementation        |
| `CognitiveConfig`        | ✅ Done   | Builder pattern with strategies     |
| `WorkingMemory`          | ✅ Done   | Capacity limits, search, task state |
| `ThoughtStep` / `Plan`   | ✅ Done   | Structured reasoning types          |
| Unit tests               | ✅ Done   | 31 tests in `cognitive_test.rs`     |
| Documentation            | ✅ Done   | README.md + this design doc         |
| Dashboard integration    | 🔲 Future | P2 milestone                        |
| Higher-level templates   | 🔲 Future | P2 milestone                        |

---

### Future Work (P2+)

- **Dashboard Integration**: Surface cognitive agent state in the UI
- **Cognitive Templates**: Pre-built patterns for common agent roles
- **Enhanced Memory**: Integration with `MemoryReader`/`MemoryWriter`
- **Policy Configuration**: Declarative behavior policies
- **Metrics**: OpenTelemetry metrics for cognitive loop performance
