## Memory & Context System

**Status**: Implemented — this document describes Loom's memory and context systems.

---

### Overview

The memory system provides multiple layers of abstraction:

1. **MemoryStore trait** — Core storage abstraction for context items with query, batch, and indexing
2. **InMemoryStore** — Fast in-process storage for development/testing
3. **RocksDbStore** — Persistent storage with session, type, and time indices
4. **AgentContext** — High-level API integrating storage, retrieval, ranking, and windowing
5. **MemoryBuffer** — Simple in-process buffer for cognitive loop working memory

---

### Architecture

#### Core Trait: MemoryStore

```rust
#[async_trait::async_trait]
pub trait MemoryStore: Send + Sync + 'static {
    /// Store a single context item
    async fn store(&self, item: ContextItem) -> Result<()>;

    /// Store multiple items atomically
    async fn store_batch(&self, items: Vec<ContextItem>) -> Result<()>;

    /// Get item by ID
    async fn get(&self, id: &str) -> Result<Option<ContextItem>>;

    /// Query items with filters and limits
    async fn query(&self, query: MemoryQuery) -> Result<Vec<ContextItem>>;

    /// Get related items (for future knowledge graph support)
    async fn get_related(&self, id: &str, relation: &str) -> Result<Vec<ContextItem>>;

    /// Count items matching query
    async fn count(&self, query: MemoryQuery) -> Result<usize>;
}
```

#### Query API

```rust
pub struct MemoryQuery {
    pub session_id: Option<String>,
    pub item_types: Option<Vec<ContextItemType>>,
    pub since: Option<u64>,      // timestamp_ms
    pub until: Option<u64>,      // timestamp_ms
    pub limit: Option<usize>,
}

// Builder pattern
let query = MemoryQuery::new()
    .session("agent-123")
    .types(vec![ContextItemType::ToolCall, ContextItemType::ToolResult])
    .since(timestamp_30_min_ago)
    .limit(50);
```

#### ContextItem Structure

```rust
pub struct ContextItem {
    pub id: String,                    // UUID v4
    pub item_type: ContextItemType,    // Message, ToolCall, ToolResult, etc.
    pub content: ContextContent,       // Typed content wrapper
    pub metadata: ContextMetadata,     // Session, trace, timestamp
}

pub enum ContextItemType {
    Message,      // User/assistant messages
    ToolCall,     // Tool invocation
    ToolResult,   // Tool response
    Observation,  // Agent observations
    Summary,      // Compressed context
    Document,     // Retrieved documents
}
```

---

### Implementations

#### InMemoryStore (Development/Testing)

```rust
use loom_core::context::InMemoryStore;

let store = InMemoryStore::new();

// Store items
store.store(item).await?;

// Query by session
let items = store.query(
    MemoryQuery::new()
        .session("session-1")
        .limit(100)
).await?;
```

**Characteristics**:
- DashMap-based concurrent access
- Session index for fast filtering
- Type index for item type queries
- Timestamp ordering

#### RocksDbStore (Production)

```rust
use loom_core::context::RocksDbStore;

let store = RocksDbStore::open("/path/to/db")?;

// Same API as InMemoryStore
store.store(item).await?;

let items = store.query(
    MemoryQuery::new()
        .session("session-1")
        .since(timestamp)
).await?;
```

**Characteristics**:
- Persistent to disk
- Column families for indices:
  - `default`: item data (key: id, value: JSON)
  - `session_idx`: session → item IDs
  - `type_idx`: type → item IDs
  - `time_idx`: timestamp → item IDs
- Crash recovery
- Large history support

---

### AgentContext: High-Level API

`AgentContext` provides a unified interface combining storage, retrieval, ranking, and windowing:

```rust
use loom_core::context::AgentContext;

// Create with defaults (InMemoryStore, RecencyRetrieval, TemporalRanker)
let ctx = AgentContext::new("session-123");

// Or with custom store
let store = Arc::new(RocksDbStore::open("/path/to/db")?);
let ctx = AgentContext::with_store("session-123", store);

// Record conversation
ctx.record_user_message("What's the weather?").await?;
ctx.record_assistant_message("Let me check...").await?;

// Record tool usage
ctx.record_tool_call("weather", json!({"location": "NYC"})).await?;
ctx.record_tool_result("weather", json!({"temp": 72, "condition": "sunny"})).await?;

// Build context for LLM (with token budget)
let context = ctx.build_context(4000).await?;
println!("{}", context); // Formatted for LLM prompt
```

---

### MemoryBuffer: Working Memory

`MemoryBuffer` provides a simple capacity-limited buffer for cognitive loop working memory:

```rust
use loom_core::cognitive::MemoryBuffer;

let mut buffer = MemoryBuffer::new(100); // max 100 items

// Add items (auto-evicts oldest when at capacity)
buffer.add_user_message("Hello");
buffer.add_agent_response("Hi there!");
buffer.add_observation("User seems friendly");

// Get recent items
let recent = buffer.recent(10);

// Format for LLM prompt
let context = buffer.to_context_string();
```

**Use case**: Transient conversation state within a single cognitive loop run.

---

### Context Pipeline

The `ContextPipeline` orchestrates the full flow:

```
Store → Retrieve → Rank → Window → Format
```

```rust
use loom_core::context::{
    ContextPipeline, PipelineConfig,
    RecencyRetrieval, TemporalRanker, WindowConfig,
};

let pipeline = ContextPipeline::new(
    store,
    Arc::new(RecencyRetrieval::new(100)),
    Arc::new(TemporalRanker),
    WindowConfig::default(),
);

let result = pipeline.execute(
    "session-123",
    RetrievalTrigger::NewMessage,
).await?;

// result.items: ranked, windowed context items
// result.token_count: actual token usage
```

---

### Retrieval Strategies

```rust
// By recency (most recent N items)
let retrieval = RecencyRetrieval::new(100);

// By importance (high-importance items first)
let retrieval = ImportanceRetrieval::new(50);

// By type (only specific item types)
let retrieval = TypeFilteredRetrieval::new(
    vec![ContextItemType::ToolCall, ContextItemType::ToolResult],
    inner_retrieval,
);

// Composite (combine multiple strategies)
let retrieval = CompositeRetrieval::new(vec![
    Arc::new(RecencyRetrieval::new(50)),
    Arc::new(ImportanceRetrieval::new(50)),
]);
```

---

### Ranking Strategies

```rust
// Temporal (most recent = highest)
let ranker = TemporalRanker;

// Importance-based
let ranker = ImportanceRanker;

// Composite (weighted combination)
let ranker = CompositeRanker::new(vec![
    (Arc::new(TemporalRanker), 0.5),
    (Arc::new(ImportanceRanker), 0.5),
]);
```

---

### Token Window Management

```rust
use loom_core::context::{WindowConfig, WindowManager, TiktokenCounter};

let config = WindowConfig {
    max_tokens: 4000,
    reserve_tokens: 500,  // for response
};

let counter = TiktokenCounter::new("gpt-4");
let manager = WindowManager::new(config, Arc::new(counter));

// Trim items to fit budget
let windowed = manager.fit_items(ranked_items)?;
```

---

### Testing

#### Core Tests (`core/tests/memory_test.rs`)

- ✅ Store and retrieve items
- ✅ Query by session
- ✅ Query by type
- ✅ Query with time range
- ✅ Batch operations
- ✅ Count queries

#### RocksDbStore Tests (`core/src/context/memory/persistent.rs`)

- ✅ `test_store_and_get`: Basic round-trip
- ✅ `test_query_by_session`: Session filtering
- ✅ `test_count`: Count queries
- ✅ `test_store_batch`: Atomic batch writes
- ✅ `test_query_with_limit`: Limit enforcement

**Run**: `cargo test -p loom-core store`

---

### Migration from Legacy APIs

If using the older `MemoryReader/MemoryWriter` traits:

```rust
// Old API
trait MemoryWriter {
    async fn append_event(&self, session: &str, event: Event) -> Result<()>;
    async fn summarize_episode(&self, session: &str) -> Result<Option<String>>;
}

// New API - use AgentContext
let ctx = AgentContext::new("session");
ctx.record_event(event).await?;
// Summarization via pipeline with Summary item type
```

The `ContextBuilder` (using `MemoryReader/MemoryWriter`) is being replaced by `ContextPipeline` and `AgentContext`.

---

### Design Principles

1. **Everything is Retrievable**: Store raw items, compress on read
2. **Full Traceability**: All items linked via OpenTelemetry trace_id/span_id
3. **Tool-First**: Tool calls/results are first-class citizens
4. **Intelligent Selection**: Dynamic windowing based on relevance + budget
5. **Backend Flexibility**: Same API for in-memory, RocksDB, or future backends

End of memory documentation.
