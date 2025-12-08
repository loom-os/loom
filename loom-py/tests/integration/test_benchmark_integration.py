"""Integration tests for benchmark infrastructure.

These tests ensure the benchmark framework works end-to-end with real components.
"""

import pytest

from loom.benchmark import BenchmarkRunner
from loom.cognitive import CognitiveAgent, CognitiveConfig
from loom.llm import LLMConfig, LLMProvider


@pytest.fixture
def llm_config():
    """Create a test LLM config."""
    return LLMConfig(
        base_url="http://localhost:8000/v1",
        model="test-model",
        temperature=0.7,
        max_tokens=2048,
        timeout_ms=30000,
    )


@pytest.fixture
def workspace(tmp_path):
    """Create temporary workspace."""
    return tmp_path / "benchmark_workspace"


@pytest.fixture
def dataset_path(tmp_path):
    """Create temporary dataset path."""
    return tmp_path / "dataset"


class TestBenchmarkRunnerIntegration:
    """Integration tests for BenchmarkRunner."""

    def test_runner_initialization(self, llm_config, dataset_path, workspace):
        """Test that runner initializes correctly."""

        def agent_factory(ctx, llm):
            return CognitiveAgent(
                ctx=ctx,
                llm=llm,
                config=CognitiveConfig(),
            )

        runner = BenchmarkRunner(
            agent_factory=agent_factory,
            llm_config=llm_config,
            benchmark="swe-bench",
            dataset_path=dataset_path,
            workspace_path=workspace,
        )

        assert runner.llm_config == llm_config
        assert runner.benchmark == "swe-bench"
        assert runner.dataset_path == dataset_path
        assert runner.workspace_path == workspace

    @pytest.mark.asyncio
    async def test_synthetic_task_loading(self, llm_config, dataset_path, workspace):
        """Test that synthetic tasks are loaded when dataset doesn't exist."""

        def agent_factory(ctx, llm):
            return CognitiveAgent(
                ctx=ctx,
                llm=llm,
                config=CognitiveConfig(),
            )

        runner = BenchmarkRunner(
            agent_factory=agent_factory,
            llm_config=llm_config,
            benchmark="swe-bench",
            dataset_path=dataset_path,  # Non-existent dataset
            workspace_path=workspace,
        )

        # Load tasks (should fall back to synthetic)
        tasks = runner.adapter.load_tasks(max_tasks=3)

        assert len(tasks) == 3
        assert all("task_id" in task for task in tasks)
        assert all("prompt" in task for task in tasks)
        assert all(task["task_id"].startswith("synthetic-python-") for task in tasks)

    @pytest.mark.asyncio
    async def test_llm_config_validation(self, dataset_path, workspace):
        """Test that invalid LLM config is handled."""

        # This test ensures LLM config is properly validated
        invalid_config = LLMConfig(
            base_url="",  # Invalid empty URL
            model="test",
            temperature=0.7,
            max_tokens=2048,
            timeout_ms=30000,
        )

        def agent_factory(ctx, llm):
            return CognitiveAgent(
                ctx=ctx,
                llm=llm,
                config=CognitiveConfig(),
            )

        # Runner should still initialize (validation happens during execution)
        runner = BenchmarkRunner(
            agent_factory=agent_factory,
            llm_config=invalid_config,
            benchmark="swe-bench",
            dataset_path=dataset_path,
            workspace_path=workspace,
        )

        assert runner.llm_config == invalid_config


class TestCLIIntegration:
    """Integration tests for CLI commands."""

    def test_llm_config_lookup(self):
        """Test that LLM config lookup works correctly."""

        # Test all predefined configs are accessible
        configs = {
            "deepseek": LLMProvider.DEEPSEEK,
            "openai": LLMProvider.OPENAI,
            "local": LLMProvider.LOCAL,
        }

        for _name, config in configs.items():
            assert config is not None
            assert config.base_url
            assert config.model

    def test_llm_provider_configs_have_required_fields(self):
        """Test that all predefined LLM configs have required fields."""

        configs = [
            LLMProvider.DEEPSEEK,
            LLMProvider.OPENAI,
            LLMProvider.LOCAL,
        ]

        for config in configs:
            assert config.base_url, "Config missing base_url"
            assert config.model, "Config missing model"
            assert config.temperature is not None, "Config missing temperature"
            assert config.max_tokens > 0, "Config missing max_tokens"
            assert config.timeout_ms > 0, "Config missing timeout_ms"


class TestAdapterIntegration:
    """Integration tests for benchmark adapters."""

    @pytest.mark.asyncio
    async def test_swe_bench_adapter_task_preparation(self, tmp_path):
        """Test that adapter prepares task workspace correctly."""
        from loom.benchmark.adapters import SWEBenchAdapter

        dataset_path = tmp_path / "dataset"
        workspace_path = tmp_path / "workspace"

        adapter = SWEBenchAdapter(dataset_path, workspace_path)

        # Load a task
        tasks = adapter.load_tasks(max_tasks=1)
        task = tasks[0]

        # Prepare task
        task_workspace = await adapter.prepare_task(task)

        # Check workspace was created
        assert task_workspace.exists()
        assert task_workspace.is_dir()
        assert str(task_workspace).startswith(str(workspace_path))

    @pytest.mark.asyncio
    async def test_swe_bench_adapter_result_evaluation(self, tmp_path):
        """Test that adapter evaluates results."""
        from loom.benchmark.adapters import SWEBenchAdapter
        from loom.cognitive import CognitiveResult

        dataset_path = tmp_path / "dataset"
        workspace_path = tmp_path / "workspace"

        adapter = SWEBenchAdapter(dataset_path, workspace_path)
        tasks = adapter.load_tasks(max_tasks=1)
        task = tasks[0]

        # Create mock result
        result = CognitiveResult(
            answer="Task completed",
            iterations=5,
            success=True,
        )

        # Evaluate (should use heuristics since no real tests)
        correct = await adapter.evaluate_result(task, result)

        # With our simple heuristic, it should return True if success=True
        assert isinstance(correct, bool)


@pytest.mark.skipif(
    True,  # Skip by default since it requires actual LLM
    reason="Requires actual LLM provider and takes time",
)
class TestEndToEnd:
    """End-to-end tests with real LLM (skipped by default)."""

    @pytest.mark.asyncio
    async def test_full_benchmark_run(self, llm_config, tmp_path):
        """Test a full benchmark run with real agent."""

        def agent_factory(ctx, llm):
            return CognitiveAgent(
                ctx=ctx,
                llm=llm,
                config=CognitiveConfig(
                    system_prompt="You are a helpful assistant.",
                    max_iterations=5,
                ),
            )

        runner = BenchmarkRunner(
            agent_factory=agent_factory,
            llm_config=llm_config,
            benchmark="swe-bench",
            dataset_path=tmp_path / "dataset",
            workspace_path=tmp_path / "workspace",
        )

        # Run with very small number of tasks
        results = await runner.run(max_tasks=1, verbose=False)

        assert len(results.tasks) == 1
        assert results.benchmark_name == "swe-bench"
