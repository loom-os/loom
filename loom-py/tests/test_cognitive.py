"""Unit tests for cognitive module - streaming and non-streaming."""

import asyncio
from typing import AsyncIterator

import pytest

from loom.cognitive import (
    CognitiveAgent,
    CognitiveConfig,
    CognitiveResult,
    ThinkingStrategy,
    ThoughtStep,
)
from loom.cognitive.types import Observation, ToolCall

# ============================================================================
# Mock Fixtures
# ============================================================================


class MockContext:
    """Mock Context for testing without real Bridge connection."""

    def __init__(self, agent_id: str = "test-agent"):
        self.agent_id = agent_id
        self._tool_results = {}

    def set_tool_result(self, tool_name: str, result: str):
        """Set mock result for a tool call."""
        self._tool_results[tool_name] = result

    async def tool(self, name: str, payload: dict = None, timeout_ms: int = 30000) -> str:
        """Mock tool execution."""
        if name in self._tool_results:
            return self._tool_results[name]
        return f'{{"result": "mock result for {name}"}}'


class MockLLMProvider:
    """Mock LLM provider for testing."""

    def __init__(self):
        self.responses = []
        self.stream_chunks = []
        self._call_count = 0
        self._stream_call_count = 0

    def set_responses(self, responses: list[str]):
        """Set sequence of responses for generate() calls."""
        self.responses = responses
        self._call_count = 0

    def set_stream_chunks(self, chunks: list[str]):
        """Set chunks for generate_stream() calls."""
        self.stream_chunks = chunks
        self._stream_call_count = 0

    def set_stream_responses(self, responses: list[list[str]]):
        """Set multiple streaming responses for multiple iterations."""
        self._stream_responses = responses
        self._stream_call_count = 0

    async def generate(
        self,
        prompt: str,
        *,
        system: str = None,
        temperature: float = None,
        max_tokens: int = None,
        timeout_ms: int = None,
    ) -> str:
        """Mock generate - returns next response in sequence."""
        if self._call_count < len(self.responses):
            response = self.responses[self._call_count]
            self._call_count += 1
            return response
        return "FINAL ANSWER: No more responses configured"

    async def generate_stream(
        self,
        prompt: str,
        *,
        system: str = None,
        temperature: float = None,
        max_tokens: int = None,
        timeout_ms: int = None,
    ) -> AsyncIterator[str]:
        """Mock streaming generate - yields chunks."""
        # Support multiple iterations with different responses
        if hasattr(self, "_stream_responses") and self._stream_call_count < len(
            self._stream_responses
        ):
            chunks = self._stream_responses[self._stream_call_count]
            self._stream_call_count += 1
        else:
            chunks = self.stream_chunks

        for chunk in chunks:
            yield chunk
            await asyncio.sleep(0)  # Allow other coroutines to run


@pytest.fixture
def mock_ctx():
    """Create a mock context."""
    return MockContext()


@pytest.fixture
def mock_llm():
    """Create a mock LLM provider."""
    return MockLLMProvider()


@pytest.fixture
def cognitive_config():
    """Create default cognitive config for testing."""
    return CognitiveConfig(
        system_prompt="You are a helpful test assistant.",
        thinking_strategy=ThinkingStrategy.REACT,
        max_iterations=5,
        temperature=0.7,
    )


@pytest.fixture
def cognitive_agent(mock_ctx, mock_llm, cognitive_config):
    """Create a CognitiveAgent with mocks."""
    return CognitiveAgent(
        ctx=mock_ctx,
        llm=mock_llm,
        config=cognitive_config,
        available_tools=["weather:get", "system:shell"],
    )


# ============================================================================
# Non-Streaming Tests
# ============================================================================


class TestCognitiveAgentRun:
    """Tests for CognitiveAgent.run() method."""

    @pytest.mark.asyncio
    async def test_run_single_shot(self, mock_ctx, mock_llm):
        """Test single-shot strategy returns direct answer."""
        config = CognitiveConfig(
            thinking_strategy=ThinkingStrategy.SINGLE_SHOT,
            temperature=0.5,
        )
        agent = CognitiveAgent(ctx=mock_ctx, llm=mock_llm, config=config)

        mock_llm.set_responses(["This is a direct answer without tools."])

        result = await agent.run("What is 2+2?")

        assert result.success is True
        assert result.answer == "This is a direct answer without tools."
        assert result.iterations == 1
        assert len(result.steps) == 0

    @pytest.mark.asyncio
    async def test_run_react_final_answer(self, cognitive_agent, mock_llm):
        """Test ReAct with immediate final answer."""
        mock_llm.set_responses(["FINAL ANSWER: The answer is 42."])

        result = await cognitive_agent.run("What is the meaning of life?")

        assert result.success is True
        assert "42" in result.answer
        assert result.iterations == 1

    @pytest.mark.asyncio
    async def test_run_react_with_tool_call(self, cognitive_agent, mock_llm, mock_ctx):
        """Test ReAct with tool call then final answer."""
        # Tool call must be in JSON format for parse_react_response
        mock_llm.set_responses(
            [
                'I need to check the weather. {"tool": "weather:get", "args": {"location": "Tokyo"}}',
                "FINAL ANSWER: The weather in Tokyo is sunny, 25°C.",
            ]
        )
        mock_ctx.set_tool_result("weather:get", '{"temp": "25°C", "condition": "sunny"}')

        result = await cognitive_agent.run("What's the weather in Tokyo?")

        assert result.success is True
        assert "Tokyo" in result.answer
        assert len(result.steps) == 1
        assert result.steps[0].tool_call is not None
        assert result.steps[0].tool_call.name == "weather:get"
        assert result.steps[0].observation is not None
        assert result.steps[0].observation.success is True

    @pytest.mark.asyncio
    async def test_run_with_context(self, cognitive_agent, mock_llm):
        """Test run() with context parameter."""
        mock_llm.set_responses(["FINAL ANSWER: Based on the context, the answer is yes."])

        result = await cognitive_agent.run(
            "Should I bring an umbrella?",
            context=["User is in Seattle", "Current month is November"],
        )

        assert result.success is True
        assert result.answer is not None

    @pytest.mark.asyncio
    async def test_run_max_iterations(self, mock_ctx, mock_llm):
        """Test that run respects max_iterations."""
        config = CognitiveConfig(
            thinking_strategy=ThinkingStrategy.REACT,
            max_iterations=2,
        )
        agent = CognitiveAgent(
            ctx=mock_ctx,
            llm=mock_llm,
            config=config,
            available_tools=["test:tool"],
        )

        # Always return tool calls (JSON format), never final answer
        mock_llm.set_responses(
            [
                'Let me try. {"tool": "test:tool", "args": {}}',
                'Try again. {"tool": "test:tool", "args": {}}',
                'One more time. {"tool": "test:tool", "args": {}}',  # Should not reach
            ]
        )

        result = await agent.run("Test iteration limit")

        assert result.iterations == 2  # Should stop at max
        assert len(result.steps) == 2


# ============================================================================
# Streaming Tests
# ============================================================================


class TestCognitiveAgentRunStream:
    """Tests for CognitiveAgent.run_stream() method."""

    @pytest.mark.asyncio
    async def test_run_stream_yields_chunks(self, cognitive_agent, mock_llm):
        """Test that run_stream yields text chunks."""
        mock_llm.set_stream_chunks(
            [
                "FINAL ",
                "ANSWER: ",
                "Hello ",
                "World!",
            ]
        )

        chunks = []
        async for item in cognitive_agent.run_stream("Say hello"):
            if isinstance(item, str):
                chunks.append(item)

        assert len(chunks) == 4
        assert "".join(chunks) == "FINAL ANSWER: Hello World!"

    @pytest.mark.asyncio
    async def test_run_stream_yields_final_result(self, cognitive_agent, mock_llm):
        """Test that run_stream yields CognitiveResult at end."""
        mock_llm.set_stream_chunks(["FINAL ANSWER: Test complete."])

        final_result = None
        async for item in cognitive_agent.run_stream("Test"):
            if isinstance(item, CognitiveResult):
                final_result = item

        assert final_result is not None
        assert final_result.success is True
        assert "Test complete" in final_result.answer

    @pytest.mark.asyncio
    async def test_run_stream_with_context(self, cognitive_agent, mock_llm):
        """Test run_stream with context parameter."""
        mock_llm.set_stream_chunks(["FINAL ANSWER: Context received."])

        results = []
        async for item in cognitive_agent.run_stream(
            "Check context",
            context=["Previous message 1", "Previous message 2"],
        ):
            results.append(item)

        # Should have chunks and final result
        assert len(results) > 0
        assert any(isinstance(r, CognitiveResult) for r in results)

    @pytest.mark.asyncio
    async def test_run_stream_with_tool_call(self, cognitive_agent, mock_llm, mock_ctx):
        """Test run_stream yields ThoughtStep on tool calls."""
        # Set up multiple streaming responses for iterations
        mock_llm.set_stream_responses(
            [
                # First iteration: tool call in JSON format
                ["Need to check weather. ", '{"tool": "weather:get", "args": {"location": "NYC"}}'],
                # Second iteration: final answer
                ["FINAL ANSWER: Weather is 20°C"],
            ]
        )
        mock_ctx.set_tool_result("weather:get", '{"temp": "20°C"}')

        items = []
        async for item in cognitive_agent.run_stream("Weather in NYC?"):
            items.append(item)

        # Should have text chunks, ThoughtSteps, and final result
        text_chunks = [i for i in items if isinstance(i, str)]
        thought_steps = [i for i in items if isinstance(i, ThoughtStep)]
        final_results = [i for i in items if isinstance(i, CognitiveResult)]

        assert len(text_chunks) > 0
        assert len(thought_steps) >= 1
        assert thought_steps[0].tool_call is not None
        assert thought_steps[0].tool_call.name == "weather:get"
        assert len(final_results) == 1

    @pytest.mark.asyncio
    async def test_run_stream_fallback_for_non_react(self, mock_ctx, mock_llm):
        """Test run_stream falls back to run() for non-ReAct strategies."""
        config = CognitiveConfig(
            thinking_strategy=ThinkingStrategy.SINGLE_SHOT,
        )
        agent = CognitiveAgent(ctx=mock_ctx, llm=mock_llm, config=config)

        mock_llm.set_responses(["Direct answer without streaming."])

        results = []
        async for item in agent.run_stream("Test"):
            results.append(item)

        # Should yield single CognitiveResult
        assert len(results) == 1
        assert isinstance(results[0], CognitiveResult)
        assert results[0].answer == "Direct answer without streaming."


# ============================================================================
# Types Tests
# ============================================================================


class TestCognitiveTypes:
    """Tests for cognitive type classes."""

    def test_thought_step_creation(self):
        """Test ThoughtStep dataclass."""
        step = ThoughtStep(
            step=1,
            reasoning="I need to check the weather",
            tool_call=ToolCall(name="weather:get", arguments={"location": "NYC"}),
        )

        assert step.step == 1
        assert step.reasoning == "I need to check the weather"
        assert step.tool_call.name == "weather:get"
        assert step.observation is None

    def test_tool_call_to_dict(self):
        """Test ToolCall.to_dict() method."""
        tc = ToolCall(name="test:tool", arguments={"arg1": "value1"})
        d = tc.to_dict()

        assert d["tool"] == "test:tool"
        assert d["args"] == {"arg1": "value1"}

    def test_observation_success(self):
        """Test Observation for successful tool call."""
        obs = Observation(
            tool_name="weather:get",
            success=True,
            output='{"temp": "25°C"}',
            latency_ms=150,
        )

        assert obs.success is True
        assert obs.error is None
        assert obs.latency_ms == 150

    def test_observation_failure(self):
        """Test Observation for failed tool call."""
        obs = Observation(
            tool_name="weather:get",
            success=False,
            output="",
            error="Connection timeout",
            latency_ms=5000,
        )

        assert obs.success is False
        assert obs.error == "Connection timeout"

    def test_cognitive_result_default(self):
        """Test CognitiveResult default values."""
        result = CognitiveResult(answer="test")

        assert result.answer == "test"
        assert result.iterations == 0
        # Note: success defaults to True in the implementation
        assert result.success is True
        assert result.steps == []
        assert result.error is None


# ============================================================================
# Config Tests
# ============================================================================


class TestCognitiveConfig:
    """Tests for CognitiveConfig."""

    def test_default_config(self):
        """Test default configuration values."""
        config = CognitiveConfig()

        assert config.thinking_strategy == ThinkingStrategy.REACT
        assert config.max_iterations == 10
        assert config.temperature == 0.7
        assert config.system_prompt is None

    def test_custom_config(self):
        """Test custom configuration."""
        config = CognitiveConfig(
            system_prompt="Custom prompt",
            thinking_strategy=ThinkingStrategy.CHAIN_OF_THOUGHT,
            max_iterations=5,
            temperature=0.3,
        )

        assert config.system_prompt == "Custom prompt"
        assert config.thinking_strategy == ThinkingStrategy.CHAIN_OF_THOUGHT
        assert config.max_iterations == 5
        assert config.temperature == 0.3

    def test_thinking_strategy_enum(self):
        """Test ThinkingStrategy enum values."""
        assert ThinkingStrategy.REACT.value == "react"
        assert ThinkingStrategy.SINGLE_SHOT.value == "single_shot"
        # COT is short for chain_of_thought
        assert ThinkingStrategy.CHAIN_OF_THOUGHT.value == "cot"


# ============================================================================
# Memory Integration Tests
# ============================================================================


class TestCognitiveMemory:
    """Tests for memory integration in cognitive agent."""

    @pytest.mark.asyncio
    async def test_memory_stores_conversation(self, cognitive_agent, mock_llm):
        """Test that conversation is stored in memory."""
        mock_llm.set_responses(["FINAL ANSWER: First response."])
        await cognitive_agent.run("First question")

        # Check memory has entries using get_context
        history = cognitive_agent.memory.get_context()
        assert len(history) >= 2  # At least user + assistant

        # User message should be first
        assert history[0]["role"] == "user"
        assert "First question" in history[0]["content"]

    @pytest.mark.asyncio
    async def test_memory_clear(self, cognitive_agent, mock_llm):
        """Test memory clear functionality."""
        mock_llm.set_responses(["FINAL ANSWER: Response."])
        await cognitive_agent.run("Question")

        assert len(cognitive_agent.memory.get_context()) > 0

        cognitive_agent.memory.clear()

        assert len(cognitive_agent.memory.get_context()) == 0

    @pytest.mark.asyncio
    async def test_context_added_to_memory(self, cognitive_agent, mock_llm):
        """Test that context is added to memory."""
        mock_llm.set_responses(["FINAL ANSWER: Done."])
        await cognitive_agent.run(
            "Question",
            context=["Context item 1", "Context item 2"],
        )

        history = cognitive_agent.memory.get_context()
        # Should have context entries
        context_entries = [h for h in history if "Context:" in h.get("content", "")]
        assert len(context_entries) >= 2
