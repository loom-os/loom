"""Benchmark infrastructure for evaluating agent performance.

This module provides tools for benchmarking Loom agents with industry-standard
benchmarks like SWE-bench, WebArena, and GAIA. It includes:

- BenchmarkRunner: Execute benchmark tasks with/without context engineering
- TaskMetrics: Capture detailed performance metrics
- BenchmarkResults: Aggregate results and statistics
- ComparisonReport: Compare baseline vs optimized runs

Example:
    ```python
    from loom.benchmark import BenchmarkRunner, SWEBenchAdapter
    from loom.cognitive import CognitiveAgent

    # Setup agent
    agent = create_agent()

    # Run benchmark
    runner = BenchmarkRunner(
        agent=agent,
        benchmark="swe-bench",
        dataset_path="./datasets/swe-bench-lite"
    )

    results = await runner.run(max_tasks=50)
    print(f"Success rate: {results.success_rate:.1%}")
    print(f"Avg tokens: {results.avg_tokens:.0f}")
    ```
"""

from .metrics import BenchmarkResults, ComparisonReport, TaskMetrics
from .runner import BenchmarkRunner

__all__ = [
    "BenchmarkRunner",
    "TaskMetrics",
    "BenchmarkResults",
    "ComparisonReport",
]
