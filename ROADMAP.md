# Loom Roadmap

**Vision**: Build an event-driven AI agent runtime that enables long-lifecycle, desktop/edge agents with proper context engineering and observable reasoning.

**Key Insight**: Loom is a **Runtime**, not a library. The differentiation from LangChain/CrewAI is:

- Long-running agents (not script execution)
- Event-driven triggers (not code calls)
- System integration (hotkeys, files, clipboard)
- Cross-process agent collaboration

---

## Architecture Principles

### Brain/Hand Separation

```
┌─────────────────────────────────────────────────────────────────────┐
│  Python (Brain 🧠)                  Rust Core (Hands 🤚)            │
│  ════════════════                   ══════════════════              │
│  • LLM calls (direct HTTP)          • Event Bus                     │
│  • Cognitive Loop (ReAct/CoT)       • Tool Registry + Sandbox       │
│  • Context Engineering              • Agent Lifecycle               │
│  • Memory strategies                • Persistent Store              │
│  • Business logic                   • System Integration            │
│                                     • MCP Proxy                     │
│  Fast iteration needed              Stable infrastructure           │
└─────────────────────────────────────────────────────────────────────┘
```

### Responsibility Matrix

| Component           | Rust Core    | Python SDK       | Agent Code |
| ------------------- | ------------ | ---------------- | ---------- |
| Event Bus           | ✅           | -                | -          |
| Tool Execution      | ✅ (sandbox) | -                | -          |
| Agent Lifecycle     | ✅           | -                | -          |
| Persistent Store    | ✅           | -                | -          |
| MCP Proxy           | ✅           | -                | -          |
| LLM Calls           | ❌           | ✅ (direct HTTP) | -          |
| Cognitive Loop      | ❌           | ✅               | -          |
| Context Engineering | ❌           | ✅               | -          |
| Business Logic      | -            | -                | ✅         |

---

## App Progression

```
App 1: Chat Assistant (MVP)           ✅ Working
    ↓ validates: brain/hand separation, direct LLM calls, tool use

App 2: Chat Assistant + Research      🚧 In Progress
    ↓ enhances: context engineering, multi-agent research

App 3: Market Analyst                 📋 Planned
    ↓ unlocks: long lifecycle, proactive agents, memory tiers

App 4: Desktop Assistant              📋 Planned
    ↓ unlocks: hotkeys, clipboard, system integration
```

---

## Phase 1: Foundation ✅ Complete

- [x] Python `LLMProvider` direct HTTP calls
- [x] Chat Assistant app working
- [x] Cognitive Loop with ReAct pattern
- [x] Tool calling via Rust Bridge
- [x] Streaming support

---

## Phase 2: Context Engineering & Research (Current)

**Core Insight**: Before multi-agent, we must build **production-grade context engineering**.
See `loom-py/docs/context/DESIGN.md` for full technical specification.

### Context Engineering Capability Map

```
Context Engineering in Loom
├── Reduction (Python)           ← P0: Token efficiency
│     ├── Step → CompactStep
│     ├── Minimal observation rules per tool
│     └── Heavy output → file offload
│
├── Compaction (Python)          ← P0: Stable compression ✅
│     ├── StepCompactor class ✅
│     ├── Threshold-based triggers (>5 steps) ✅
│     ├── Grouping & summarization ✅
│     └── Prompt integration ✅
│
├── Isolation (Python + Rust)    ← P1: Multi-agent ready
│     ├── Independent working memories
│     ├── agent.spawn / agent.result
│     └── No shared prompt context
│
├── Offloading (Rust + Python)   ← P1: Scalability (lifecycle design)
│     ├── Phase 1-4: Creation → Reference → Retrieval ✅
│     ├── Phase 5: Promotion (SHORT → LONG term) 📋
│     ├── Phase 6-7: TTL → Garbage Collection 📋
│     └── Phase 8: Archival with search 📋
│
├── Hierarchical Tools           ← P2: Simplify LLM
│     ├── L1: Function tools (LLM-facing)
│     ├── L2: Shell utilities
│     └── L3: Script APIs
│
└── Memory Architecture          ← P2: Long-term
      ├── Working memory
      ├── Short-term memory
      └── Long-term (RocksDB)
```

### Tasks by Priority

**P0: Core Context Quality (Week 1-2)** ✅ **COMPLETED**

| Task                   | Description                              | Status | Commit  |
| ---------------------- | ---------------------------------------- | ------ | ------- |
| 2.1 Step & CompactStep | Unified step model with reduction        | ✅     | 2741a77 |
| 2.2 StepReducer        | Tool-specific minimal observation rules  | ✅     | 2741a77 |
| 2.3 StepCompactor      | Step history compaction with grouping    | ✅     | 2741a77 |
| 2.4 File Offloading    | Heavy output → workspace files           | ✅     | 2741a77 |
| 2.5 Prompt Integration | Compaction in build_react_prompt         | ✅     | 785af0d |
| 2.6 Tool Descriptors   | Full parameter info in system prompt     | ✅     | 9185ec2 |
| 2.7 Few-Shot Examples  | Curated ReAct success patterns           | ✅     | 9185ec2 |
| 2.8 Agent Integration  | Auto reduction/offload in CognitiveAgent | ✅     | ae51993 |

**Key Metrics:**

- 210 unit tests passing (30 step + 17 compactor + 26 offloader + 22 tool descriptor + integration)
- ~2,000 lines of production code
- Token reduction: 29.4% (reduction) + 60-85% (compaction) = up to 90% total
- No negative impact on task completion (compaction tested in 50 tasks, 87% success rate)

**Documentation:**

- `context/DESIGN.md` - Overall architecture
- `context/REDUCTION.md` - Per-step reduction rules
- `context/COMPACTION.md` - Long conversation compression ✨ NEW
- `context/OFFLOADING.md` - Data offloading patterns
- `context/LIFECYCLE.md` - 8-phase offload lifecycle ✨ NEW
- `context/CONTEXT_INTEGRATION.md` - End-to-end integration guide
- `context/OFFLOAD_MANAGEMENT.md` - User guide for file management

**P1: Offload Lifecycle & Multi-Agent (Week 3)** 🚧 **IN PROGRESS**

| Task                     | Description                           | Status |
| ------------------------ | ------------------------------------- | ------ |
| 2.9 GAIA Benchmark       | General AI assistant evaluation       | 📋     |
| 2.10 TAU-bench           | Multi-agent tool coordination         | 📋     |
| 2.11 Offload Index       | JSON-based metadata persistence       | 📋     |
| 2.12 TTL & GC            | Automatic expiration and cleanup      | 📋     |
| 2.13 Promotion API       | SHORT_TERM → LONG_TERM tier promotion | 📋     |
| 2.14 Context Isolation   | Per-agent working memory              | 📋     |
| 2.15 Agent Spawning      | EventBus-based spawn/result           | 📋     |
| 2.16 Goal-only Prompting | No parent context leak                | 📋     |

**Benchmark System (2025-12-08):**

- ✅ Token tracking: Added to CognitiveResult with prompt/completion/total metrics
- ✅ Cost calculation: Accurate pricing based on DeepSeek rates ($0.14/$0.28 per 1M tokens)
- ✅ Generic framework: BenchmarkRunner, TaskMetrics, ComparisonReport
- ✅ Test infrastructure: Integration tests for runner and CLI
- ✅ Documentation: Comprehensive guide in `/docs/BENCHMARK.md`

**Strategic Pivot (2025-12-08):**

Removed SWE-bench adapter. **Rationale**: SWE-bench evaluates one-shot coding tasks (20% of Loom's value),
missing core runtime features like EventBus, multi-agent collaboration, long lifecycle, and desktop integration.

**New Priorities**:

- **GAIA** (P0): General AI assistant tasks, maps to chat-assistant app, evaluates tool coordination
- **TAU-bench** (P0): Multi-agent scenarios, maps to market-analyst app, tests EventBus and collaboration
- **SWE-bench** (P2, Optional): If we implement Docker harness for full evaluation (3-4 weeks effort)

See `docs/BENCHMARK_STRATEGY.md` for detailed comparison.

**P2: Advanced Features (Week 4+)** 📋

| Task                     | Description                       | Status |
| ------------------------ | --------------------------------- | ------ |
| 2.17 WebArena Benchmark  | Real-world web interaction tasks  | 📋     |
| 2.18 SWE-bench           | Software engineering tasks        | 📋     |
| 2.19 Archival System     | Semantic search in archived files | 📋     |
| 2.20 RocksDB Integration | Long-term offload metadata in DB  | 📋     |
| 2.21 Task-scoped Offload | `.loom/offload/<task_id>/` layout | 📋     |
| 2.22 Hierarchical Tools  | L1/L2/L3 action space             | 📋     |
| 2.23 Script Offloading   | python:run_script tool            | 📋     |
| 2.24 Semantic Ranking    | Embedding-based retrieval         | 📋     |

### Previous Completions

- [x] Workspace & file system (fs:read, fs:write, fs:list, fs:delete)
- [x] Human-in-the-loop approval
- [x] Shell command safety (60+ safe commands)
- [x] ReAct loop hallucination fixes
- [x] Web search (Brave API)

---

## Phase 3: Market Analyst App

**Objective**: Long-lifecycle trading system with proactive monitoring.

### Tasks

- [ ] Agent auto-restart on crash
- [ ] Memory tiers (working/short-term/long-term)
- [ ] Scheduled triggers
- [ ] OKX API integration

---

## Phase 4: Desktop Assistant App

**Objective**: Personal assistant with system integration.

### Tasks

- [ ] Global hotkey registration
- [ ] Clipboard monitoring
- [ ] System notifications
- [ ] Voice integration (loom-audio)

---

## Phase 5: Architecture Cleanup (Ongoing)

- [ ] Extract loom-dashboard
- [ ] Clean up Rust Core cognitive module
- [ ] `pip install loom` ready

---

## Timeline Summary

| Phase      | Duration  | Focus                    |
| ---------- | --------- | ------------------------ |
| Phase 1    | ✅        | Foundation               |
| Phase 2 P0 | 2 weeks   | Context Engineering Core |
| Phase 2 P1 | 1 week    | Multi-Agent              |
| Phase 2 P2 | 1 week    | Advanced                 |
| Phase 3    | 3-4 weeks | Market Analyst           |
| Phase 4    | 4 weeks   | Desktop                  |

---

## Design Documents

- `loom-py/docs/context/DESIGN.md` — Full Context Engineering specification
- `loom-py/docs/context/REDUCTION.md` — Step reduction & per-tool rules
- `loom-py/docs/context/COMPACTION.md` — Long conversation history compression
- `loom-py/docs/context/ISOLATION.md` — Multi-agent context isolation
- `loom-py/docs/context/OFFLOADING.md` — Data & logic offloading patterns
- `loom-py/docs/context/LIFECYCLE.md` — Complete 8-phase offload lifecycle
- `loom-py/docs/context/OFFLOAD_MANAGEMENT.md` — User guide for viewing/cleaning files
- `loom-py/docs/BENCHMARKING.md` — Agent benchmark strategy & integration ✨ NEW

---

_Last updated: 2025-12-05_
