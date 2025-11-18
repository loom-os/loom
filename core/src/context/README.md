# Context Module

Builds LLM-ready prompts from agent state and memory.

## Overview

The context layer turns recent experience and retrieval results into a PromptBundle that the LLM client can consume. It provides both **general-purpose memory** (MemoryReader/Writer for episodic summaries) and **agent-specific memory** for decision tracking, deduplication, and execution idempotency.

## Key types

- **TokenBudget**: max_input_tokens, max_output_tokens used to keep payloads bounded.
- **PromptBundle**: system, instructions, optional tools_json_schema, context_docs (Vec<String>), history (Vec<String>).
- **MemoryWriter**: append_event(session, Event) and summarize_episode(session) for episodic summaries.
- **MemoryReader**: retrieve(query, k, filters) for simple retrieval.

## Memory implementations

### InMemoryMemory (General + Agent-specific)

`memory.rs` provides `InMemoryMemory`, an in-process implementation supporting:

1. **General-purpose memory** (MemoryReader/Writer):

   - Episodic summaries per session
   - Naive substring retrieval

2. **Agent decision memory**:
   - `save_plan()`: Store agent decisions with deduplication hashing
   - `get_recent_plans()`: Query historical decisions for context
   - `check_duplicate()`: Detect duplicate decisions within time windows
   - `mark_executed()` / `check_executed()`: Track execution idempotency
   - `get_execution_stats()`: Calculate win rates and success metrics

Used in `market-analyst` demo for Planner/Executor coordination.

## ContextBuilder

`builder.rs` assembles a PromptBundle from the current session:

- Adds a "Recent episode summary" if available via MemoryWriter::summarize_episode.
- Adds "Retrieved context" lines from MemoryReader::retrieve using the goal string.
- Leaves history empty at P0 (dialog-turn tracking can be added later).

Trigger input:

- session_id: scope for memory operations
- goal: optional string used as the instruction and retrieval query
- tool_hints: optional hints (reserved for future tool selection)
- budget: TokenBudget to inform downstream budgeting

## Usage

### General-purpose memory (episodic summaries)

```rust
use loom_core::context::{memory::InMemoryMemory, builder::{ContextBuilder, TriggerInput}, TokenBudget};
use std::sync::Arc;

let mem = InMemoryMemory::new();
let builder = ContextBuilder::new(Arc::clone(&mem), Arc::clone(&mem));

let bundle = builder.build(TriggerInput {
    session_id: "s1".into(),
    goal: Some("Summarize the last interactions".into()),
    tool_hints: vec![],
    budget: TokenBudget::default(),
}).await?;
```

### Agent decision memory (Planner/Executor pattern)

```rust
use loom_core::context::memory::InMemoryMemory;
use std::sync::Arc;

let memory = Arc::new(InMemoryMemory::new());

// Planner: Check for duplicates before saving
let (is_dup, _) = memory.check_duplicate(
    "session-1", "BTC", "BUY", "Bullish trend", 300
).await?;

if !is_dup {
    let hash = memory.save_plan(
        "session-1", "BTC", "BUY", 0.85, "Bullish trend", "llm"
    ).await?;

    // Pass plan_hash to Executor...
}

// Executor: Check idempotency before execution
let (already_exec, _) = memory.check_executed("session-1", &plan_hash).await?;

if !already_exec {
    // Execute order...
    memory.mark_executed(
        "session-1", &plan_hash, "BTC", "BUY", 0.85,
        "success", true, "order-123", 100.0
    ).await?;
}

// Query statistics
let stats = memory.get_execution_stats("session-1", "BTC").await?;
println!("Win rate: {:.1}%", stats.win_rate * 100.0);
```

The resulting `PromptBundle` can be passed to the LLM client which converts it into chat messages or a single fused input.

## Architecture

- **Core Storage**: DashMap-based concurrent storage for both episodic and agent decision data
- **Dual Interface**: Implements both legacy traits (MemoryReader/Writer) and new agent-specific methods
- **Session Isolation**: All data partitioned by session_id for multi-agent/multi-thread safety
- **gRPC Bridge**: Memory service exposed via Bridge for Python SDK and external agents
- **Deduplication**: MD5-based plan hashing with configurable time windows
- **Idempotency**: Execution tracking prevents duplicate order submissions

See `docs/core/memory.md` for detailed design and implementation.

## Contracts and edge cases

- Retrieval and summaries are best-effort; missing memory simply yields an empty context_docs.
- Budgeting is enforced later by the LLM adapter; ContextBuilder does not truncate strings.
- History is not role-annotated in P0; when dialog tracking is added, prefer role-aware entries.

## Extensibility

- Plug in persistent MemoryReader/Writer backed by RocksDB/Vector DB.
- Add role-aware history and tokenizer-aware budgeting.
- Include tool/function schemas in PromptBundle for tool calling flows.
