"""Unit tests for benchmark infrastructure."""

from __future__ import annotations

from pathlib import Path

import pytest

from loom.benchmark import BenchmarkResults, ComparisonReport, TaskMetrics


class TestTaskMetrics:
    """Test TaskMetrics data class."""

    def test_create_task_metrics(self):
        """Test creating task metrics."""
        metrics = TaskMetrics(
            task_id="test-1",
            success=True,
            iterations=5,
            total_tokens=1000,
            total_cost_usd=0.05,
            execution_time_sec=10.5,
            offloaded_files=3,
            compacted_steps=2,
            avg_prompt_tokens=200,
            peak_prompt_tokens=250,
            token_savings_pct=75.0,
            correct_answer=True,
        )

        assert metrics.task_id == "test-1"
        assert metrics.success is True
        assert metrics.total_tokens == 1000
        assert metrics.token_savings_pct == 75.0

    def test_to_dict(self):
        """Test converting metrics to dictionary."""
        metrics = TaskMetrics(
            task_id="test-1",
            success=True,
            iterations=5,
            total_tokens=1000,
            total_cost_usd=0.05,
            execution_time_sec=10.5,
        )

        data = metrics.to_dict()
        assert data["task_id"] == "test-1"
        assert data["success"] is True
        assert data["total_tokens"] == 1000

    def test_from_dict(self):
        """Test creating metrics from dictionary."""
        data = {
            "task_id": "test-1",
            "success": True,
            "iterations": 5,
            "total_tokens": 1000,
            "total_cost_usd": 0.05,
            "execution_time_sec": 10.5,
            "offloaded_files": 0,
            "compacted_steps": 0,
            "avg_prompt_tokens": 0,
            "peak_prompt_tokens": 0,
            "token_savings_pct": 0.0,
            "correct_answer": False,
            "human_eval_score": None,
            "error_message": None,
        }

        metrics = TaskMetrics.from_dict(data)
        assert metrics.task_id == "test-1"
        assert metrics.success is True


class TestBenchmarkResults:
    """Test BenchmarkResults aggregation."""

    def test_empty_results(self):
        """Test empty results."""
        results = BenchmarkResults(benchmark_name="test", tasks=[])

        assert results.success_rate == 0.0
        assert results.avg_tokens == 0.0
        assert results.avg_cost == 0.0

    def test_results_with_tasks(self):
        """Test results with multiple tasks."""
        tasks = [
            TaskMetrics(
                task_id=f"task-{i}",
                success=True,
                iterations=5,
                total_tokens=1000 + i * 100,
                total_cost_usd=0.05,
                execution_time_sec=10.0,
                correct_answer=True,
            )
            for i in range(5)
        ]

        results = BenchmarkResults(benchmark_name="test", tasks=tasks)

        assert results.success_rate == 1.0
        assert results.correctness_rate == 1.0
        assert results.avg_tokens == 1200.0  # (1000+1100+1200+1300+1400)/5
        assert len(results.tasks) == 5

    def test_mixed_success(self):
        """Test with mixed success/failure."""
        tasks = [
            TaskMetrics(
                task_id="task-1",
                success=True,
                iterations=5,
                total_tokens=1000,
                total_cost_usd=0.05,
                execution_time_sec=10.0,
                correct_answer=True,
            ),
            TaskMetrics(
                task_id="task-2",
                success=False,
                iterations=2,
                total_tokens=500,
                total_cost_usd=0.025,
                execution_time_sec=5.0,
                correct_answer=False,
            ),
        ]

        results = BenchmarkResults(benchmark_name="test", tasks=tasks)

        assert results.success_rate == 0.5
        assert results.correctness_rate == 0.5

    def test_summary(self):
        """Test summary generation."""
        tasks = [
            TaskMetrics(
                task_id="task-1",
                success=True,
                iterations=5,
                total_tokens=1000,
                total_cost_usd=0.05,
                execution_time_sec=10.0,
                offloaded_files=3,
                compacted_steps=2,
                token_savings_pct=70.0,
                correct_answer=True,
            )
        ]

        results = BenchmarkResults(
            benchmark_name="test",
            tasks=tasks,
            context_engineering_enabled=True,
        )

        summary = results.summary()
        assert summary["benchmark"] == "test"
        assert summary["tasks_total"] == 1
        assert summary["tasks_succeeded"] == 1
        assert summary["success_rate"] == "100.0%"

    def test_save_and_load(self, tmp_path: Path):
        """Test saving and loading results."""
        tasks = [
            TaskMetrics(
                task_id="task-1",
                success=True,
                iterations=5,
                total_tokens=1000,
                total_cost_usd=0.05,
                execution_time_sec=10.0,
                correct_answer=True,
            )
        ]

        results = BenchmarkResults(
            benchmark_name="test",
            tasks=tasks,
            timestamp="2025-01-01T00:00:00",
        )

        # Save
        output_path = tmp_path / "results.json"
        results.save(output_path)

        assert output_path.exists()

        # Load
        loaded = BenchmarkResults.load(output_path)
        assert loaded.benchmark_name == "test"
        assert len(loaded.tasks) == 1
        assert loaded.tasks[0].task_id == "task-1"


class TestComparisonReport:
    """Test comparison report generation."""

    def test_token_reduction(self):
        """Test token reduction calculation."""
        baseline_tasks = [
            TaskMetrics(
                task_id="task-1",
                success=True,
                iterations=5,
                total_tokens=10000,
                total_cost_usd=0.5,
                execution_time_sec=20.0,
                correct_answer=True,
            )
        ]

        optimized_tasks = [
            TaskMetrics(
                task_id="task-1",
                success=True,
                iterations=5,
                total_tokens=2000,
                total_cost_usd=0.1,
                execution_time_sec=18.0,
                correct_answer=True,
                offloaded_files=5,
                compacted_steps=3,
                token_savings_pct=80.0,
            )
        ]

        baseline = BenchmarkResults(
            benchmark_name="test",
            tasks=baseline_tasks,
            context_engineering_enabled=False,
        )

        optimized = BenchmarkResults(
            benchmark_name="test",
            tasks=optimized_tasks,
            context_engineering_enabled=True,
        )

        report = ComparisonReport(baseline=baseline, optimized=optimized)

        assert report.token_reduction() == 80.0
        assert report.cost_reduction() == 80.0
        assert report.quality_delta() == 0.0  # Both 100% correct

    def test_quality_delta(self):
        """Test quality change calculation."""
        baseline_tasks = [
            TaskMetrics(
                task_id="task-1",
                success=True,
                iterations=5,
                total_tokens=10000,
                total_cost_usd=0.5,
                execution_time_sec=20.0,
                correct_answer=True,
            ),
            TaskMetrics(
                task_id="task-2",
                success=True,
                iterations=5,
                total_tokens=10000,
                total_cost_usd=0.5,
                execution_time_sec=20.0,
                correct_answer=False,
            ),
        ]

        optimized_tasks = [
            TaskMetrics(
                task_id="task-1",
                success=True,
                iterations=5,
                total_tokens=2000,
                total_cost_usd=0.1,
                execution_time_sec=18.0,
                correct_answer=True,
            ),
            TaskMetrics(
                task_id="task-2",
                success=True,
                iterations=5,
                total_tokens=2000,
                total_cost_usd=0.1,
                execution_time_sec=18.0,
                correct_answer=True,
            ),
        ]

        baseline = BenchmarkResults(
            benchmark_name="test",
            tasks=baseline_tasks,
            context_engineering_enabled=False,
        )

        optimized = BenchmarkResults(
            benchmark_name="test",
            tasks=optimized_tasks,
            context_engineering_enabled=True,
        )

        report = ComparisonReport(baseline=baseline, optimized=optimized)

        # Baseline: 50% correct, Optimized: 100% correct -> +50%
        assert report.quality_delta() == pytest.approx(50.0, abs=0.1)

    def test_summary(self):
        """Test comparison summary."""
        baseline_tasks = [
            TaskMetrics(
                task_id="task-1",
                success=True,
                iterations=5,
                total_tokens=5000,
                total_cost_usd=0.25,
                execution_time_sec=15.0,
                correct_answer=True,
            )
        ]

        optimized_tasks = [
            TaskMetrics(
                task_id="task-1",
                success=True,
                iterations=5,
                total_tokens=1000,
                total_cost_usd=0.05,
                execution_time_sec=12.0,
                correct_answer=True,
                offloaded_files=2,
                compacted_steps=1,
                token_savings_pct=80.0,
            )
        ]

        baseline = BenchmarkResults(benchmark_name="test", tasks=baseline_tasks)
        optimized = BenchmarkResults(benchmark_name="test", tasks=optimized_tasks)

        report = ComparisonReport(baseline=baseline, optimized=optimized)

        summary = report.summary()
        assert "baseline" in summary
        assert "optimized" in summary
        assert "improvements" in summary
        assert summary["improvements"]["token_reduction"] == "80.0%"

    def test_print_summary(self, capsys):
        """Test printing comparison summary."""
        baseline_tasks = [
            TaskMetrics(
                task_id="task-1",
                success=True,
                iterations=5,
                total_tokens=5000,
                total_cost_usd=0.25,
                execution_time_sec=15.0,
                correct_answer=True,
            )
        ]

        optimized_tasks = [
            TaskMetrics(
                task_id="task-1",
                success=True,
                iterations=5,
                total_tokens=1000,
                total_cost_usd=0.05,
                execution_time_sec=12.0,
                correct_answer=True,
            )
        ]

        baseline = BenchmarkResults(benchmark_name="test", tasks=baseline_tasks)
        optimized = BenchmarkResults(benchmark_name="test", tasks=optimized_tasks)

        report = ComparisonReport(baseline=baseline, optimized=optimized)
        report.print_summary()

        captured = capsys.readouterr()
        assert "Benchmark Comparison" in captured.out
        assert "BASELINE" in captured.out
        assert "OPTIMIZED" in captured.out
        assert "IMPROVEMENTS" in captured.out
