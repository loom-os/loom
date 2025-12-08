"""Integration tests for benchmark CLI.

These tests ensure the CLI commands work end-to-end,
catching issues like missing arguments that unit tests don't cover.
"""

from unittest.mock import AsyncMock, Mock, patch

import pytest

from loom.cli.benchmark import cmd_benchmark_compare, cmd_benchmark_run


class TestBenchmarkCLI:
    """Test benchmark CLI commands."""

    @pytest.fixture
    def mock_args(self, tmp_path):
        """Create mock CLI arguments."""
        args = Mock()
        args.benchmark = "test-benchmark"
        args.dataset = str(tmp_path / "dataset")
        args.max_tasks = 2
        args.no_context_engineering = False
        args.output = None
        args.workspace = str(tmp_path / "workspace")
        args.llm = "local"
        args.verbose = False
        return args

    @pytest.mark.asyncio
    async def test_cmd_benchmark_run_with_valid_llm(self, mock_args, tmp_path):
        """Test that cmd_benchmark_run accepts valid LLM provider names."""
        # Mock the BenchmarkRunner at the import location
        with patch("loom.benchmark.BenchmarkRunner") as MockRunner:
            mock_runner = Mock()
            MockRunner.return_value = mock_runner

            # Mock the run method as a coroutine
            mock_result = Mock()
            mock_result.success_rate = 1.0
            mock_result.avg_tokens = 1000
            mock_result.avg_cost = 0.05
            mock_result.summary = Mock(return_value={"test": "summary"})

            async def async_run(*args, **kwargs):
                return mock_result

            mock_runner.run = async_run

            # Should not raise any errors
            await cmd_benchmark_run(mock_args)

            # Verify BenchmarkRunner was created
            assert MockRunner.called

            # Verify agent_path was passed (new API uses agent_path)
            call_kwargs = MockRunner.call_args[1]
            assert "agent_path" in call_kwargs
            assert "agent_factory" not in call_kwargs  # Old API no longer used

    @pytest.mark.skip(reason="LLM provider validation now happens in loom.toml loading, not CLI")
    @pytest.mark.asyncio
    async def test_cmd_benchmark_run_with_invalid_llm(self, mock_args, capsys, tmp_path):
        """Test that agent_path must be a valid path."""
        # Use real path to avoid Mock conversion issues
        mock_args.agent_path = str(tmp_path / "nonexistent")
        mock_args.llm = "deepseek"  # Valid LLM

        # This should work now (path will be created if needed)
        with patch("loom.benchmark.BenchmarkRunner") as MockRunner:
            mock_runner = Mock()
            MockRunner.return_value = mock_runner
            mock_runner.run = AsyncMock(return_value=Mock(summary=Mock(return_value={})))

            await cmd_benchmark_run(mock_args)

            # Verify it was called
            assert MockRunner.called

        captured = capsys.readouterr()
        assert "Unknown LLM provider" in captured.out
        assert "invalid_provider" in captured.out

    @pytest.mark.asyncio
    async def test_cmd_benchmark_run_loads_agent_from_path(self, mock_args, tmp_path):
        """Test that agent is loaded from loom.toml path."""
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

        mock_args.agent_path = agent_dir

        with patch("loom.benchmark.BenchmarkRunner") as MockRunner:
            mock_runner = Mock()
            MockRunner.return_value = mock_runner
            mock_runner.run = AsyncMock(return_value=Mock(summary=Mock(return_value={})))

            await cmd_benchmark_run(mock_args)

            # Verify agent_path was passed correctly
            call_kwargs = MockRunner.call_args[1]
            assert "agent_path" in call_kwargs
            # Should be a Path object
            from pathlib import Path

            assert isinstance(call_kwargs["agent_path"], Path)

    @pytest.mark.asyncio
    async def test_cmd_benchmark_compare_with_valid_llm(self, mock_args):
        """Test that cmd_benchmark_compare works with valid LLM."""
        with patch("loom.benchmark.BenchmarkRunner") as MockRunner:
            mock_runner = Mock()
            MockRunner.return_value = mock_runner

            # Mock run_comparison to return a ComparisonReport
            async def async_run_comparison(*args, **kwargs):
                from loom.benchmark.metrics import ComparisonReport

                mock_baseline = Mock(success_rate=0.8, avg_tokens=1200, avg_cost=0.06)
                mock_optimized = Mock(success_rate=0.85, avg_tokens=1100, avg_cost=0.055)
                # ComparisonReport is a dataclass, construct it directly
                return ComparisonReport(
                    baseline=mock_baseline,
                    optimized=mock_optimized,
                )

            mock_runner.run_comparison = async_run_comparison

            await cmd_benchmark_compare(mock_args)

            # Verify BenchmarkRunner was created with agent_path (new API)
            assert MockRunner.called
            call_kwargs = MockRunner.call_args[1]
            assert "agent_path" in call_kwargs
            assert "llm_config" not in call_kwargs  # Old API no longer used
            assert "agent_factory" not in call_kwargs  # Old API no longer used

    @pytest.mark.asyncio
    async def test_all_llm_providers_are_valid(self, mock_args):
        """Test that all supported LLM providers work."""
        providers = ["deepseek", "openai", "local"]

        for provider in providers:
            mock_args.llm = provider

            # Create fresh mocks for each iteration
            with patch("loom.benchmark.BenchmarkRunner") as MockRunner:
                mock_runner = Mock()
                MockRunner.return_value = mock_runner

                async def async_run(*args, **kwargs):
                    return Mock(
                        success_rate=1.0,
                        avg_tokens=1000,
                        avg_cost=0.05,
                        summary=Mock(return_value={}),
                    )

                mock_runner.run = async_run

                # Should not raise any errors
                await cmd_benchmark_run(mock_args)

                # Verify it was called
                assert MockRunner.called, f"Provider {provider} failed - MockRunner was not called"


class TestBenchmarkRunnerIntegration:
    """Integration tests for BenchmarkRunner."""

    @pytest.mark.skip(reason="API changed - now uses agent_path instead of agent_factory")
    @pytest.mark.asyncio
    async def test_runner_accepts_llm_config_not_provider(self):
        """Test that runner accepts LLM config object - DEPRECATED."""
        pass
