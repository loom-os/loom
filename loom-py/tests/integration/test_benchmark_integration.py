"""Integration tests for benchmark infrastructure.

These tests ensure the benchmark framework works end-to-end with real components.
"""

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

    @pytest.mark.skip(reason="No adapters implemented yet - GAIA/TAU-bench pending")
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
            benchmark="test-benchmark",
            dataset_path=dataset_path,
            workspace_path=workspace,
        )

        assert runner.benchmark == "test-benchmark"
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


@pytest.mark.skip(reason="SWE-bench adapter removed - pivot to GAIA/TAU-bench")
class TestAdapterIntegration:
    """Integration tests for benchmark adapters (deprecated)."""

    @pytest.mark.asyncio
    async def test_adapter_placeholder(self):
        """Placeholder for future GAIA/TAU-bench adapter tests."""
        pass


@pytest.mark.skip(reason="API changed - requires real dataset and agent_path setup")
class TestEndToEnd:
    """End-to-end tests with real LLM (skipped - needs API update)."""

    @pytest.mark.asyncio
    async def test_full_benchmark_run(self, llm_config, tmp_path):
        """Test a full benchmark run with real agent."""
        pass
