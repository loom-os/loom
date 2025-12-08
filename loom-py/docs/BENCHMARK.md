# Benchmark System Documentation

## Overview

Loom's benchmark system validates agent capabilities and measures the impact of context engineering on performance, cost, and quality. This system is crucial for Phase 2 of the roadmap (Context Engineering & Research).

## Architecture

```
Benchmark System
├── Core Components
│   ├── BenchmarkRunner       - Orchestrates benchmark execution
│   ├── TaskMetrics          - Per-task performance metrics
│   ├── BenchmarkResults     - Aggregated results
│   └── ComparisonReport     - Baseline vs optimized comparison
│
├── Adapters (Dataset-specific)
│   ├── SWEBenchAdapter      - Software engineering tasks
│   ├── WebArenaAdapter      - Web interaction tasks (planned)
│   └── GAIAAdapter          - General assistant tasks (planned)
│
└── CLI Commands
    ├── loom benchmark run     - Run single benchmark
    ├── loom benchmark compare - Compare baseline vs optimized
    └── loom benchmark report  - Generate reports
```

## Key Features

### 1. Token Usage Tracking

The system tracks comprehensive token metrics:

- `total_tokens`: Total tokens across all LLM calls
- `prompt_tokens`: Input tokens
- `completion_tokens`: Output tokens
- `avg_prompt_tokens`: Average prompt size per iteration
- `peak_prompt_tokens`: Maximum prompt size encountered

**Implementation**: Token tracking added to `CognitiveResult` and collected from `LLMProvider._last_usage`.

### 2. Context Engineering Evaluation

Measures the impact of:

- **Step Reduction**: Minimal observation rules per tool
- **Step Compaction**: History compression for long conversations
- **Data Offloading**: Heavy output moved to workspace files

**Metrics**:

- `offloaded_files`: Number of files created for data offloading
- `compacted_steps`: Number of steps compressed
- `token_savings_pct`: Estimated token savings percentage

### 3. Correctness Evaluation

For synthetic tasks:

- Executes test files to verify fixes
- Checks for "All tests passed!" in output
- Returns boolean correctness score

For real SWE-bench:

- Runs actual test suites from dataset
- Compares test pass rates

### 4. Cost Calculation

Estimates LLM costs based on token usage:

- DeepSeek pricing: $0.14/1M input, $0.28/1M output tokens
- Calculates per-task and average costs
- Enables cost/quality trade-off analysis

## Usage

### Quick Start

```bash
# 1. Run benchmark with synthetic tasks
loom benchmark run swe-bench \
    --agent-path ./apps/chat-assistant \
    --dataset ./datasets/swe-bench-lite \
    --max-tasks 3 \
    --output results/run1.json \
    --verbose

# 2. Run comparison (baseline vs optimized)
loom benchmark compare swe-bench \
    --agent-path ./apps/chat-assistant \
    --dataset ./datasets/swe-bench-lite \
    --max-tasks 5 \
    --output results/comparison.json

# 3. Generate report
loom benchmark report results/comparison.json
```

### Automated Testing

```bash
# Run test script
./scripts/test_benchmark.sh
```

## Synthetic Tasks

When no dataset file is present, the system generates synthetic Python debugging tasks:

### Task 1: Average Calculation Bug

```python
# Bug: Off-by-one error
return total / (len(numbers) + 1)  # Should be: len(numbers)
```

### Task 2: String Reversal

```python
# Bug: Reverses characters instead of words
return sentence[::-1]  # Should: return " ".join(sentence.split()[::-1])
```

### Task 3: Duplicate Removal

```python
# Bug: Doesn't preserve order
return list(set(items))  # Should use dict.fromkeys() or manual deduplication
```

Each task includes:

- Problem statement
- Buggy source code
- Test file with assertions
- Expected behavior

## Benchmark Workflow

```
1. Load Tasks
   ├─> Read from dataset JSON or generate synthetic
   └─> Normalize task format

2. Prepare Task
   ├─> Create workspace directory
   ├─> Write source files
   └─> Set up agent context

3. Run Agent
   ├─> Execute cognitive loop
   ├─> Track token usage
   └─> Collect metrics

4. Evaluate Result
   ├─> Run test suite
   ├─> Check correctness
   └─> Record pass/fail

5. Aggregate Metrics
   ├─> Calculate statistics
   ├─> Compute token savings
   └─> Generate report
```

## Metrics Reference

### TaskMetrics

| Field                | Type  | Description                             |
| -------------------- | ----- | --------------------------------------- |
| `task_id`            | str   | Unique task identifier                  |
| `success`            | bool  | Whether agent completed without error   |
| `iterations`         | int   | Number of cognitive loop iterations     |
| `total_tokens`       | int   | Total tokens used (prompt + completion) |
| `total_cost_usd`     | float | Estimated cost in USD                   |
| `execution_time_sec` | float | Wall-clock time in seconds              |
| `offloaded_files`    | int   | Number of files created for offloading  |
| `compacted_steps`    | int   | Number of steps compacted               |
| `avg_prompt_tokens`  | int   | Average prompt size                     |
| `peak_prompt_tokens` | int   | Maximum prompt size                     |
| `token_savings_pct`  | float | Estimated token reduction percentage    |
| `correct_answer`     | bool  | Whether solution passed tests           |
| `error_message`      | str   | Error details if failed                 |

### BenchmarkResults

Aggregates metrics across all tasks:

```python
results.success_rate       # % of tasks that completed
results.correctness_rate   # % of tasks with correct solutions
results.avg_tokens         # Average tokens per task
results.avg_cost           # Average cost per task (USD)
results.avg_time           # Average execution time (seconds)
results.total_offloaded_files    # Total files offloaded
results.total_compacted_steps    # Total steps compacted
results.avg_token_savings        # Average token savings %
```

### ComparisonReport

Compares baseline (no context engineering) vs optimized:

```python
report.token_reduction()   # % token reduction
report.cost_reduction()    # % cost reduction
report.quality_delta()     # Change in correctness rate
```

## Integration with Roadmap

This benchmark system supports **Phase 2: Context Engineering & Research** goals:

### ✅ Completed (P0)

- Token usage tracking in CognitiveResult
- Cost calculation based on actual LLM pricing
- Synthetic task generation for testing
- Correctness evaluation via test execution
- Metrics collection and aggregation

### 🚧 In Progress (P1)

- SWE-bench dataset integration (basic done, needs full implementation)
- Offload lifecycle tracking (Phase 1-4 done, need Phase 5-8)
- Multi-agent isolation testing

### 📋 Planned (P2)

- WebArena benchmark adapter
- GAIA benchmark adapter
- Semantic search for archived offloads
- Task-scoped offload management
- Long-term memory promotion API

## Extending the System

### Adding a New Benchmark

1. Create adapter in `loom-py/src/loom/benchmark/adapters/`:

```python
class MyBenchmarkAdapter:
    def load_tasks(self, max_tasks: int) -> list[dict]:
        """Load tasks from your dataset."""
        pass

    async def prepare_task(self, task: dict) -> Path:
        """Set up workspace for task."""
        pass

    async def evaluate_result(
        self, task: dict, result: CognitiveResult
    ) -> bool:
        """Evaluate if solution is correct."""
        pass
```

2. Register in `BenchmarkRunner._load_adapter()`:

```python
if benchmark == "my-benchmark":
    from .adapters.my_benchmark import MyBenchmarkAdapter
    return MyBenchmarkAdapter(self.dataset_path, self.workspace_path)
```

3. Add CLI option in `loom-py/src/loom/cli/benchmark.py`:

```python
run_parser.add_argument(
    "benchmark",
    choices=["swe-bench", "my-benchmark"],
    # ...
)
```

### Custom Metrics

Add fields to `TaskMetrics` in `metrics.py`:

```python
@dataclass
class TaskMetrics:
    # ... existing fields ...
    custom_metric: float = 0.0
```

Update `BenchmarkRunner._run_task()` to collect:

```python
custom_value = calculate_custom_metric(result)
return TaskMetrics(
    # ... existing params ...
    custom_metric=custom_value,
)
```

## Testing

### Unit Tests

```bash
pytest loom-py/tests/unit/test_benchmark.py -v
```

### Integration Tests

```bash
pytest loom-py/tests/integration/test_benchmark_integration.py -v
```

### Full Test Suite

```bash
./scripts/test_benchmark.sh
```

## Performance Considerations

### Token Tracking Overhead

- Minimal: ~1-2ms per LLM call
- Uses existing OpenTelemetry spans
- No additional API calls

### File I/O for Offloading

- Workspace files written to `<workspace>/<task_id>/`
- Cache files in `.loom/cache/`
- Cleaned up automatically per task

### Memory Usage

- Metrics stored in memory during run
- Serialized to JSON after completion
- ~1KB per task in memory

## Known Limitations

1. **Synthetic Tasks Only**: Real SWE-bench dataset integration is partial
2. **No Parallel Execution**: Tasks run sequentially (by design for determinism)
3. **Simple Cost Model**: Uses flat per-token pricing, doesn't account for caching
4. **Python-Only**: Current synthetic tasks are Python-specific

## Future Improvements

See ROADMAP.md Phase 2 for detailed plans:

- **P1 (Week 3)**:

  - Full SWE-bench dataset support
  - Offload lifecycle phases 5-8
  - Multi-agent context isolation

- **P2 (Week 4+)**:
  - WebArena and GAIA benchmarks
  - Semantic search in archived offloads
  - Hierarchical tool categorization
  - Embedding-based retrieval

## References

- [ROADMAP.md](/ROADMAP.md) - Overall project roadmap
- [loom-py/docs/context/DESIGN.md](/loom-py/docs/context/DESIGN.md) - Context engineering architecture
- [loom-py/docs/BENCHMARKING.md](/loom-py/docs/BENCHMARKING.md) - Benchmark strategy (if exists)
- [SWE-bench Paper](https://arxiv.org/abs/2310.06770) - Original benchmark paper

## Support

For questions or issues:

1. Check this documentation
2. Review test files in `loom-py/tests/`
3. See example usage in `scripts/test_benchmark.sh`
4. Open an issue on GitHub

---

_Last updated: 2025-12-08_
