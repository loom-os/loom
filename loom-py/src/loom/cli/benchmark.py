"""Benchmark CLI commands for Loom agents.

This module provides CLI commands for running benchmarks and generating reports.

Commands:
    loom benchmark run - Run a benchmark with optional context engineering
    loom benchmark compare - Run comparison between baseline and optimized
    loom benchmark report - Generate reports from benchmark results
"""

from __future__ import annotations

import asyncio
import sys
from pathlib import Path


def add_benchmark_subparser(subparsers):
    """Add benchmark subcommands to main CLI parser.

    Args:
        subparsers: Subparser from argparse
    """
    bench_parser = subparsers.add_parser(
        "benchmark",
        help="Run agent benchmarks and generate reports",
        description="Benchmark Loom agents against industry-standard datasets",
    )

    bench_subparsers = bench_parser.add_subparsers(dest="benchmark_cmd", help="Benchmark commands")

    # benchmark run
    run_parser = bench_subparsers.add_parser(
        "run",
        help="Run benchmark tasks",
        description="Execute benchmark tasks and collect metrics",
    )
    run_parser.add_argument(
        "benchmark",
        type=str,
        help="Benchmark to run (e.g., 'gaia', 'tau-bench', 'agentbench')",
    )
    run_parser.add_argument(
        "--agent-path",
        type=str,
        required=True,
        help="Path to agent project directory (with loom.toml)",
    )
    run_parser.add_argument(
        "--dataset",
        type=str,
        required=True,
        help="Path to benchmark dataset",
    )
    run_parser.add_argument(
        "--max-tasks",
        type=int,
        default=10,
        help="Maximum number of tasks to run (default: 10)",
    )
    run_parser.add_argument(
        "--no-context-engineering",
        action="store_true",
        help="Disable context engineering (baseline)",
    )
    run_parser.add_argument(
        "--output",
        type=str,
        help="Output path for results JSON",
    )
    run_parser.add_argument(
        "--workspace",
        type=str,
        help="Workspace path for task execution",
    )
    run_parser.add_argument(
        "-v",
        "--verbose",
        action="store_true",
        help="Print detailed progress",
    )

    # benchmark compare
    compare_parser = bench_subparsers.add_parser(
        "compare",
        help="Run comparison: baseline vs optimized",
        description="Run benchmark with and without context engineering",
    )
    compare_parser.add_argument(
        "benchmark",
        type=str,
        help="Benchmark to run (e.g., 'gaia', 'tau-bench', 'agentbench')",
    )
    compare_parser.add_argument(
        "--agent-path",
        type=str,
        required=True,
        help="Path to agent project directory (with loom.toml)",
    )
    compare_parser.add_argument(
        "--dataset",
        type=str,
        required=True,
        help="Path to benchmark dataset",
    )
    compare_parser.add_argument(
        "--max-tasks",
        type=int,
        default=10,
        help="Maximum number of tasks per run (default: 10)",
    )
    compare_parser.add_argument(
        "--output",
        type=str,
        help="Output path for comparison report JSON",
    )
    compare_parser.add_argument(
        "--workspace",
        type=str,
        help="Workspace path for task execution",
    )

    # benchmark report
    report_parser = bench_subparsers.add_parser(
        "report",
        help="Generate report from benchmark results",
        description="Create markdown/console report from saved results",
    )
    report_parser.add_argument(
        "input",
        type=str,
        help="Path to benchmark results JSON file",
    )
    report_parser.add_argument(
        "--format",
        choices=["console", "markdown"],
        default="console",
        help="Report format (default: console)",
    )
    report_parser.add_argument(
        "--output",
        type=str,
        help="Output file for markdown report",
    )

    bench_parser.set_defaults(func=cmd_benchmark)


def cmd_benchmark(args):
    """Handle benchmark commands."""
    if not hasattr(args, "benchmark_cmd") or args.benchmark_cmd is None:
        print("Error: No benchmark command specified. Use -h for help.")
        sys.exit(1)

    if args.benchmark_cmd == "run":
        asyncio.run(cmd_benchmark_run(args))
    elif args.benchmark_cmd == "compare":
        asyncio.run(cmd_benchmark_compare(args))
    elif args.benchmark_cmd == "report":
        cmd_benchmark_report(args)


async def cmd_benchmark_run(args):
    """Run benchmark tasks and collect metrics."""
    from ..benchmark import BenchmarkRunner

    print(f"🚀 Running {args.benchmark} benchmark")
    print(f"Dataset: {args.dataset}")
    print(f"Max tasks: {args.max_tasks}")

    if args.agent_path:
        print(f"Agent: {args.agent_path}")
    print(f"Context engineering: {'❌ Disabled' if args.no_context_engineering else '✅ Enabled'}")
    print()

    # Create runner with agent path
    runner = BenchmarkRunner(
        benchmark=args.benchmark,
        dataset_path=args.dataset,
        agent_path=args.agent_path,
        workspace_path=args.workspace,
    )

    # Run benchmark
    results = await runner.run(
        max_tasks=args.max_tasks,
        context_engineering=not args.no_context_engineering,
        verbose=args.verbose or True,
    )

    # Save results
    if args.output:
        output_path = Path(args.output)
        output_path.parent.mkdir(parents=True, exist_ok=True)
        results.save(output_path)
        print(f"\n💾 Results saved to: {output_path}")
    else:
        print("\n📊 Results summary:")
        summary = results.summary()
        for key, value in summary.items():
            print(f"  {key}: {value}")


async def cmd_benchmark_compare(args):
    """Run comparison between baseline and optimized."""
    from ..benchmark import BenchmarkRunner

    print(f"📊 Running comparison for {args.benchmark}")
    print(f"Dataset: {args.dataset}")
    print(f"Max tasks per run: {args.max_tasks}")
    if args.agent_path:
        print(f"Agent: {args.agent_path}")
    print()

    # Create runner with agent path
    runner = BenchmarkRunner(
        benchmark=args.benchmark,
        dataset_path=args.dataset,
        agent_path=args.agent_path,
        workspace_path=args.workspace,
    )

    # Run comparison
    comparison = await runner.run_comparison(
        max_tasks=args.max_tasks,
        verbose=True,
    )

    # Save results
    if args.output:
        output_path = Path(args.output)
        output_path.parent.mkdir(parents=True, exist_ok=True)
        comparison.save(output_path)
        print(f"\n💾 Comparison report saved to: {output_path}")


def cmd_benchmark_report(args):
    """Generate report from saved benchmark results."""
    from ..benchmark import BenchmarkResults, ComparisonReport

    input_path = Path(args.input)

    if not input_path.exists():
        print(f"❌ Error: Results file not found: {input_path}")
        sys.exit(1)

    print(f"📖 Generating report from: {input_path}")

    # Load results
    import json

    data = json.loads(input_path.read_text())

    # Check if it's a comparison or single run
    if "baseline" in data and "optimized" in data:
        # It's a comparison report
        baseline = BenchmarkResults(
            benchmark_name=data["baseline"]["benchmark"],
            context_engineering_enabled=data["baseline"]["context_engineering"],
            timestamp=data["baseline"].get("timestamp", ""),
        )
        baseline.tasks = [_load_task_metrics(t) for t in data["baseline"]["tasks"]]

        optimized = BenchmarkResults(
            benchmark_name=data["optimized"]["benchmark"],
            context_engineering_enabled=data["optimized"]["context_engineering"],
            timestamp=data["optimized"].get("timestamp", ""),
        )
        optimized.tasks = [_load_task_metrics(t) for t in data["optimized"]["tasks"]]

        comparison = ComparisonReport(baseline=baseline, optimized=optimized)

        if args.format == "console":
            comparison.print_summary()
        elif args.format == "markdown":
            md_report = _generate_markdown_report(comparison)
            if args.output:
                Path(args.output).write_text(md_report)
                print(f"✅ Markdown report saved to: {args.output}")
            else:
                print(md_report)
    else:
        # Single run results
        results = BenchmarkResults.load(input_path)

        if args.format == "console":
            print("\n" + "=" * 80)
            print(f"Benchmark Results: {results.benchmark_name}")
            print("=" * 80)
            summary = results.summary()
            for key, value in summary.items():
                print(f"  {key}: {value}")
            print("=" * 80 + "\n")
        elif args.format == "markdown":
            md_report = _generate_markdown_single(results)
            if args.output:
                Path(args.output).write_text(md_report)
                print(f"✅ Markdown report saved to: {args.output}")
            else:
                print(md_report)


def _load_task_metrics(data: dict):
    """Helper to load TaskMetrics from dict."""
    from ..benchmark import TaskMetrics

    return TaskMetrics(**data)


def _generate_markdown_report(comparison) -> str:
    """Generate markdown report for comparison."""
    from ..benchmark import ComparisonReport

    comparison: ComparisonReport  # Type hint for IDE
    baseline = comparison.baseline
    optimized = comparison.optimized

    md = f"""# Benchmark Comparison: {baseline.benchmark_name}

## Overview

- **Date**: {optimized.timestamp}
- **Tasks**: {len(baseline.tasks)} tasks per run
- **LLM**: Context engineering comparison

## Results

### Baseline (No Context Engineering)

| Metric | Value |
|--------|-------|
| Success Rate | {baseline.success_rate:.1%} |
| Correctness | {baseline.correctness_rate:.1%} |
| Avg Tokens | {baseline.avg_tokens:,.0f} |
| Avg Cost | ${baseline.avg_cost:.4f} |
| Avg Time | {baseline.avg_time:.1f}s |

### Optimized (With Context Engineering)

| Metric | Value |
|--------|-------|
| Success Rate | {optimized.success_rate:.1%} |
| Correctness | {optimized.correctness_rate:.1%} |
| Avg Tokens | {optimized.avg_tokens:,.0f} |
| Avg Cost | ${optimized.avg_cost:.4f} |
| Avg Time | {optimized.avg_time:.1f}s |
| Offloaded Files | {optimized.total_offloaded_files} |
| Compacted Steps | {optimized.total_compacted_steps} |
| Token Savings | {optimized.avg_token_savings:.1f}% |

## Improvements

| Metric | Delta |
|--------|-------|
| Token Reduction | {comparison.token_reduction():.1f}% |
| Cost Reduction | {comparison.cost_reduction():.1f}% |
| Quality Delta | {comparison.quality_delta():+.1f}% |
| Time Delta | {(optimized.avg_time - baseline.avg_time):+.1f}s |

## Conclusion

Context engineering achieved **{comparison.token_reduction():.1f}% token reduction** with **{comparison.quality_delta():+.1f}%** quality change.
"""

    return md


def _generate_markdown_single(results) -> str:
    """Generate markdown report for single run."""
    from ..benchmark import BenchmarkResults

    results: BenchmarkResults  # Type hint for IDE
    summary = results.summary()

    md = f"""# Benchmark Results: {results.benchmark_name}

## Overview

- **Date**: {results.timestamp}
- **Context Engineering**: {'✅ Enabled' if results.context_engineering_enabled else '❌ Disabled'}
- **Tasks**: {len(results.tasks)}

## Metrics

| Metric | Value |
|--------|-------|
| Success Rate | {summary['success_rate']} |
| Correctness | {summary['correctness_rate']} |
| Avg Tokens | {summary['avg_tokens']:,} |
| Avg Cost | {summary['avg_cost_usd']} |
| Avg Time | {summary['avg_time_sec']} |
"""

    if results.context_engineering_enabled:
        md += f"""| Offloaded Files | {summary['total_offloaded_files']} |
| Compacted Steps | {summary['total_compacted_steps']} |
| Token Savings | {summary['avg_token_savings']} |
"""

    return md


__all__ = ["add_benchmark_subparser", "cmd_benchmark"]
