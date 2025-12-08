"""Benchmark infrastructure for evaluating agent performance.

This module provides tools for benchmarking Loom agents with industry-standard
benchmarks like GAIA, TAU-bench, and AgentBench. It includes:

- BenchmarkRunner: Execute benchmark tasks with/without context engineering
- TaskMetrics: Capture detailed performance metrics
- BenchmarkResults: Aggregate results and statistics
- ComparisonReport: Compare baseline vs optimized runs

Example:
    ```python
    from loom.benchmark import BenchmarkRunner
    from loom.cognitive import CognitiveAgent

    # Setup agent
    agent = create_agent()

    # Run benchmark
    runner = BenchmarkRunner(
        agent_path="apps/chat-assistant",
        benchmark="gaia",
        dataset_path="./datasets/gaia-validation"
    )

    results = await runner.run(max_tasks=50)
    print(f"Success rate: {results.success_rate:.1%}")
    print(f"Avg tokens: {results.avg_tokens:.0f}")
    ```
"""

from .metrics import BenchmarkResults, ComparisonReport, TaskMetrics
from .prompts import get_generic_prompt
from .runner import BenchmarkRunner

__all__ = [
    "BenchmarkRunner",
    "TaskMetrics",
    "BenchmarkResults",
    "ComparisonReport",
    "get_swe_bench_prompt",
    "get_generic_prompt",
]
