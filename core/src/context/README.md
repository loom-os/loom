# Context Engineering Module

The context module provides structured context management for AI agents, implementing
the "Context Engineering" pattern for LLM applications.

## Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                    CONTEXT PIPELINE                              │
│                                                                  │
│  Store ──▶ Retrieve ──▶ Rank ──▶ Window ──▶ PromptBundle        │
│                                                                  │
│  ┌─────────┐  ┌───────────┐  ┌────────┐  ┌────────┐            │
│  │ Memory  │  │ Recency   │  │Temporal│  │ Token  │            │
│  │ Store   │  │ Semantic  │  │Importnc│  │ Budget │            │
│  └─────────┘  └───────────┘  └────────┘  └────────┘            │
└─────────────────────────────────────────────────────────────────┘
```

## Module Structure

```
context/
├── agent_context.rs    # High-level API for agents
├── types.rs            # Core types (ContextItem, Metadata, etc.)
├── builder.rs          # ContextBuilder for PromptBundle creation
├── memory/             # Storage backends
│   └── store.rs          # MemoryStore trait + InMemoryStore
├── retrieval/          # Retrieval strategies
│   └── strategy.rs       # RecencyRetrieval, SemanticRetrieval
├── ranking/            # Context ranking
│   └── ranker.rs         # TemporalRanker, ImportanceRanker
├── window/             # Token budget management
│   ├── manager.rs        # WindowManager
│   └── token_counter.rs  # TiktokenCounter
└── pipeline/           # Context orchestration
    └── orchestrator.rs   # ContextPipeline
```

## Key Types

### ContextItem

The fundamental unit of context:

```rust
pub struct ContextItem {
    pub id: String,
    pub item_type: ContextItemType,
    pub content: ContextContent,
    pub metadata: ContextMetadata,
}

pub enum ContextItemType {
    Message { role: MessageRole },
    ToolCall { tool_name: String },
    ToolResult { tool_name: String, success: bool },
    Event { event_type: String },
    Observation { source: String },
    Document { title: String },
}
```

### AgentContext

High-level API for agents to record and retrieve context:

```rust
let ctx = AgentContext::with_defaults("session-1", "agent-1");

// Record interactions
ctx.record_message(MessageRole::User, "What's the weather?").await?;
ctx.record_tool_call("weather.get", json!({"city": "Tokyo"})).await?;
ctx.record_tool_result("weather.get", true, result, call_id).await?;
ctx.record_message(MessageRole::Assistant, "It's sunny in Tokyo.").await?;

// Retrieve context for LLM
let trigger = RetrievalTrigger::new("session-1")
    .with_goal("Continue the conversation")
    .with_budget(TokenBudget::default());
let bundle = ctx.get_context(trigger).await?;
```

### ContextPipeline

Orchestrates the full context flow:

```rust
let pipeline = ContextPipeline::new(
    store,           // MemoryStore implementation
    retrieval,       // RetrievalStrategy
    ranker,          // ContextRanker
    window_manager,  // WindowManager
    config,          // PipelineConfig
);

let bundle = pipeline.build_context(trigger).await?;
```

### PromptBundle

LLM-ready prompt structure:

```rust
pub struct PromptBundle {
    pub system: String,
    pub instructions: Option<String>,
    pub context_docs: Vec<String>,
    pub history: Vec<String>,
    pub tools_json_schema: Option<Value>,
}
```

## Storage

### MemoryStore Trait

```rust
#[async_trait]
pub trait MemoryStore: Send + Sync {
    async fn store(&self, item: ContextItem) -> Result<()>;
    async fn get(&self, id: &str) -> Result<Option<ContextItem>>;
    async fn query(&self, query: MemoryQuery) -> Result<Vec<ContextItem>>;
    async fn delete(&self, id: &str) -> Result<()>;
}
```

### Implementations

| Implementation | Use Case |
|---------------|----------|
| `InMemoryStore` | Development, testing |
| `RocksDbStore` | Production persistence (TODO) |

## Retrieval Strategies

| Strategy | Description |
|----------|-------------|
| `RecencyRetrieval` | Most recent N items |
| `TypeFilteredRetrieval` | Filter by ContextItemType |
| `ImportanceRetrieval` | Filter by importance threshold |
| `CompositeRetrieval` | Combine multiple strategies |

## Ranking

| Ranker | Description |
|--------|-------------|
| `TemporalRanker` | Sort by timestamp (newest/oldest first) |
| `ImportanceRanker` | Sort by importance score |
| `CompositeRanker` | Chain multiple rankers |

## Token Windowing

`WindowManager` enforces token budgets:

```rust
let config = WindowConfig {
    max_tokens: 4096,
    per_type_budgets: hashmap! {
        ContextItemType::Message { role: MessageRole::User } => 2048,
        ContextItemType::ToolResult { .. } => 1024,
    },
    reserve_output: 1024,
};

let manager = WindowManager::new(TiktokenCounter::gpt4(), config);
let selected = manager.select_within_budget(items)?;
```

## Integration with Cognitive Loop

`SimpleCognitiveLoop` integrates `AgentContext` for automatic context recording:

```rust
let ctx = AgentContext::with_defaults("session", "agent");
let loop_impl = SimpleCognitiveLoop::new(config, llm, tools)
    .with_context(ctx);

// Context is automatically recorded during:
// - perceive(): Events are recorded
// - act(): Tool calls and results are recorded
```

## Migration from WorkingMemory

`cognitive/working_memory.rs` is deprecated. Use `AgentContext` instead:

| WorkingMemory | AgentContext |
|--------------|--------------|
| `add_event(e)` | `record_event(&e).await` |
| `add_observation(t, r)` | `record_tool_result(t, ok, r, id).await` |
| `add_user_message(m)` | `record_message(User, m).await` |
| `recent(n)` | `get_context(trigger).await` |

## Testing

```bash
cargo test -p loom-core context::
```

All context tests (40+) are in the respective module files.
