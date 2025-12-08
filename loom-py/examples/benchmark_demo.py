"""Example: Benchmark context engineering impact on agent performance.

This example demonstrates how to:
1. Run a benchmark with context engineering enabled
2. Run a baseline without context engineering
3. Compare the results
4. Generate a report

Usage:
    python examples/benchmark_demo.py
"""

import asyncio
from pathlib import Path

from loom.benchmark import BenchmarkRunner
from loom.cognitive import CognitiveAgent, CognitiveConfig


async def main():
    """Run benchmark demo."""
    print("=" * 80)
    print("Loom Benchmark Demo: Context Engineering Impact")
    print("=" * 80)
    print()

    # Setup paths
    workspace = Path("./benchmark_workspace")
    workspace.mkdir(exist_ok=True)

    results_dir = Path("./benchmark_results")
    results_dir.mkdir(exist_ok=True)

    # Initialize LLM (using mock for demo)
    print("📝 Initializing LLM provider...")
    # In production, use: llm = LLMProvider.from_name("deepseek")
    # For demo, we'll create a minimal mock
    llm = None  # Replace with actual LLM provider

    # Create agent factory
    def agent_factory(ctx, llm_provider):
        """Factory function to create cognitive agents."""
        return CognitiveAgent(
            ctx=ctx,
            llm=llm_provider,
            config=CognitiveConfig(
                system_prompt="You are a software engineering assistant. Fix bugs and complete coding tasks.",
                max_iterations=20,
            ),
            workspace_path=workspace,
        )

    # Create benchmark runner
    print("🚀 Creating benchmark runner...")
    runner = BenchmarkRunner(
        agent_factory=agent_factory,
        llm=llm,
        benchmark="swe-bench",
        dataset_path="./datasets/swe-bench-lite",  # Will use synthetic tasks if not found
        workspace_path=workspace,
    )

    # Demo 1: Run with context engineering (default)
    print("\n" + "-" * 80)
    print("Demo 1: Running with Context Engineering")
    print("-" * 80)

    optimized_results = await runner.run(
        max_tasks=5,
        context_engineering=True,
        verbose=True,
    )

    # Save optimized results
    optimized_path = results_dir / "optimized.json"
    optimized_results.save(optimized_path)
    print(f"\n💾 Saved optimized results to: {optimized_path}")

    # Demo 2: Run without context engineering (baseline)
    print("\n" + "-" * 80)
    print("Demo 2: Running without Context Engineering (Baseline)")
    print("-" * 80)

    baseline_results = await runner.run(
        max_tasks=5,
        context_engineering=False,
        verbose=True,
    )

    # Save baseline results
    baseline_path = results_dir / "baseline.json"
    baseline_results.save(baseline_path)
    print(f"\n💾 Saved baseline results to: {baseline_path}")

    # Demo 3: Generate comparison report
    print("\n" + "-" * 80)
    print("Demo 3: Comparison Report")
    print("-" * 80)

    from loom.benchmark import ComparisonReport

    comparison = ComparisonReport(
        baseline=baseline_results,
        optimized=optimized_results,
    )

    # Print formatted comparison
    comparison.print_summary()

    # Save comparison
    comparison_path = results_dir / "comparison.json"
    comparison.save(comparison_path)
    print(f"💾 Saved comparison report to: {comparison_path}")

    # Demo 4: Key insights
    print("\n" + "-" * 80)
    print("Key Insights")
    print("-" * 80)
    print(f"✅ Token Reduction: {comparison.token_reduction():.1f}%")
    print(f"✅ Cost Reduction: {comparison.cost_reduction():.1f}%")
    print(f"✅ Quality Delta: {comparison.quality_delta():+.1f}%")
    print(f"✅ Time Delta: {(optimized_results.avg_time - baseline_results.avg_time):+.1f}s")
    print()

    # Provide CLI examples
    print("\n" + "=" * 80)
    print("CLI Usage Examples")
    print("=" * 80)
    print("\n# Run benchmark with 10 tasks:")
    print("loom benchmark run swe-bench --dataset ./datasets/swe-bench-lite --max-tasks 10 -v")
    print("\n# Run comparison:")
    print("loom benchmark compare swe-bench --dataset ./datasets/swe-bench-lite --max-tasks 10")
    print("\n# Generate report:")
    print("loom benchmark report results/comparison.json --format markdown")
    print("\n" + "=" * 80 + "\n")


if __name__ == "__main__":
    asyncio.run(main())
