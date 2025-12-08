"""Benchmark metrics collection and aggregation.

This module defines the data structures for capturing and analyzing
benchmark performance metrics, including token usage, costs, and quality.
"""

from __future__ import annotations

import json
from dataclasses import dataclass, field
from pathlib import Path
from typing import Optional


@dataclass
class TaskMetrics:
    """Metrics for a single benchmark task execution.

    Attributes:
        task_id: Unique identifier for the task
        success: Whether the task completed successfully
        iterations: Number of cognitive loop iterations
        total_tokens: Total tokens used (all prompts + completions)
        total_cost_usd: Estimated cost in USD
        execution_time_sec: Wall-clock time for execution
        offloaded_files: Number of files created for data offloading
        compacted_steps: Number of steps that were compacted
        avg_prompt_tokens: Average tokens per prompt
        peak_prompt_tokens: Maximum tokens in a single prompt
        token_savings_pct: Estimated token savings vs no context engineering
        correct_answer: Whether the final answer was correct
        human_eval_score: Optional human evaluation score (0-1)
        error_message: Error message if task failed
    """

    task_id: str
    success: bool
    iterations: int
    total_tokens: int
    total_cost_usd: float
    execution_time_sec: float

    # Context engineering metrics
    offloaded_files: int = 0
    compacted_steps: int = 0
    avg_prompt_tokens: int = 0
    peak_prompt_tokens: int = 0
    token_savings_pct: float = 0.0

    # Quality metrics
    correct_answer: bool = False
    human_eval_score: Optional[float] = None
    error_message: Optional[str] = None

    def to_dict(self) -> dict:
        """Convert to dictionary for serialization."""
        return {
            "task_id": self.task_id,
            "success": self.success,
            "iterations": self.iterations,
            "total_tokens": self.total_tokens,
            "total_cost_usd": self.total_cost_usd,
            "execution_time_sec": self.execution_time_sec,
            "offloaded_files": self.offloaded_files,
            "compacted_steps": self.compacted_steps,
            "avg_prompt_tokens": self.avg_prompt_tokens,
            "peak_prompt_tokens": self.peak_prompt_tokens,
            "token_savings_pct": self.token_savings_pct,
            "correct_answer": self.correct_answer,
            "human_eval_score": self.human_eval_score,
            "error_message": self.error_message,
        }

    @classmethod
    def from_dict(cls, data: dict) -> "TaskMetrics":
        """Create from dictionary."""
        return cls(**data)


@dataclass
class BenchmarkResults:
    """Aggregate results from a benchmark run.

    Attributes:
        benchmark_name: Name of the benchmark (e.g., "swe-bench")
        tasks: List of individual task metrics
        context_engineering_enabled: Whether context engineering was used
        timestamp: ISO timestamp of the run
    """

    benchmark_name: str
    tasks: list[TaskMetrics] = field(default_factory=list)
    context_engineering_enabled: bool = True
    timestamp: str = ""

    @property
    def success_rate(self) -> float:
        """Percentage of tasks that completed successfully."""
        if not self.tasks:
            return 0.0
        return sum(t.success for t in self.tasks) / len(self.tasks)

    @property
    def correctness_rate(self) -> float:
        """Percentage of tasks with correct answers."""
        if not self.tasks:
            return 0.0
        return sum(t.correct_answer for t in self.tasks) / len(self.tasks)

    @property
    def avg_tokens(self) -> float:
        """Average tokens per task."""
        if not self.tasks:
            return 0.0
        return sum(t.total_tokens for t in self.tasks) / len(self.tasks)

    @property
    def avg_cost(self) -> float:
        """Average cost per task in USD."""
        if not self.tasks:
            return 0.0
        return sum(t.total_cost_usd for t in self.tasks) / len(self.tasks)

    @property
    def avg_time(self) -> float:
        """Average execution time per task in seconds."""
        if not self.tasks:
            return 0.0
        return sum(t.execution_time_sec for t in self.tasks) / len(self.tasks)

    @property
    def total_offloaded_files(self) -> int:
        """Total number of files offloaded across all tasks."""
        return sum(t.offloaded_files for t in self.tasks)

    @property
    def total_compacted_steps(self) -> int:
        """Total number of steps compacted across all tasks."""
        return sum(t.compacted_steps for t in self.tasks)

    @property
    def avg_token_savings(self) -> float:
        """Average token savings percentage."""
        if not self.tasks:
            return 0.0
        return sum(t.token_savings_pct for t in self.tasks) / len(self.tasks)

    def summary(self) -> dict:
        """Generate summary statistics."""
        return {
            "benchmark": self.benchmark_name,
            "context_engineering": self.context_engineering_enabled,
            "tasks_total": len(self.tasks),
            "tasks_succeeded": sum(t.success for t in self.tasks),
            "tasks_correct": sum(t.correct_answer for t in self.tasks),
            "success_rate": f"{self.success_rate:.1%}",
            "correctness_rate": f"{self.correctness_rate:.1%}",
            "avg_tokens": int(self.avg_tokens),
            "avg_cost_usd": f"${self.avg_cost:.4f}",
            "avg_time_sec": f"{self.avg_time:.1f}",
            "total_offloaded_files": self.total_offloaded_files,
            "total_compacted_steps": self.total_compacted_steps,
            "avg_token_savings": f"{self.avg_token_savings:.1f}%",
        }

    def save(self, path: Path) -> None:
        """Save results to JSON file."""
        data = {
            "benchmark_name": self.benchmark_name,
            "context_engineering_enabled": self.context_engineering_enabled,
            "timestamp": self.timestamp,
            "summary": self.summary(),
            "tasks": [t.to_dict() for t in self.tasks],
        }
        path.write_text(json.dumps(data, indent=2))

    @classmethod
    def load(cls, path: Path) -> "BenchmarkResults":
        """Load results from JSON file."""
        data = json.loads(path.read_text())
        return cls(
            benchmark_name=data["benchmark_name"],
            context_engineering_enabled=data["context_engineering_enabled"],
            timestamp=data["timestamp"],
            tasks=[TaskMetrics.from_dict(t) for t in data["tasks"]],
        )


@dataclass
class ComparisonReport:
    """Comparison between baseline and optimized benchmark runs.

    Attributes:
        baseline: Results without context engineering
        optimized: Results with context engineering
    """

    baseline: BenchmarkResults
    optimized: BenchmarkResults

    def token_reduction(self) -> float:
        """Calculate token reduction percentage."""
        if self.baseline.avg_tokens == 0:
            return 0.0
        return (
            (self.baseline.avg_tokens - self.optimized.avg_tokens) / self.baseline.avg_tokens
        ) * 100

    def cost_reduction(self) -> float:
        """Calculate cost reduction percentage."""
        if self.baseline.avg_cost == 0:
            return 0.0
        return ((self.baseline.avg_cost - self.optimized.avg_cost) / self.baseline.avg_cost) * 100

    def quality_delta(self) -> float:
        """Calculate quality change (correctness rate delta)."""
        return (self.optimized.correctness_rate - self.baseline.correctness_rate) * 100

    def summary(self) -> dict:
        """Generate comparison summary."""
        return {
            "benchmark": self.baseline.benchmark_name,
            "baseline": self.baseline.summary(),
            "optimized": self.optimized.summary(),
            "improvements": {
                "token_reduction": f"{self.token_reduction():.1f}%",
                "cost_reduction": f"{self.cost_reduction():.1f}%",
                "quality_delta": f"{self.quality_delta():+.1f}%",
                "time_delta": f"{(self.optimized.avg_time - self.baseline.avg_time):+.1f}s",
            },
        }

    def print_summary(self) -> None:
        """Print formatted comparison summary."""
        print("\n" + "=" * 80)
        print(f"Benchmark Comparison: {self.baseline.benchmark_name}")
        print("=" * 80)

        print("\n📊 BASELINE (No Context Engineering)")
        print("-" * 80)
        baseline_sum = self.baseline.summary()
        print(f"  Tasks: {baseline_sum['tasks_total']} total")
        print(f"  Success Rate: {baseline_sum['success_rate']}")
        print(f"  Correctness: {baseline_sum['correctness_rate']}")
        print(f"  Avg Tokens: {baseline_sum['avg_tokens']:,}")
        print(f"  Avg Cost: {baseline_sum['avg_cost_usd']}")
        print(f"  Avg Time: {baseline_sum['avg_time_sec']}")

        print("\n✨ OPTIMIZED (With Context Engineering)")
        print("-" * 80)
        opt_sum = self.optimized.summary()
        print(f"  Tasks: {opt_sum['tasks_total']} total")
        print(f"  Success Rate: {opt_sum['success_rate']}")
        print(f"  Correctness: {opt_sum['correctness_rate']}")
        print(f"  Avg Tokens: {opt_sum['avg_tokens']:,}")
        print(f"  Avg Cost: {opt_sum['avg_cost_usd']}")
        print(f"  Avg Time: {opt_sum['avg_time_sec']}")
        print(f"  Offloaded Files: {opt_sum['total_offloaded_files']}")
        print(f"  Compacted Steps: {opt_sum['total_compacted_steps']}")
        print(f"  Token Savings: {opt_sum['avg_token_savings']}")

        print("\n📈 IMPROVEMENTS")
        print("-" * 80)
        print(f"  Token Reduction: {self.token_reduction():.1f}%")
        print(f"  Cost Reduction: {self.cost_reduction():.1f}%")
        print(f"  Quality Delta: {self.quality_delta():+.1f}%")
        print(f"  Time Delta: {(self.optimized.avg_time - self.baseline.avg_time):+.1f}s")

        print("\n" + "=" * 80 + "\n")

    def save(self, path: Path) -> None:
        """Save comparison report to JSON file."""
        data = self.summary()
        path.write_text(json.dumps(data, indent=2))


__all__ = [
    "TaskMetrics",
    "BenchmarkResults",
    "ComparisonReport",
]
