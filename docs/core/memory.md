## Memory & Context System (Design Draft)

**Status**: Design / draft — this document describes the intended evolution of Loom's memory system around the existing `context` module.

---

### Goals

- Provide a **unified abstraction** for episodic and semantic memory that is usable from Rust and SDKs.
- Keep the **initial implementation simple** (in‑process, RocksDB, or external KV) while allowing later upgrades to vector databases and knowledge graphs.
- Make memory operations **observable** (reads, writes, retrieval quality) and easy to debug.
- Integrate cleanly with:
  - `ContextBuilder` (prompt assembly),
  - Cognitive agents (planning and reasoning),
  - Event persistence/replay.

---

### Existing Building Blocks

The `context` module already exposes two core traits:

```rust
#[async_trait::async_trait]
pub trait MemoryWriter: Send + Sync {
    async fn append_event(&self, session: &str, event: crate::proto::Event) -> crate::Result<()>;
    async fn summarize_episode(&self, session: &str) -> crate::Result<Option<String>>;
}

#[async_trait::async_trait]
pub trait MemoryReader: Send + Sync {
    async fn retrieve(
        &self,
        query: &str,
        k: usize,
        filters: Option<serde_json::Value>,
    ) -> crate::Result<Vec<String>>;
}
```

and an in‑memory implementation (`InMemoryMemory`) used for demos and tests.

This document formalizes these traits as the **primary memory extension point** for Loom.

---

### Memory Types

The memory system is intended to support three conceptual layers:

1. **Episodic memory**
   - Sequences of events or summaries scoped to a session/thread.
   - Backed by RocksDB or another KV store for durability.
   - Used to reconstruct recent context and conversation history.
2. **Semantic memory**
   - Embedding‑based retrieval over documents, facts, or past interactions.
   - Backed by a vector store (FAISS/Milvus/Weaviate, or pluggable adapters).
   - Used to recall relevant knowledge given a query.
3. **Working memory**
   - Short‑lived, in‑process state used by a single agent while handling events.
   - Typically stored in the agent's internal state or ephemeral caches.

The `MemoryReader/Writer` traits cover episodic and a simplified view of semantic memory; working memory remains an agent‑local concern.

---

### Planned Implementations

#### 1. RocksDB‑backed episodic memory

A first step is to provide a persistent `MemoryReader/Writer` implementation backed by RocksDB:

- Keys scoped by `session_id` and (optionally) time buckets.
- Values as compact textual summaries (similar to `InMemoryMemory`, but durable).
- Configurable retention and compaction policies.

This implementation is suitable for:

- Reconstructing recent dialogues for LLM prompts.
- Simple time‑ordered inspection and debugging.

#### 2. Pluggable semantic memory provider

Longer term, Loom should support semantic retrieval via a separate trait, e.g.:

```rust
#[async_trait::async_trait]
pub trait SemanticMemory: Send + Sync {
    async fn upsert(
        &self,
        session: &str,
        id: &str,
        text: &str,
        metadata: serde_json::Value,
    ) -> crate::Result<()>;

    async fn query(
        &self,
        session: &str,
        query: &str,
        k: usize,
        filters: Option<serde_json::Value>,
    ) -> crate::Result<Vec<String>>;
}
```

Concrete adapters can then wrap FAISS, Milvus, or other backends.

---

### Integration with ContextBuilder

`ContextBuilder` will remain the primary consumer of memory when constructing prompts:

- It can call `MemoryWriter::summarize_episode` to include a **recent episode summary**.
- It can call `MemoryReader::retrieve` (and, later, semantic providers) to include **retrieved context** lines.
- It assembles these pieces into a `PromptBundle` that LLM clients can consume.

Future enhancements may include:

- Token‑aware truncation and budgeting.
- Role‑annotated histories (user/assistant/tool) derived from episodic memory.

---

### Memory and Events

To tie memory into the event system and persistence story, P2 introduces **standardized memory topics/events**:

- Write/update events:
  - Topic: `memory.{session_id}.update`
  - Payload: event or summary to be written via `MemoryWriter`/semantic provider.
- Retrieval events:
  - Topic: `memory.{session_id}.retrieve`
  - Payload: query parameters; response delivered on a reply topic or via an action result.

A dedicated memory agent or service can subscribe to these topics and delegate to the configured memory backends, making memory operations:

- Visible on the Dashboard (as normal events),
- Easier to route and scale,
- Easier to test in isolation.

---

### Cognitive Agents and Memory

Cognitive agents (see `cognitive_runtime.md`) are expected to:

- Call `append_event` from their `perceive` stage to keep episodic memory up to date.
- Use `summarize_episode` and `retrieve` during `think` to build rich context.
- Optionally emit `memory.update` / `memory.retrieve` events instead of calling traits directly, when cross‑process memory services are used.

This keeps the cognitive layer and memory layer loosely coupled while preserving a clear contract between them.

---

### Observability and Tuning

Planned observability features include:

- Metrics:
  - Read/write QPS per memory backend.
  - Retrieval latency and hit ratios.
  - Storage size and retention effectiveness.
- Tracing:
  - Spans for `append_event`, `summarize_episode`, `retrieve`, and semantic queries.
  - Links between event processing spans and memory operations.

Tuning knobs:

- Retention windows per agent or namespace.
- Maximum items per session and eviction strategies.
- Backend‑specific configuration (e.g., index parameters for vector stores).
