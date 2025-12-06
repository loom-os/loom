# Offloaded File Lifecycle

> **Complete lifecycle model for offloaded files in Loom**
>
> From creation → reference → retrieval → expiration → cleanup

## Overview

Offloading's core goal is to **remove unnecessary heavy content from LLM prompts** while maintaining **reliable retrieval, traceability, verifiability, and controllable cleanup**.

This document describes the complete lifecycle of offloaded files in Loom's event-driven, multi-agent architecture with RocksDB memory and workspace isolation.

---

## The 8-Phase Lifecycle

```
1. Creation          → Tool writes file
2. Registration      → Metadata written to working memory / index
3. Reference         → "Result saved to file X" in context
4. Retrieval         → LLM reads on-demand via tool
5. Promotion         → Short-term → Long-term memory
6. Expiration        → TTL-based marking
7. Garbage Collection → Automatic cleanup
8. Archival          → Optional long-term storage
```

---

## Phase 1: Creation (File Offload Storage)

### Current Implementation ✅

When a tool executes and output exceeds thresholds:

```python
# loom-py/src/loom/context/offloader.py
result = offloader.offload(
    content=large_output,
    category="search",      # file_read, shell_output, web, search
    identifier="query_123"
)
```

**File Location:**

```
workspace/
  .loom/
    cache/
      search/
        websearch_<timestamp>_<hash>.json
      file_read/
        file_<name>_<hash>.txt
      shell_output/
        shell_<timestamp>_<hash>.txt
      web/
        web_<url>_<hash>.html
```

**Thresholds (configurable):**

- Size: > 2KB (default)
- Lines: > 50 lines (default)

**File Naming:**

```
<category>_<timestamp>_<content_hash>.<ext>
└─┬─────┘ └────┬─────┘ └───────┬────────┘ └┬┘
  │            │                │            └─ Extension
  │            │                └─ First 16 chars of SHA-256
  │            └─ Unix timestamp (deduplication across time)
  └─ Category (enables cleanup by type)
```

### Planned Enhancements 📋

**Task-scoped offloading:**

```python
# Future: workspace/offload/<task_id>/<step_id>_<hash>.json
workspace/
  .loom/
    offload/           # Structured by task
      task_001/
        step_001_full.json
        step_001_compact.json
        step_001_raw.txt
      task_002/
        ...
```

**Benefits:**

- ✅ Isolate files per task
- ✅ Easy bulk deletion when task completes
- ✅ Better context for retrieval

### Responsibility Split

| Layer     | Responsibility                                             |
| --------- | ---------------------------------------------------------- |
| Python    | Decision logic (what/when to offload), file format, naming |
| Rust Core | Safe file write, sandboxed I/O, return path                |

---

## Phase 2: Registration (Metadata Indexing)

### Current Implementation 🚧 Partial

Offloaded files are referenced in `Step.outcome_ref`:

```python
# loom-py/src/loom/context/step.py
@dataclass
class Step:
    id: str
    tool_name: str
    minimal_args: dict
    observation: str           # One-line summary
    timestamp_ms: int
    outcome_ref: Optional[str] # File path if offloaded ✅
    success: bool
    error: Optional[str]
    metadata: dict
```

**Current flow:**

1. Tool executes → `DataOffloader.offload()` returns `OffloadResult`
2. `StepReducer` creates `Step` with `outcome_ref=file_path`
3. `Step` added to `CognitiveAgent.steps` list
4. Prompt builder checks `step.outcome_ref` for offload references

### Planned: Explicit Offload Index 📋

Create a dedicated metadata store for offloaded files:

```python
# Future: loom-py/src/loom/context/offload_index.py
class OffloadIndex:
    """Persistent index of offloaded files for retrieval."""

    def register(
        self,
        file_id: str,
        path: str,
        step_id: str,
        task_id: Optional[str],
        metadata: dict,
    ) -> None:
        """Register an offloaded file."""
        entry = {
            "path": path,
            "step_id": step_id,
            "task_id": task_id,
            "timestamp": time.time(),
            "size": os.path.getsize(path),
            "hash": compute_content_hash(open(path).read()),
            "category": metadata.get("category"),
            "ttl_hours": metadata.get("ttl_hours", 24),
            "expires_at": time.time() + metadata.get("ttl_hours", 24) * 3600,
        }
        # Store in RocksDB or JSON index
        self._store[file_id] = entry

    def get(self, file_id: str) -> Optional[dict]:
        """Retrieve metadata for a file."""
        return self._store.get(file_id)

    def list_expired(self) -> list[str]:
        """List all expired file IDs."""
        now = time.time()
        return [fid for fid, meta in self._store.items()
                if meta["expires_at"] < now]
```

**Why this matters:**

> **Manus Context Engineering insight:**
>
> "Context Reduction depends on your ability to reliably continue accessing original data."

If offloaded data is lost:

- ❌ Compaction fails
- ❌ Summarization fails
- ❌ LLM reasoning chain breaks
- ❌ Multi-agent collaboration corrupts
- ❌ Large tasks (crawling, coding, analysis) cannot resume

### Storage Options

| Option         | Pros                   | Cons                      | Status       |
| -------------- | ---------------------- | ------------------------- | ------------ |
| In-memory dict | Fast, simple           | Lost on restart           | ✅ Current   |
| JSON file      | Persistent, readable   | Not concurrent-safe       | 📋 Next step |
| RocksDB        | Persistent, concurrent | Requires Rust integration | 📋 Phase 3   |

---

## Phase 3: Reference in Context (Lightweight Pointers)

### Current Implementation ✅

When building ReAct prompt, only include reference:

```python
# loom-py/src/loom/cognitive/loop.py
def build_react_prompt(...) -> str:
    for step in steps:
        if step.outcome_ref:
            # Show file path, not content
            lines.append(f"Result saved to {step.outcome_ref}")
            lines.append(f"Summary: {step.observation}")
        else:
            lines.append(step.observation)
```

**LLM sees:**

```
Tool: web:search
Arguments: {"query": "google AI pricing"}
Result saved to .loom/cache/search/websearch_1765003729.json
Summary: Found 23 results about Google AI pricing

Use fs:read_file to retrieve if needed.
```

**Token savings:**

- Before: 15,000 tokens (full search results)
- After: 50 tokens (reference + summary)
- **Reduction: 99.7%**

---

## Phase 4: Retrieval (On-Demand Access)

### Current Implementation ✅

LLM explicitly requests file via tool:

```
Thought: I need detailed pricing info from the search results.
Action: fs:read_file
Arguments: {"path": ".loom/cache/search/websearch_1765003729.json"}
```

Python executes:

```python
# loom-py/src/loom/cognitive/agent.py
result = await ctx.tool(
    "fs:read_file",
    {"path": ".loom/cache/search/websearch_1765003729.json"}
)
```

**Key design principles:**

✅ Retrieval is **explicit** (not automatic)
✅ LLM accesses via **tool calls** (not prompt injection)
✅ Prompt **never polluted** with full content
✅ Works across agent restarts (if index persists)

---

## Phase 5: Promotion (Short-term → Long-term)

### Current Implementation ❌ Not Yet

This is a critical capability that most systems miss, but **Manus emphasizes**:

Some offloaded files should graduate from working memory to long-term memory:

**Examples:**

- Analysis reports
- User intent files
- Configuration snapshots
- Intermediate task artifacts (scraped databases, compiled results)

### Planned Design 📋

```python
# Future API
class OffloadIndex:
    def promote(
        self,
        file_id: str,
        tier: MemoryTier = MemoryTier.LONG_TERM,
        category: str = "archive"
    ) -> str:
        """Promote file from working → long-term memory.

        Returns:
            New path in persistent storage
        """
        meta = self.get(file_id)
        new_path = (
            self.workspace
            / "persistent"
            / self.agent_id
            / category
            / Path(meta["path"]).name
        )

        # Move file
        shutil.move(meta["path"], new_path)

        # Update metadata
        meta["path"] = str(new_path)
        meta["tier"] = tier
        meta["ttl_hours"] = None  # No expiration
        self._store[file_id] = meta

        # Write to RocksDB for persistence
        self._persist_to_rocks(file_id, meta)

        return str(new_path)
```

**Directory structure after promotion:**

```
workspace/
  .loom/
    cache/              # Working memory (TTL)
      search/
        temp_*.json

  persistent/           # Long-term memory (no TTL)
    agent_researcher/
      reports/
        market_analysis_2025_01.md
      datasets/
        scraped_prices.json
      config/
        search_preferences.json
```

**Promotion triggers:**

| Trigger         | Example                              | Tier         |
| --------------- | ------------------------------------ | ------------ |
| User command    | `/save_report`                       | LONG_TERM    |
| LLM decision    | "This dataset might be useful later" | SHORT_TERM   |
| Task completion | Final analysis report                | LONG_TERM    |
| Explicit API    | `agent.promote_file(file_id)`        | Configurable |

---

## Phase 6: Expiration (TTL-based)

### Current Implementation 🚧 Partial

`OffloadConfig` has `max_age_hours` field but **not enforced yet**:

```python
@dataclass
class OffloadConfig:
    max_age_hours: int = 24  # ✅ Defined, ❌ Not enforced
```

### Planned TTL Strategy 📋

Different file types need different lifetimes:

| File Type        | Default TTL | Rationale                         |
| ---------------- | ----------- | --------------------------------- |
| Shell output     | Minutes     | High noise, low value             |
| Search results   | Hours       | Hot data but goes stale           |
| Web scrapes      | Hours       | Can re-scrape if needed           |
| Session notes    | Days        | Context changes slowly            |
| Reports/analysis | Never       | Real value, needs manual deletion |

**Metadata structure:**

```python
{
  "file_id": "search_abc123",
  "path": ".loom/cache/search/websearch_xyz.json",
  "created_at": 1765003729,
  "ttl_hours": 6,
  "expires_at": 1765025329,  # created_at + ttl
  "tier": "WORKING",
  "promoted": False,
}
```

**Expiration logic:**

```python
def is_expired(self, file_id: str) -> bool:
    """Check if file has expired."""
    meta = self.get(file_id)
    if not meta:
        return True

    # Promoted files never expire
    if meta.get("tier") == MemoryTier.LONG_TERM:
        return False

    # Check TTL
    now = time.time()
    return now > meta.get("expires_at", 0)
```

---

## Phase 7: Garbage Collection (Automatic Cleanup)

### Current Implementation ❌ Not Yet

No automatic cleanup exists. Files persist until manual deletion.

### Planned GC Strategy 📋

**Conditions for deletion:**

```python
def should_delete(self, file_id: str) -> bool:
    """Check if file should be garbage collected."""
    meta = self.get(file_id)

    return all([
        self.is_expired(file_id),           # ✅ Past TTL
        not self.is_referenced(file_id),    # ✅ Not in active workflow
        not meta.get("important", False),   # ✅ Not marked important
        meta["tier"] != MemoryTier.LONG_TERM,  # ✅ Not promoted
    ])

def is_referenced(self, file_id: str) -> bool:
    """Check if file is referenced in active context."""
    # Check if in compacted step history
    for step in self.compactor.get_recent_steps():
        if step.outcome_ref and file_id in step.outcome_ref:
            return True

    # Check if in prompt construction
    if file_id in self._active_file_ids:
        return True

    return False
```

**GC Execution:**

```python
def run_garbage_collection(self) -> dict:
    """Run garbage collection on offloaded files.

    Returns:
        Stats dict with deleted_count, freed_bytes, etc.
    """
    deleted = []
    freed_bytes = 0

    for file_id in self.list_all():
        if self.should_delete(file_id):
            meta = self.get(file_id)
            path = Path(meta["path"])

            if path.exists():
                freed_bytes += path.stat().st_size
                path.unlink()

            self._store.pop(file_id)
            deleted.append(file_id)

    return {
        "deleted_count": len(deleted),
        "freed_bytes": freed_bytes,
        "freed_mb": freed_bytes / (1024 * 1024),
        "deleted_files": deleted,
    }
```

**GC Triggers:**

| Trigger            | When                        | Responsible Layer           |
| ------------------ | --------------------------- | --------------------------- |
| Periodic           | Every N minutes             | Rust Core (background task) |
| On session start   | Agent initialization        | Python Cognitive Layer      |
| On memory pressure | Cache > max_size_mb         | Python OffloadIndex         |
| Manual             | User command `/clean_cache` | Python CLI                  |

---

## Phase 8: Archival (Optional Long-term Storage)

### Current Implementation ❌ Not Yet

### Planned Design 📋

When user or LLM explicitly requests archival:

```python
# User command
/archive_report market_analysis_2025_01.md

# Or LLM decision
Thought: This dataset will be useful for future analysis.
Action: archive_file
Arguments: {"file_id": "search_abc123", "category": "datasets"}
```

**Implementation:**

```python
def archive(
    self,
    file_id: str,
    category: str = "general",
    tags: list[str] = None,
) -> str:
    """Archive file for long-term storage.

    Args:
        file_id: File to archive
        category: Archive category (reports, datasets, config)
        tags: Search tags

    Returns:
        Path in archive
    """
    meta = self.get(file_id)
    archive_path = (
        self.workspace
        / "archive"
        / category
        / Path(meta["path"]).name
    )

    # Move file
    archive_path.parent.mkdir(parents=True, exist_ok=True)
    shutil.move(meta["path"], archive_path)

    # Update metadata
    meta["archived"] = True
    meta["archive_path"] = str(archive_path)
    meta["archive_category"] = category
    meta["archive_tags"] = tags or []
    meta["archived_at"] = time.time()

    # Write to RocksDB with indexing
    self._index_for_retrieval(file_id, meta)

    return str(archive_path)
```

**Archive directory structure:**

```
workspace/
  archive/
    reports/
      market_analysis_2025_01.md
      competitor_research_2025_01.pdf
    datasets/
      price_history_btc.json
      scraped_news_articles.json
    config/
      trading_strategy_v1.json
```

**Retrieval from archive:**

```python
# Semantic search in archive
results = index.search_archive(
    query="Bitcoin price analysis",
    category="reports",
    limit=5
)

# Tag-based search
results = index.search_by_tags(
    tags=["market", "crypto"],
    category="datasets"
)
```

---

## Lifecycle Summary

```
┌─────────────┐
│  Creation   │ Tool writes to .loom/cache/<category>/
└──────┬──────┘
       │
       ▼
┌─────────────┐
│Registration │ Metadata → index (+ RocksDB in future)
└──────┬──────┘
       │
       ▼
┌─────────────┐
│ Reference   │ Prompt shows "Saved to X" (not content)
└──────┬──────┘
       │
       ▼
┌─────────────┐
│ Retrieval   │ LLM calls fs:read_file explicitly
└──────┬──────┘
       │
       ├───────────────────┐
       │                   │
       ▼                   ▼
┌─────────────┐    ┌─────────────┐
│ Promotion   │    │ Expiration  │
│ (SHORT/LONG)│    │  (TTL check)│
└──────┬──────┘    └──────┬──────┘
       │                   │
       ▼                   ▼
┌─────────────┐    ┌─────────────┐
│  Archival   │    │     GC      │
│ (preserve)  │    │  (delete)   │
└─────────────┘    └─────────────┘
```

---

## Implementation Status

| Phase           | Status         | Location       | Priority |
| --------------- | -------------- | -------------- | -------- |
| 1. Creation     | ✅ Complete    | `offloader.py` | P0       |
| 2. Registration | 🚧 Partial     | `step.py`      | P0       |
| 3. Reference    | ✅ Complete    | `loop.py`      | P0       |
| 4. Retrieval    | ✅ Complete    | `agent.py`     | P0       |
| 5. Promotion    | ❌ Not started | -              | P1       |
| 6. Expiration   | 🚧 Partial     | `offloader.py` | P1       |
| 7. GC           | ❌ Not started | -              | P1       |
| 8. Archival     | ❌ Not started | -              | P2       |

**Legend:**

- ✅ Complete and tested
- 🚧 Partial implementation
- ❌ Planned but not started

---

## Architecture Layers

### Python Cognitive Layer (Brain 🧠)

**Responsible for:**

- Complete lifecycle management
- TTL enforcement
- GC scheduling
- Promotion decisions
- Metadata management
- Pointer injection in prompts
- Compaction / reduction

**Files:**

- `loom-py/src/loom/context/offloader.py` (Creation)
- `loom-py/src/loom/context/offload_index.py` (Registration, GC) 📋 Future
- `loom-py/src/loom/cognitive/loop.py` (Reference)
- `loom-py/src/loom/cognitive/agent.py` (Retrieval)

### Rust Core (Hands 🤚)

**Responsible for:**

- Safe file I/O
- Sandboxed execution
- Event bus for cross-agent coordination
- RocksDB persistence (long-term tier)
- Background GC tasks

**Files:**

- `core/src/tools/filesystem.rs` (Creation)
- `bridge/src/memory_handler.rs` (Persistence)

---

## Design Rationale

### Why This Full Lifecycle?

Because **Manus identified the key insight**:

> "Context Reduction relies on being able to reliably continue accessing original data."

If offloaded data is lost or unrecoverable:

- ❌ **Compaction fails** — Can't summarize what's gone
- ❌ **Summarization fails** — No source data to work from
- ❌ **LLM reasoning breaks** — Missing context in chain
- ❌ **Multi-agent collaboration corrupts** — Shared refs fail
- ❌ **Large tasks can't resume** — Crawling, coding, analysis all need state

Therefore, files must have:

- ✅ **Persistence** — Survive restarts
- ✅ **Recoverability** — Indexed and searchable
- ✅ **Controllability** — Explicit lifecycle management

### Why 8 Phases Instead of 3?

Simple systems only do: Create → Reference → Delete

But this breaks when:

- Agent restarts mid-task
- Multi-agent workflows share data
- Long-running tasks span days
- User wants to keep specific artifacts

A **production-grade** offloading system needs all 8 phases to enable agents to be **long-running systems**, not one-shot scripts.

---

## Next Steps

**Immediate (P0):**

1. ✅ Complete Phase 2: Implement `OffloadIndex` with JSON persistence
2. ✅ Add Phase 6: TTL enforcement in `is_expired()`
3. ✅ Add Phase 7: Basic GC with `run_garbage_collection()`

**Near-term (P1):**

1. Implement Phase 5: Promotion API
2. Add task-scoped offloading (`.loom/offload/<task_id>/`)
3. RocksDB integration for long-term tier
4. Background GC in Rust Core

**Future (P2):**

1. Phase 8: Archival with semantic search
2. Embedding-based retrieval
3. Cross-agent offload sharing
4. Compression for old files

---

## See Also

- [Context Engineering Design](DESIGN.md)
- [Offloading Patterns](OFFLOADING.md)
- [Offload Management Guide](OFFLOAD_MANAGEMENT.md)
- [Context Integration](CONTEXT_INTEGRATION.md)
- [ROADMAP](../../ROADMAP.md)
