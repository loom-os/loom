# Agent Benchmarking Strategy

> **Measuring Context Engineering Impact with Industry-Standard Benchmarks**

## Current Status

### What We Have ✅

- **Reduction**: Per-step token optimization (29.4% savings)
- **Compaction**: Long conversation compression (60-85% savings)
- **Offloading**: Data externalization (up to 99% for large outputs)
- **Total**: Up to 90% token reduction

### What We Need 📊

- **Quantitative validation** against standard tasks
- **Comparative analysis** (with vs without context engineering)
- **Real-world task performance** (not just synthetic tests)
- **Cost/quality trade-offs** measurement

---

## Industry-Standard Agent Benchmarks

### 1. **WebArena** ⭐ Recommended

**What it tests:**

- Real-world web interaction tasks (e-commerce, forums, Wikipedia, etc.)
- 812 realistic tasks across multiple domains
- Requires multi-step reasoning and tool use

**Why it's good for us:**

- ✅ Long conversations (5-20+ steps typical)
- ✅ Heavy context (web pages, search results)
- ✅ Perfect for testing compaction + offloading
- ✅ Industry-recognized (CMU + others)

**Example tasks:**

```
Task 1: "Find the cheapest laptop under $500 with at least 8GB RAM"
→ 12 steps: search, filter, compare, verify
→ Without context eng: 8000+ tokens
→ With context eng: ~1500 tokens

Task 2: "Post a question on the forum about Python best practices"
→ 8 steps: navigate, login, compose, submit
→ Tests offloading (HTML pages), compaction (navigation history)
```

**Integration effort:** Medium (need browser automation via MCP or native tool)

**Paper:** https://arxiv.org/abs/2307.13854
**Code:** https://github.com/web-arena-x/webarena

---

### 2. **SWE-bench** ⭐ Highly Relevant

**What it tests:**

- Real GitHub issues from popular Python projects
- 2,294 tasks requiring code understanding and editing
- Multi-file reasoning

**Why it's good for us:**

- ✅ Very long contexts (multiple files, large codebases)
- ✅ Tests all three: reduction, compaction, offloading
- ✅ Industry gold standard for coding agents
- ✅ Clear success metric (unit tests pass/fail)

**Example tasks:**

```
Task: "Fix bug in scikit-learn #12345: ValueError in cross_validate"
→ 15-30 steps: read files, search code, analyze, edit, test
→ Without context eng: 15000+ tokens (multiple file contents)
→ With context eng: ~2000 tokens (offloaded files, compacted history)
```

**Integration effort:** Low (we already have fs:read, fs:write, shell:run)

**Paper:** https://arxiv.org/abs/2310.06770
**Code:** https://github.com/princeton-nlp/SWE-bench

---

### 3. **GAIA** (General AI Assistant)

**What it tests:**

- Real-world assistant tasks (research, calculations, multi-step reasoning)
- 466 tasks with ground-truth answers
- Requires web search, file operations, tool use

**Why it's good for us:**

- ✅ Diverse task types
- ✅ Tests context management across modalities
- ✅ Human-evaluated difficulty levels (1-3)
- ✅ Public leaderboard

**Example tasks:**

```
Level 1: "What is the population of the capital of France?"
→ 2-3 steps, tests basic tool use

Level 3: "Analyze the financial reports from Q1-Q4 2023 and predict Q1 2024 revenue"
→ 20+ steps, tests everything
```

**Integration effort:** Medium (need web search + file parsing)

**Paper:** https://arxiv.org/abs/2311.12983
**Code:** https://huggingface.co/datasets/gaia-benchmark/GAIA

---

### 4. **AgentBench**

**What it tests:**

- 8 distinct environments (code, game, web, OS, etc.)
- 3,691 tasks total
- Multi-domain evaluation

**Why it's good for us:**

- ✅ Comprehensive coverage
- ✅ Tests different context patterns per domain
- ✅ Widely cited (Tsinghua University)

**Cons:**

- ❌ Very broad (might be overkill)
- ❌ Some domains require special setup

**Integration effort:** High (8 different environments)

**Paper:** https://arxiv.org/abs/2308.03688
**Code:** https://github.com/THUDM/AgentBench

---

## Recommended Approach

### Phase 1: Quick Validation (Week 1)

**Use:** **SWE-bench Lite** (300 verified tasks subset)

**Why start here:**

1. ✅ We already have all needed tools (fs:read/write, shell:run)
2. ✅ Clear success metric (tests pass = 100%, fail = 0%)
3. ✅ Fast iteration (no browser setup needed)
4. ✅ Tests all context engineering components

**Setup:**

```bash
# Install SWE-bench
pip install swe-bench

# Get dataset
from datasets import load_dataset
dataset = load_dataset("princeton-nlp/SWE-bench_Lite")

# 300 tasks from: django, scikit-learn, requests, matplotlib, etc.
```

**Metrics to track:**

```python
{
    "task_id": "django__django-12345",
    "success": True,
    "iterations": 18,
    "total_tokens": 2500,  # WITH context eng
    "baseline_tokens": 12000,  # WITHOUT (estimated)
    "token_savings": 79.2,
    "cost_usd": 0.125,
    "baseline_cost_usd": 0.600,
    "cost_savings": 79.2,
    "task_time_sec": 45,
}
```

---

### Phase 2: Real-World Validation (Week 2-3)

**Use:** **WebArena** (100 task subset)

**Why next:**

1. ✅ Tests long conversations (typical agent use case)
2. ✅ Heavy web content (tests offloading at scale)
3. ✅ Multi-domain (e-commerce, forums, knowledge bases)
4. ✅ Realistic user scenarios

**Setup:**

```bash
# Clone WebArena
git clone https://github.com/web-arena-x/webarena
cd webarena

# Start local test sites
docker-compose up -d

# Configure Loom agent
# Need: browser:navigate, web:search, form:fill tools
```

**Metrics to track:**

```python
{
    "task_id": "webarena_123",
    "domain": "shopping",
    "success": True,
    "steps": 12,
    "offloaded_files": 8,  # HTML pages saved
    "compacted_steps": 5,   # Old steps compressed
    "avg_prompt_tokens": 1800,
    "peak_prompt_tokens": 2500,
    "without_ce_estimated": 9000,
}
```

---

### Phase 3: Comprehensive Evaluation (Week 4+)

**Use:** **GAIA** (all 466 tasks)

**Why last:**

1. ✅ Most diverse benchmark
2. ✅ Human-evaluated (quality validation)
3. ✅ Public leaderboard (compare vs others)

---

## Implementation Plan

### 1. Add Benchmark Infrastructure

```python
# loom-py/src/loom/benchmark/__init__.py
from .runner import BenchmarkRunner
from .metrics import BenchmarkMetrics
from .reporters import ConsoleReporter, JSONReporter

# loom-py/src/loom/benchmark/runner.py
class BenchmarkRunner:
    """Run agent benchmarks with/without context engineering."""

    def __init__(
        self,
        agent: CognitiveAgent,
        benchmark: str,  # "swe-bench", "webarena", "gaia"
        dataset_path: str,
    ):
        self.agent = agent
        self.benchmark = benchmark
        self.dataset = load_benchmark(benchmark, dataset_path)

    async def run(
        self,
        task_ids: Optional[list[str]] = None,
        with_context_engineering: bool = True,
        max_tasks: int = 100,
    ) -> BenchmarkResults:
        """Run benchmark tasks."""

        results = []
        for task in self.dataset.iter_tasks(task_ids, max_tasks):
            # Configure agent
            self.agent.config.use_reduction = with_context_engineering
            self.agent.config.use_compaction = with_context_engineering

            # Run task
            result = await self._run_task(task)
            results.append(result)

            # Log progress
            self._log_progress(results)

        return BenchmarkResults(results)

    async def run_comparison(self, max_tasks: int = 50):
        """Run with and without context engineering."""

        print("Running WITHOUT context engineering...")
        baseline = await self.run(
            with_context_engineering=False,
            max_tasks=max_tasks,
        )

        print("\nRunning WITH context engineering...")
        optimized = await self.run(
            with_context_engineering=True,
            max_tasks=max_tasks,
        )

        return ComparisonReport(baseline, optimized)
```

### 2. Task-Specific Adapters

```python
# loom-py/src/loom/benchmark/adapters/swe_bench.py
class SWEBenchAdapter:
    """Adapt SWE-bench tasks to Loom agent format."""

    def __init__(self, workspace: Path):
        self.workspace = workspace

    def prepare_task(self, task: dict) -> dict:
        """Prepare SWE-bench task for agent."""
        return {
            "goal": f"Fix issue: {task['problem_statement']}",
            "context": [
                f"Repository: {task['repo']}",
                f"Base commit: {task['base_commit']}",
                f"Files to check: {', '.join(task['hints'])}",
            ],
            "success_criteria": lambda: self._run_tests(task),
        }

    def _run_tests(self, task: dict) -> bool:
        """Run task-specific tests."""
        # Apply patch, run tests, check if pass
        ...
```

### 3. Metrics Collection

```python
# loom-py/src/loom/benchmark/metrics.py
@dataclass
class TaskMetrics:
    """Metrics for a single task."""
    task_id: str
    success: bool
    iterations: int
    total_tokens: int
    total_cost_usd: float
    execution_time_sec: float

    # Context engineering specific
    offloaded_files: int
    compacted_steps: int
    avg_prompt_tokens: int
    peak_prompt_tokens: int
    token_savings_pct: float

    # Quality metrics
    correct_answer: bool
    human_eval_score: Optional[float] = None

@dataclass
class BenchmarkResults:
    """Aggregate results."""
    tasks: list[TaskMetrics]

    @property
    def success_rate(self) -> float:
        return sum(t.success for t in self.tasks) / len(self.tasks)

    @property
    def avg_tokens(self) -> float:
        return sum(t.total_tokens for t in self.tasks) / len(self.tasks)

    @property
    def avg_cost(self) -> float:
        return sum(t.total_cost_usd for t in self.tasks) / len(self.tasks)

    def summary(self) -> dict:
        return {
            "total_tasks": len(self.tasks),
            "success_rate": f"{self.success_rate:.1%}",
            "avg_tokens": int(self.avg_tokens),
            "avg_cost": f"${self.avg_cost:.3f}",
            "total_cost": f"${sum(t.total_cost_usd for t in self.tasks):.2f}",
        }
```

### 4. Comparison Reports

```python
# loom-py/src/loom/benchmark/reporters.py
class ComparisonReport:
    """Compare baseline vs optimized runs."""

    def __init__(self, baseline: BenchmarkResults, optimized: BenchmarkResults):
        self.baseline = baseline
        self.optimized = optimized

    def print_summary(self):
        """Print comparison table."""

        print("\n" + "="*60)
        print("BENCHMARK COMPARISON REPORT")
        print("="*60)

        print(f"\n{'Metric':<30} {'Baseline':<15} {'Optimized':<15} {'Δ':<10}")
        print("-"*70)

        metrics = [
            ("Success Rate",
             f"{self.baseline.success_rate:.1%}",
             f"{self.optimized.success_rate:.1%}",
             self._delta_pct(self.baseline.success_rate, self.optimized.success_rate)),

            ("Avg Tokens per Task",
             f"{self.baseline.avg_tokens:.0f}",
             f"{self.optimized.avg_tokens:.0f}",
             self._delta_pct(self.baseline.avg_tokens, self.optimized.avg_tokens)),

            ("Avg Cost per Task",
             f"${self.baseline.avg_cost:.3f}",
             f"${self.optimized.avg_cost:.3f}",
             self._delta_pct(self.baseline.avg_cost, self.optimized.avg_cost)),

            ("Total Cost",
             f"${sum(t.total_cost_usd for t in self.baseline.tasks):.2f}",
             f"${sum(t.total_cost_usd for t in self.optimized.tasks):.2f}",
             None),
        ]

        for name, base, opt, delta in metrics:
            delta_str = f"{delta:+.1%}" if delta else ""
            print(f"{name:<30} {base:<15} {opt:<15} {delta_str:<10}")

        print("\n" + "="*60)

        # Token savings breakdown
        print("\nToken Savings Breakdown:")
        print(f"  Reduction:  ~30% (per-step optimization)")
        print(f"  Compaction: ~70% (history compression)")
        print(f"  Offloading: ~90% (extreme cases)")
        print(f"  Combined:   {self._token_savings():.1%} (measured)")

        print("\n" + "="*60)
```

---

## Expected Results

### Hypothesis

| Metric           | Baseline   | With Context Eng | Expected Δ                        |
| ---------------- | ---------- | ---------------- | --------------------------------- |
| **Token Usage**  | 8000/task  | 1500/task        | **-81%**                          |
| **Cost**         | $0.40/task | $0.08/task       | **-80%**                          |
| **Success Rate** | 75%        | 75%              | **0%** (no degradation)           |
| **Latency**      | 45s        | 42s              | **-7%** (fewer tokens to process) |

### Key Questions to Answer

1. **Does context engineering hurt quality?**

   - Measure: success rate, correctness
   - Expected: No (or minimal <2%) degradation

2. **How much do we actually save?**

   - Measure: tokens, cost
   - Expected: 70-85% (matches synthetic tests)

3. **Where does it help most?**

   - Measure: by task length (5, 10, 20+ steps)
   - Expected: More savings on longer tasks

4. **What breaks?**
   - Measure: failure modes
   - Expected: Edge cases where compaction loses critical info

---

## Integration into ROADMAP

### New Section: Phase 2.5 - Benchmarking (Week 2-3)

```markdown
**P0.5: Benchmark Validation** 🚧 In Progress

| Task  | Description                            | Status |
| ----- | -------------------------------------- | ------ |
| 2.8.1 | SWE-bench Lite integration (300 tasks) | 📋     |
| 2.8.2 | Baseline run (no context eng)          | 📋     |
| 2.8.3 | Optimized run (with context eng)       | 📋     |
| 2.8.4 | Comparison report & analysis           | 📋     |
| 2.8.5 | WebArena integration (100 tasks)       | 📋     |
| 2.8.6 | GAIA integration (full dataset)        | 📋     |
| 2.8.7 | Public benchmark results               | 📋     |
```

---

## Quick Start Script

```bash
# Install benchmark dependencies
cd loom-py
pip install swe-bench datasets

# Run quick benchmark (10 tasks)
python -m loom.benchmark.cli \
    --benchmark swe-bench \
    --tasks 10 \
    --compare \
    --output results/swe_bench_comparison.json

# Generate report
python -m loom.benchmark.report \
    --input results/swe_bench_comparison.json \
    --format markdown \
    --output results/report.md
<!-- ``` -->

---

## Success Criteria

### Must Have

- ✅ At least 1 benchmark integrated (SWE-bench)
- ✅ Comparison report showing token savings
- ✅ No significant quality degradation (<2%)

### Nice to Have

- ✨ Multiple benchmarks (SWE-bench + WebArena)
- ✨ Public results page
- ✨ Continuous benchmark CI/CD

### Stretch Goals

- 🎯 Top 10 on public leaderboard
- 🎯 Blog post with detailed analysis
- 🎯 Paper submission

---

## See Also

- [DESIGN.md](context/DESIGN.md) - Context engineering architecture
- [REDUCTION.md](context/REDUCTION.md) - Per-step optimization
- [COMPACTION.md](context/COMPACTION.md) - History compression
- Benchmarks: [SWE-bench](https://github.com/princeton-nlp/SWE-bench), [WebArena](https://github.com/web-arena-x/webarena), [GAIA](https://huggingface.co/datasets/gaia-benchmark/GAIA)

---

_Created: 2025-12-06_
