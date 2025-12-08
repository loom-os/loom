# Benchmark Module

> Industry-standard benchmarking for Loom agents with context engineering evaluation

## Overview

The benchmark module provides infrastructure for evaluating Loom agents against industry-standard benchmarks like SWE-bench, WebArena, and GAIA. It enables:

- **Quantitative evaluation** of agent performance
- **Context engineering impact** measurement
- **Token savings** calculation
- **Cost/quality trade-offs** analysis

## Quick Start

### 1. Run a Benchmark

```bash
# Run SWE-bench with 10 tasks
loom benchmark run swe-bench \
    --dataset ./datasets/swe-bench-lite \
    --max-tasks 10 \
    --output results/baseline.json \
    --verbose

# Run with context engineering disabled (baseline)
loom benchmark run swe-bench \
    --dataset ./datasets/swe-bench-lite \
    --max-tasks 10 \
    --no-context-engineering \
    --output results/baseline.json
```

### 2. Run Comparison

```bash
# Compare baseline vs optimized
loom benchmark compare swe-bench \
    --dataset ./datasets/swe-bench-lite \
    --max-tasks 10 \
    --output results/comparison.json
```

### 3. Generate Reports

```bash
# Console report
loom benchmark report results/comparison.json

# Markdown report
loom benchmark report results/comparison.json \
    --format markdown \
    --output results/report.md
```

## Programmatic Usage

### Basic Benchmark Run

```python
from loom.benchmark import BenchmarkRunner
from loom.cognitive import CognitiveAgent, CognitiveConfig
from loom.llm import LLMProvider
from pathlib import Path

# Initialize LLM
llm = LLMProvider.from_name("deepseek")

# Create agent factory
def agent_factory(ctx, llm_provider):
    return CognitiveAgent(
        ctx=ctx,
        llm=llm_provider,
        config=CognitiveConfig(
            system_prompt="You are a software engineering assistant.",
            max_iterations=20,
        ),
        workspace_path=Path("./workspace"),
    )

# Create runner
runner = BenchmarkRunner(
    agent_factory=agent_factory,
    llm=llm,
    benchmark="swe-bench",
    dataset_path="./datasets/swe-bench-lite",
)

# Run benchmark
results = await runner.run(max_tasks=50, verbose=True)

# Print summary
print(f"Success rate: {results.success_rate:.1%}")
print(f"Avg tokens: {results.avg_tokens:.0f}")
print(f"Token savings: {results.avg_token_savings:.1f}%")

# Save results
results.save(Path("results/run.json"))
```

### Comparison Run

```python
# Run comparison
comparison = await runner.run_comparison(max_tasks=50)

# Print comparison
comparison.print_summary()

# Access metrics
print(f"Token reduction: {comparison.token_reduction():.1f}%")
print(f"Cost reduction: {comparison.cost_reduction():.1f}%")
print(f"Quality delta: {comparison.quality_delta():+.1f}%")

# Save comparison
comparison.save(Path("results/comparison.json"))
```

### Custom Metrics Collection

```python
from loom.benchmark import TaskMetrics

# Manual metrics
metrics = TaskMetrics(
    task_id="custom-task-1",
    success=True,
    iterations=8,
    total_tokens=2500,
    total_cost_usd=0.125,
    execution_time_sec=15.2,
    offloaded_files=5,
    compacted_steps=3,
    avg_prompt_tokens=300,
    peak_prompt_tokens=450,
    token_savings_pct=78.5,
    correct_answer=True,
)

# Add to results
results.tasks.append(metrics)
```

## Supported Benchmarks

### SWE-bench

**Status**: ✅ Implemented (MVP)

**Description**: Software engineering tasks from real GitHub issues

**Tasks**: 2,294 total (300 in Lite version)

**Metrics**:

- Task success rate
- Test pass rate
- Token usage
- Execution time

**Example**:

```bash
loom benchmark run swe-bench \
    --dataset ./swe-bench-lite \
    --max-tasks 50
```

### WebArena (Coming Soon)

**Status**: 📋 Planned

**Description**: Real-world web interaction tasks

**Tasks**: 812 realistic scenarios

**Metrics**:

- Task completion rate
- Navigation efficiency
- Token usage with web content

### GAIA (Coming Soon)

**Status**: 📋 Planned

**Description**: General AI assistant evaluation

**Tasks**: 466 diverse tasks

**Metrics**:

- Multi-domain performance
- Human evaluation scores

## Architecture

### Components

```
benchmark/
├── __init__.py          # Public API
├── runner.py            # BenchmarkRunner
├── metrics.py           # Metrics & results
├── adapters/            # Dataset adapters
│   ├── __init__.py
│   └── swe_bench.py     # SWE-bench adapter
└── README.md            # This file
```

### Data Flow

```
Dataset → Adapter → Runner → Agent → Metrics → Results
                       ↓
                  Task Prep
                  Evaluation
```

### Metrics Collected

```python
@dataclass
class TaskMetrics:
    # Basic
    task_id: str
    success: bool
    iterations: int
    execution_time_sec: float

    # Token usage
    total_tokens: int
    avg_prompt_tokens: int
    peak_prompt_tokens: int

    # Context engineering
    offloaded_files: int
    compacted_steps: int
    token_savings_pct: float

    # Cost
    total_cost_usd: float

    # Quality
    correct_answer: bool
    human_eval_score: Optional[float]
```

## Creating Custom Adapters

### Basic Adapter Template

```python
from loom.benchmark.adapters import BaseAdapter

class MyBenchmarkAdapter:
    """Adapter for custom benchmark."""

    def __init__(self, dataset_path: Path, workspace_path: Path):
        self.dataset_path = dataset_path
        self.workspace_path = workspace_path

    def load_tasks(self, max_tasks: int) -> list[dict]:
        """Load tasks from dataset."""
        # Load and normalize tasks
        return tasks

    async def prepare_task(self, task: dict):
        """Prepare execution environment."""
        # Setup workspace, files, etc.
        return context

    async def evaluate_result(self, task: dict, result) -> bool:
        """Evaluate if result is correct."""
        # Run tests, check correctness
        return is_correct
```

### SWE-bench Example

See `adapters/swe_bench.py` for a complete implementation:

```python
from loom.benchmark.adapters import SWEBenchAdapter

adapter = SWEBenchAdapter(
    dataset_path=Path("./swe-bench-lite"),
    workspace_path=Path("./workspaces")
)

# Load tasks
tasks = adapter.load_tasks(max_tasks=10)

# Prepare workspace
ctx = await adapter.prepare_task(tasks[0])

# Evaluate result
correct = await adapter.evaluate_result(tasks[0], result)
```

## Best Practices

### 1. Start Small

```bash
# Test with 5-10 tasks first
loom benchmark run swe-bench --dataset ./data --max-tasks 10
```

### 2. Use Comparison Mode

```bash
# Always compare baseline vs optimized
loom benchmark compare swe-bench --dataset ./data --max-tasks 50
```

### 3. Save Results

```bash
# Always save results for later analysis
loom benchmark run swe-bench \
    --dataset ./data \
    --output results/$(date +%Y%m%d)_run.json
```

### 4. Monitor Progress

```bash
# Use verbose mode to track progress
loom benchmark run swe-bench --dataset ./data -v
```

### 5. Analyze Failures

```python
# Load results and check failures
results = BenchmarkResults.load("results/run.json")

failures = [t for t in results.tasks if not t.success]
for task in failures:
    print(f"{task.task_id}: {task.error_message}")
```

## Expected Results

### Context Engineering Impact

Based on testing, context engineering should provide:

| Metric      | Baseline   | Optimized  | Improvement |
| ----------- | ---------- | ---------- | ----------- |
| Token Usage | 8000/task  | 1500/task  | **-81%**    |
| Cost        | $0.40/task | $0.08/task | **-80%**    |
| Quality     | 75%        | 75%        | **0%**      |
| Latency     | 45s        | 42s        | **-7%**     |

### Typical Run Times

- 10 tasks: ~5-10 minutes
- 50 tasks: ~30-60 minutes
- 300 tasks: ~4-8 hours

## Troubleshooting

### Dataset Not Found

```bash
# Download SWE-bench Lite
pip install datasets
python -c "from datasets import load_dataset; load_dataset('princeton-nlp/SWE-bench_Lite')"
```

### Agent Fails to Start

```bash
# Check bridge is running
loom up --mode bridge-only

# Verify connection
echo $LOOM_BRIDGE_ADDR
```

### Out of Memory

```bash
# Reduce max tasks
loom benchmark run swe-bench --max-tasks 5

# Or disable verbose mode
loom benchmark run swe-bench --max-tasks 50  # No -v
```

## See Also

- [BENCHMARKING.md](../../docs/BENCHMARKING.md) - Detailed benchmarking strategy
- [Context Engineering Docs](../context/engineering/README.md) - Context optimization techniques
- [Cognitive Agent Guide](../../docs/COGNITIVE_GUIDE.md) - Agent configuration

## Contributing

To add a new benchmark:

1. Create adapter in `adapters/new_benchmark.py`
2. Implement `load_tasks()`, `prepare_task()`, `evaluate_result()`
3. Add tests in `tests/unit/test_benchmark.py`
4. Update `runner.py` to support new benchmark
5. Document usage in this README

---

_Last updated: 2025-12-07_
