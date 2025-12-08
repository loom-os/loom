"""Benchmark runner for executing agent tasks and collecting metrics.

This module provides the BenchmarkRunner class for running agent benchmarks
with configurable context engineering settings.
"""

from __future__ import annotations

import time
from datetime import datetime
from pathlib import Path
from typing import TYPE_CHECKING, Optional

from opentelemetry import trace

from ..cognitive import CognitiveAgent
from .metrics import BenchmarkResults, ComparisonReport, TaskMetrics

if TYPE_CHECKING:
    pass

# Get tracer
tracer = trace.get_tracer(__name__)


class BenchmarkRunner:
    """Execute benchmark tasks and collect performance metrics.

    This runner can execute tasks with or without context engineering enabled,
    allowing for direct comparison of performance characteristics.

    Example:
        ```python
        # Option 1: Use existing agent from a project directory
        runner = BenchmarkRunner(
            agent_path="apps/chat-assistant",
            benchmark="swe-bench",
            dataset_path="./datasets/swe-bench-lite"
        )

        # Option 2: Provide a pre-configured agent instance
        runner = BenchmarkRunner(
            agent=cognitive_agent,
            benchmark="swe-bench",
            dataset_path="./datasets/swe-bench-lite"
        )

        # Run with context engineering
        results = await runner.run(max_tasks=50)

        # Run comparison
        comparison = await runner.run_comparison(max_tasks=50)
        comparison.print_summary()
        ```
    """

    def __init__(
        self,
        benchmark: str,
        dataset_path: Path | str,
        agent: Optional[CognitiveAgent] = None,
        agent_path: Optional[Path | str] = None,
        workspace_path: Optional[Path | str] = None,
    ):
        """Initialize benchmark runner.

        Args:
            benchmark: Benchmark name (e.g., "swe-bench", "webarena")
            dataset_path: Path to benchmark dataset
            agent: Pre-configured CognitiveAgent instance (provide this OR agent_path)
            agent_path: Path to agent project directory with loom.toml (provide this OR agent)
            workspace_path: Optional workspace path for agents

        Raises:
            ValueError: If neither agent nor agent_path is provided, or both are provided
        """
        if agent is None and agent_path is None:
            raise ValueError("Must provide either 'agent' or 'agent_path'")
        if agent is not None and agent_path is not None:
            raise ValueError("Cannot provide both 'agent' and 'agent_path', choose one")

        self.agent = agent
        self.agent_path = Path(agent_path) if agent_path else None
        self.benchmark = benchmark
        self.dataset_path = Path(dataset_path)
        self.workspace_path = Path(workspace_path) if workspace_path else Path.cwd()

        # Load adapter for the benchmark
        self.adapter = self._load_adapter(benchmark)

    async def _load_agent_from_path(self, agent_path: Path) -> CognitiveAgent:
        """Load agent from a project directory with loom.toml.

        Args:
            agent_path: Path to agent project directory

        Returns:
            Configured CognitiveAgent instance

        Raises:
            FileNotFoundError: If loom.toml not found
            ValueError: If configuration is invalid
        """
        from ..agent import Agent
        from ..cognitive import CognitiveConfig, ThinkingStrategy
        from ..llm import LLMProvider
        from ..runtime.config import load_project_config

        # Load project config
        config = load_project_config(agent_path)

        # Find the agent configuration (use first agent if multiple)
        if not config.agents:
            raise ValueError(f"No agents defined in {agent_path}/loom.toml")

        agent_id = list(config.agents.keys())[0]
        agent_config = config.agents[agent_id]

        # Create base agent
        base_agent = Agent(
            agent_id=agent_id,
            topics=agent_config.get("topics", []),
            address=config.bridge.address,
        )
        await base_agent.start()

        # Create LLM provider
        llm_name = agent_config.get("llm_provider", "deepseek")
        llm = LLMProvider.from_config(base_agent._ctx, llm_name, config)

        # Create cognitive config
        strategy_name = agent_config.get("thinking_strategy", "react")
        strategy = {
            "react": ThinkingStrategy.REACT,
            "single_shot": ThinkingStrategy.SINGLE_SHOT,
            "chain_of_thought": ThinkingStrategy.CHAIN_OF_THOUGHT,
        }.get(strategy_name, ThinkingStrategy.REACT)

        cognitive_config = CognitiveConfig(
            system_prompt=agent_config.get(
                "system_prompt",
                "You are a software engineering assistant. Fix bugs and complete coding tasks.",
            ),
            max_iterations=agent_config.get("max_iterations", 20),
            thinking_strategy=strategy,
        )

        # Create cognitive agent
        cognitive = CognitiveAgent(
            ctx=base_agent._ctx,
            llm=llm,
            config=cognitive_config,
            workspace_path=self.workspace_path,
        )

        return cognitive

    def _load_adapter(self, benchmark: str):
        """Load the appropriate adapter for the benchmark."""
        if benchmark == "swe-bench":
            from .adapters.swe_bench import SWEBenchAdapter

            return SWEBenchAdapter(self.dataset_path, self.workspace_path)
        else:
            raise ValueError(f"Unsupported benchmark: {benchmark}")

    async def run(
        self,
        max_tasks: int = 100,
        context_engineering: bool = True,
        verbose: bool = False,
    ) -> BenchmarkResults:
        """Run benchmark tasks and collect metrics.

        Args:
            max_tasks: Maximum number of tasks to run
            context_engineering: Whether to enable context engineering
            verbose: Whether to print progress

        Returns:
            BenchmarkResults with all task metrics
        """
        with tracer.start_as_current_span("benchmark_run") as span:
            span.set_attribute("benchmark", self.benchmark)
            span.set_attribute("max_tasks", max_tasks)
            span.set_attribute("context_engineering", context_engineering)

            results = BenchmarkResults(
                benchmark_name=self.benchmark,
                context_engineering_enabled=context_engineering,
                timestamp=datetime.now().isoformat(),
            )

            tasks = self.adapter.load_tasks(max_tasks)

            if verbose:
                print(f"\n🚀 Running {len(tasks)} tasks from {self.benchmark}")
                print(
                    f"Context Engineering: {'✅ Enabled' if context_engineering else '❌ Disabled'}\n"
                )

            for i, task in enumerate(tasks):
                if verbose:
                    print(f"[{i+1}/{len(tasks)}] Running task: {task['task_id']}")

                metrics = await self._run_task(task, context_engineering, verbose)
                results.tasks.append(metrics)

                if verbose:
                    status = "✅" if metrics.success else "❌"
                    print(
                        f"{status} {metrics.task_id}: {metrics.execution_time_sec:.1f}s, {metrics.total_tokens} tokens\n"
                    )

            if verbose:
                print("\n📊 Summary:")
                summary = results.summary()
                for key, value in summary.items():
                    print(f"  {key}: {value}")

            return results

    async def _run_task(
        self,
        task: dict,
        context_engineering: bool,
        verbose: bool,
    ) -> TaskMetrics:
        """Run a single benchmark task and collect metrics.

        Args:
            task: Task specification from adapter
            context_engineering: Whether to enable context engineering
            verbose: Whether to print details

        Returns:
            TaskMetrics for this task
        """
        task_id = task["task_id"]
        start_time = time.time()

        try:
            # Prepare task workspace
            await self.adapter.prepare_task(task)

            # Get or load cognitive agent
            if self.agent:
                cognitive_agent = self.agent
            else:
                # Load agent from project directory
                cognitive_agent = await self._load_agent_from_path(self.agent_path)

            # Configure context engineering
            if not context_engineering:
                # Disable context engineering features
                if hasattr(cognitive_agent, "step_reducer"):
                    cognitive_agent.step_reducer = None
                if hasattr(cognitive_agent, "step_compactor"):
                    cognitive_agent.step_compactor = None
                if hasattr(cognitive_agent, "data_offloader"):
                    cognitive_agent.data_offloader = None

            # Run task
            result = await cognitive_agent.run(task["prompt"])

            # Calculate metrics
            execution_time = time.time() - start_time

            # Evaluate correctness
            correct = await self.adapter.evaluate_result(task, result)

            # Count context engineering usage
            offloaded_files = 0
            compacted_steps = 0
            if context_engineering and hasattr(cognitive_agent, "data_offloader"):
                cache_dir = cognitive_agent.data_offloader.cache_dir
                if cache_dir.exists():
                    # Count all offloaded files (various extensions)
                    offloaded_files = len(list(cache_dir.glob("*")))
            if context_engineering and hasattr(cognitive_agent, "step_compactor"):
                compacted_steps = getattr(cognitive_agent.step_compactor, "total_compacted", 0)

            # Token usage (from cognitive result or LLM provider)
            total_tokens = getattr(result, "total_tokens", 0)
            avg_prompt_tokens = getattr(result, "avg_prompt_tokens", 0)
            peak_prompt_tokens = getattr(result, "peak_prompt_tokens", 0)

            # Estimate token savings (rough heuristic)
            token_savings_pct = 0.0
            if context_engineering and offloaded_files + compacted_steps > 0:
                # Rough estimate: 70-85% savings based on offloading + compaction
                token_savings_pct = min(85.0, (offloaded_files * 10 + compacted_steps * 5))

            # Cost calculation (rough estimate based on tokens)
            # Assuming ~$0.05 per 1K tokens (adjust based on actual LLM pricing)
            total_cost_usd = (total_tokens / 1000) * 0.05

            return TaskMetrics(
                task_id=task_id,
                success=result.success,
                iterations=result.iterations,
                total_tokens=total_tokens,
                total_cost_usd=total_cost_usd,
                execution_time_sec=execution_time,
                offloaded_files=offloaded_files,
                compacted_steps=compacted_steps,
                avg_prompt_tokens=avg_prompt_tokens,
                peak_prompt_tokens=peak_prompt_tokens,
                token_savings_pct=token_savings_pct,
                correct_answer=correct,
            )

        except Exception as e:
            execution_time = time.time() - start_time
            if verbose:
                print(f"❌ Error in task {task_id}: {e}")

            return TaskMetrics(
                task_id=task_id,
                success=False,
                iterations=0,
                total_tokens=0,
                total_cost_usd=0.0,
                execution_time_sec=execution_time,
                correct_answer=False,
                error_message=str(e),
            )

    async def run_comparison(
        self,
        max_tasks: int = 50,
        verbose: bool = True,
    ) -> ComparisonReport:
        """Run benchmark with and without context engineering for comparison.

        Args:
            max_tasks: Maximum number of tasks to run
            verbose: Whether to print progress

        Returns:
            ComparisonReport with baseline and optimized results
        """
        if verbose:
            print("\n" + "=" * 80)
            print("BENCHMARK COMPARISON")
            print("=" * 80)

        # Run baseline (no context engineering)
        if verbose:
            print("\n🔍 Phase 1: Baseline (No Context Engineering)")
        baseline = await self.run(
            max_tasks=max_tasks,
            context_engineering=False,
            verbose=verbose,
        )

        # Run optimized (with context engineering)
        if verbose:
            print("\n✨ Phase 2: Optimized (With Context Engineering)")
        optimized = await self.run(
            max_tasks=max_tasks,
            context_engineering=True,
            verbose=verbose,
        )

        # Create comparison report
        report = ComparisonReport(baseline=baseline, optimized=optimized)

        if verbose:
            report.print_summary()

        return report


__all__ = ["BenchmarkRunner"]
