# Context Module

Builds LLM-ready prompts from agent state and memory.

## Overview

The context layer turns recent experience and retrieval results into a PromptBundle that the LLM client can consume. It’s intentionally small and pluggable for P0: an in-memory store and a minimal builder that surfaces an episode summary and a few retrieved lines.

## Key types

- TokenBudget: max_input_tokens, max_output_tokens used to keep payloads bounded.
- PromptBundle: system, instructions, optional tools_json_schema, context_docs (Vec<String>), history (Vec<String>).
- MemoryWriter: append_event(session, Event) and summarize_episode(session) for episodic summaries.
- MemoryReader: retrieve(query, k, filters) for simple retrieval.

## In-memory store

`memory.rs` provides `InMemoryMemory`, an in-process implementation of MemoryReader/Writer for demos and tests. It stores lightweight textual summaries per session and supports naive substring retrieval.

## ContextBuilder

`builder.rs` assembles a PromptBundle from the current session:

- Adds a “Recent episode summary” if available via MemoryWriter::summarize_episode.
- Adds “Retrieved context” lines from MemoryReader::retrieve using the goal string.
- Leaves history empty at P0 (dialog-turn tracking can be added later).

Trigger input:

- session_id: scope for memory operations
- goal: optional string used as the instruction and retrieval query
- tool_hints: optional hints (reserved for future tool selection)
- budget: TokenBudget to inform downstream budgeting

## Usage (minimal)

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

The resulting `PromptBundle` can be passed to the LLM client which converts it into chat messages or a single fused input.

## Contracts and edge cases

- Retrieval and summaries are best-effort; missing memory simply yields an empty context_docs.
- Budgeting is enforced later by the LLM adapter; ContextBuilder does not truncate strings.
- History is not role-annotated in P0; when dialog tracking is added, prefer role-aware entries.

## Extensibility

- Plug in persistent MemoryReader/Writer backed by RocksDB/Vector DB.
- Add role-aware history and tokenizer-aware budgeting.
- Include tool/function schemas in PromptBundle for tool calling flows.
