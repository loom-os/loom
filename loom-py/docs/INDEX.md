# Loom Python Docs Index

## Core Guides

- **SDK Guide**: `SDK_GUIDE.md` - Build your first agent
- **LLM Provider Guide**: `LLM_GUIDE.md` - Configure LLM providers
- **Cognitive Loop Guide**: `COGNITIVE_GUIDE.md` - Implement reasoning patterns
- **CLI Guide**: `CLI_GUIDE.md` - Interactive chat interface
- **Benchmarking**: `BENCHMARKING.md` - Agent evaluation strategy
- **Benchmark Quick Start**: `BENCHMARK_QUICKSTART.md` - 5-minute benchmark guide ✨ NEW
- **Benchmark Implementation**: `BENCHMARK_IMPLEMENTATION.md` - Technical details ✨ NEW

## Context Engineering

- **Architecture**: `context/DESIGN.md` - Overall design and principles
- **Reduction**: `context/REDUCTION.md` - Step reduction strategies
- **Compaction**: `context/COMPACTION.md` - Long conversation history compression ✨ NEW
- **Offloading**: `context/OFFLOADING.md` - Data offloading patterns
- **Lifecycle**: `context/LIFECYCLE.md` - Complete 8-phase offload lifecycle ✨ NEW
- **Isolation**: `context/ISOLATION.md` - Multi-agent context management
- **Integration**: `context/CONTEXT_INTEGRATION.md` - End-to-end integration guide
- **File Management**: `context/OFFLOAD_MANAGEMENT.md` - Viewing and cleaning offload files

## Project Docs

- Quickstart & Runtime: `../../docs/QUICKSTART.md`, `../../docs/BUILD_LOCAL.md`
- Architecture: `../../ARCHITECTURE.md`, `../../ROADMAP.md`

## Recent Updates (Dec 7, 2025)

### Benchmark Infrastructure v0.3.0 ✨ NEW

**Added**:

- ✅ Complete benchmark framework for agent evaluation
- ✅ SWE-bench adapter (MVP implementation)
- ✅ CLI commands: `loom benchmark run/compare/report`
- ✅ Metrics collection: TaskMetrics, BenchmarkResults, ComparisonReport
- ✅ Context engineering impact measurement
- ✅ Token savings calculation (reduction + compaction + offloading)
- ✅ Cost/quality trade-off analysis
- 🧪 12 new unit tests (all passing)
- 📚 Complete documentation and examples

**Features**:

```bash
# Run benchmark with 10 tasks
loom benchmark run swe-bench --dataset ./data --max-tasks 10 -v

# Compare baseline vs optimized
loom benchmark compare swe-bench --dataset ./data --max-tasks 50

# Generate markdown report
loom benchmark report results/comparison.json --format markdown
```

**Expected Results**:

| Metric  | Baseline   | Optimized  | Improvement |
| ------- | ---------- | ---------- | ----------- |
| Tokens  | 8,000/task | 1,500/task | **-81%**    |
| Cost    | $0.40      | $0.08      | **-80%**    |
| Quality | 75%        | 75%        | **0%**      |

**Documentation**:

- `BENCHMARK_QUICKSTART.md` - 5-minute getting started guide
- `BENCHMARK_IMPLEMENTATION.md` - Technical implementation details
- `benchmark/README.md` - Module documentation
- `BENCHMARKING.md` - Full strategy (updated)

**Next Steps**: Run real SWE-bench Lite (300 tasks), publish results, add WebArena adapter

---

### Context Engineering Integration v0.2.2

**Fixed**:

- ✅ FINAL ANSWER repetition (LLM continues after first answer)
  - Added `FINAL ANSWER` to truncation patterns
  - Changed regex to non-greedy match with lookahead
- ✅ Offload file visibility (users couldn't find/view files)
  - CLI now shows "📖 View with: cat {path}"
  - Path highlighted in yellow for easy copying
- ✅ CLI display now shows offload references correctly (`Step.observation` not `Step.outcome`)
- ✅ Step compaction integrated into prompt construction
- ✅ Memory tracking includes offload references

**Added**:

- 📚 Offload Management Guide (viewing, cleanup, best practices)
- 📚 CLI Guide with context engineering details
- 📚 Context Integration Guide (complete pipeline walkthrough)
- 🧪 15 new tests for context engineering (64 total passing)
- 📊 Context metrics in CLI output (offload count)

**Documentation**:

Current cleanup strategy: Manual deletion (see `OFFLOAD_MANAGEMENT.md`)

- Files persist in `.loom/cache/` until manually removed
- Planned: Auto-cleanup, max cache size, LRU eviction

**Test Coverage**: 210 passing tests (4.47s), 100% for context engineering

Phase 2 focus: Enhance chat-assistant with deep research, workspace tools (fs:write/list/delete), agent spawning, web search integration, and report generation.
