"""Integration tests for benchmark CLI.

These tests ensure the CLI commands work end-to-end,
catching issues like missing arguments that unit tests don't cover.
"""

from unittest.mock import Mock, patch

import pytest

from loom.cli.benchmark import cmd_benchmark_compare, cmd_benchmark_run


class TestBenchmarkCLI:
    """Test benchmark CLI commands."""

    @pytest.fixture
    def mock_args(self, tmp_path):
        """Create mock CLI arguments."""
        args = Mock()
        args.benchmark = "swe-bench"
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

            # Verify llm_config was passed (not llm provider)
            call_kwargs = MockRunner.call_args[1]
            assert "llm_config" in call_kwargs
            assert "llm" not in call_kwargs  # Should NOT have 'llm' parameter

    @pytest.mark.asyncio
    async def test_cmd_benchmark_run_with_invalid_llm(self, mock_args, capsys):
        """Test that invalid LLM provider name is caught."""
        mock_args.llm = "invalid_provider"

        await cmd_benchmark_run(mock_args)

        captured = capsys.readouterr()
        assert "Unknown LLM provider" in captured.out
        assert "invalid_provider" in captured.out

    @pytest.mark.asyncio
    async def test_cmd_benchmark_run_creates_agent_factory_correctly(self, mock_args):
        """Test that agent_factory is created with correct signature."""
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

            await cmd_benchmark_run(mock_args)

            # Get the agent_factory that was passed
            agent_factory = MockRunner.call_args[1]["agent_factory"]

            # Verify it's callable
            assert callable(agent_factory)

            # Verify the factory has correct signature by inspecting it
            import inspect

            sig = inspect.signature(agent_factory)
            params = list(sig.parameters.keys())

            # Should accept exactly 2 parameters (ctx and llm_config_arg)
            assert len(params) == 2, f"Expected 2 params, got {len(params)}: {params}"

            # This verifies we fixed the bug where TypeError occurred

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

            # Verify BenchmarkRunner was created with llm_config
            assert MockRunner.called
            call_kwargs = MockRunner.call_args[1]
            assert "llm_config" in call_kwargs
            assert "llm" not in call_kwargs

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

    @pytest.mark.asyncio
    async def test_runner_accepts_llm_config_not_provider(self):
        """Test that BenchmarkRunner accepts LLMConfig, not LLMProvider."""
        from loom.benchmark import BenchmarkRunner
        from loom.llm import LLMProvider

        # Create a dummy LLMConfig
        llm_config = LLMProvider.LOCAL

        # Create a dummy agent factory
        def agent_factory(ctx, llm_config_arg):
            # Factory receives LLMConfig, creates provider internally
            from loom.cognitive import CognitiveAgent

            # Agent factory would create LLMProvider internally (we just mock it)
            _ = LLMProvider(ctx, llm_config_arg)  # Simulate creation
            return Mock(spec=CognitiveAgent)

        # This should work without errors
        runner = BenchmarkRunner(
            agent_factory=agent_factory,
            llm_config=llm_config,
            benchmark="swe-bench",
            dataset_path="/tmp/test",
        )

        assert runner.llm_config is llm_config
        assert callable(runner.agent_factory)
