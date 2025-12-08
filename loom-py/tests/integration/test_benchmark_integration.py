"""Integration tests for benchmark infrastructure.

These tests ensure the benchmark framework works end-to-end with real components.
"""

from unittest.mock import Mock, patch

import pytest

from loom.benchmark import BenchmarkRunner
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

    def test_runner_initialization(self, llm_config, dataset_path, workspace, tmp_path):
        """Test that runner initializes correctly with agent_path."""
        # Create a minimal loom.toml
        agent_dir = tmp_path / "test_agent"
        agent_dir.mkdir()
        (agent_dir / "loom.toml").write_text(
            """
name = "test"
[agents.test]
llm_provider = "deepseek"
"""
        )

        runner = BenchmarkRunner(
            agent_path=agent_dir,
            benchmark="swe-bench",
            dataset_path=dataset_path,
            workspace_path=workspace,
        )

        assert runner.benchmark == "swe-bench"
        assert runner.dataset_path == dataset_path
        assert runner.workspace_path == workspace

    @pytest.mark.skip(reason="Synthetic tasks removed - real dataset required")
    @pytest.mark.asyncio
    async def test_synthetic_task_loading(self, llm_config, dataset_path, workspace):
        """Test that synthetic tasks are loaded when dataset doesn't exist."""
        # This test is deprecated - we now require real datasets
        pass

    @pytest.mark.skip(reason="LLM config now loaded from loom.toml, not passed directly")
    @pytest.mark.asyncio
    async def test_llm_config_validation(self, dataset_path, workspace):
        """Test that invalid LLM config is handled."""
        # This test is deprecated - LLM config is now in loom.toml
        pass


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
    @patch("subprocess.run")
    async def test_swe_bench_adapter_task_preparation(self, mock_run, tmp_path):
        """Test that adapter prepares tasks correctly."""
        import json

        from loom.benchmark.adapters import SWEBenchAdapter

        # Mock git operations
        mock_run.return_value = Mock(returncode=0, stdout="", stderr="")

        dataset_path = tmp_path / "dataset"
        dataset_path.mkdir()
        workspace_path = tmp_path / "workspace"
        workspace_path.mkdir()

        # Create a minimal tasks.json
        tasks_data = [
            {
                "instance_id": "test-repo-1",
                "repo": "test/repo",
                "base_commit": "abc123",
                "problem_statement": "Fix the bug",
                "patch": "diff...",
                "test_patch": "test diff...",
            }
        ]
        (dataset_path / "tasks.json").write_text(json.dumps(tasks_data))

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
        # Verify git operations were attempted
        assert mock_run.called

    @pytest.mark.asyncio
    async def test_swe_bench_adapter_result_evaluation(self, tmp_path):
        """Test that adapter evaluates results."""
        import json

        from loom.benchmark.adapters import SWEBenchAdapter
        from loom.cognitive import CognitiveResult

        dataset_path = tmp_path / "dataset"
        dataset_path.mkdir()
        workspace_path = tmp_path / "workspace"
        workspace_path.mkdir()

        # Create tasks.json
        tasks_data = [
            {
                "instance_id": "test-repo-1",
                "repo": "test/repo",
                "base_commit": "abc123",
                "problem_statement": "Fix the bug",
                "patch": "diff...",
                "test_patch": "test diff...",
            }
        ]
        (dataset_path / "tasks.json").write_text(json.dumps(tasks_data))

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


@pytest.mark.skip(reason="API changed - requires real dataset and agent_path setup")
class TestEndToEnd:
    """End-to-end tests with real LLM (skipped - needs API update)."""

    @pytest.mark.asyncio
    async def test_full_benchmark_run(self, llm_config, tmp_path):
        """Test a full benchmark run with real agent."""
        pass
