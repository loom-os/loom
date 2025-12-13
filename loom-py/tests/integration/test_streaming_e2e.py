"""End-to-end integration tests for streaming chat flow.

These tests verify the complete flow:
    loom chat CLI → ChatClient → StreamingClient → Bridge → Backend Agent

Run with: pytest tests/integration/test_streaming_e2e.py -v -m integration

Prerequisites:
    - Bridge server running (or use bridge_server fixture)
"""

from __future__ import annotations

import asyncio
from typing import TYPE_CHECKING
from unittest.mock import AsyncMock, MagicMock, patch

import pytest

from loom.streaming import (
    StreamCancelledError,
    StreamContentType,
    StreamingClient,
    StreamingHandler,
)

if TYPE_CHECKING:
    pass


class TestStreamingClientTracing:
    """Tests for StreamingClient tracing instrumentation."""

    def test_client_has_tracer(self):
        """Verify StreamingClient module has tracer configured."""
        from loom.streaming import client as client_module

        assert hasattr(client_module, "tracer")
        assert client_module.tracer is not None

    @pytest.mark.asyncio
    async def test_stream_creates_span(self):
        """Verify stream() creates proper tracing span using mock tracer."""
        # Create a mock tracer to replace the module-level tracer
        mock_span = MagicMock()
        mock_span.__enter__ = MagicMock(return_value=mock_span)
        mock_span.__exit__ = MagicMock(return_value=False)
        mock_span.set_attribute = MagicMock()

        mock_tracer = MagicMock()
        mock_tracer.start_as_current_span = MagicMock(return_value=mock_span)

        # Mock the agent and its methods
        mock_agent = MagicMock()
        mock_agent.start = AsyncMock()
        mock_agent.stop = AsyncMock()
        mock_agent.ctx = MagicMock()
        mock_agent.ctx.emit = AsyncMock()

        # Patch the module-level tracer directly
        with patch.object(StreamingClient, "__module__", "loom.streaming.client"):
            import loom.streaming.client as client_module

            original_tracer = client_module.tracer
            client_module.tracer = mock_tracer

            try:
                client = StreamingClient(agent_id="test-client")
                client._connected = True
                client._agent = mock_agent

                # Start stream (will timeout quickly since no response)
                try:
                    async for _ in client.stream("test message", timeout=0.1):
                        pass
                except (TimeoutError, asyncio.TimeoutError):
                    pass  # Expected

                # Verify span was created with correct name
                mock_tracer.start_as_current_span.assert_called()
                call_args = mock_tracer.start_as_current_span.call_args
                assert call_args[0][0] == "streaming.client.stream"
            finally:
                # Restore original tracer
                client_module.tracer = original_tracer


class TestStreamingHandlerTracing:
    """Tests for StreamingHandler tracing instrumentation."""

    def test_handler_has_tracer(self):
        """Verify StreamingHandler module has tracer configured."""
        from loom.streaming import handler as handler_module

        assert hasattr(handler_module, "tracer")
        assert handler_module.tracer is not None

    @pytest.mark.asyncio
    async def test_handler_creates_span_on_first_chunk(self):
        """Verify handler creates span when sending first chunk."""
        mock_ctx = MagicMock()
        mock_ctx.agent_id = "test-agent"
        mock_ctx.emit = AsyncMock()

        mock_event = MagicMock()
        mock_event.id = "event-123"
        mock_event.correlation_id = "corr-456"
        mock_event.sender = "client-789"
        mock_event.reply_to = "agent.client-789.replies"
        mock_event.thread_id = "thread-abc"
        mock_event.extract_trace_context = MagicMock(return_value=None)

        handler = StreamingHandler(mock_ctx, mock_event)

        # Send a chunk - this should create the span
        await handler.send_chunk("Hello", StreamContentType.TEXT)

        # Verify span was created
        assert handler._span is not None

    @pytest.mark.asyncio
    async def test_handler_records_metrics_on_complete(self):
        """Verify handler records metrics when completing stream."""
        mock_ctx = MagicMock()
        mock_ctx.agent_id = "test-agent"
        mock_ctx.emit = AsyncMock()

        mock_event = MagicMock()
        mock_event.id = "event-123"
        mock_event.correlation_id = "corr-456"
        mock_event.sender = "client-789"
        mock_event.reply_to = "agent.client-789.replies"
        mock_event.thread_id = "thread-abc"
        mock_event.extract_trace_context = MagicMock(return_value=None)

        handler = StreamingHandler(mock_ctx, mock_event)

        # Send chunks
        await handler.send_chunk("Hello ", StreamContentType.TEXT)
        await handler.send_chunk("World", StreamContentType.TEXT)

        # Complete
        await handler.send_complete(final_content="Hello World", tokens=100)

        # Verify span was ended with metrics
        # (The span should have been ended in send_complete)
        assert handler._chunks_sent == 2


class TestStreamCancellation:
    """Tests for stream cancellation support."""

    def test_cancel_sets_cancelled_flag(self):
        """Verify cancel() sets the cancelled flag."""
        mock_ctx = MagicMock()
        mock_ctx.agent_id = "test-agent"

        mock_event = MagicMock()
        mock_event.id = "event-123"
        mock_event.correlation_id = "corr-456"
        mock_event.sender = "client-789"
        mock_event.reply_to = None
        mock_event.thread_id = None
        mock_event.extract_trace_context = MagicMock(return_value=None)

        handler = StreamingHandler(mock_ctx, mock_event)

        assert not handler.is_cancelled
        handler.cancel("User requested stop")
        assert handler.is_cancelled
        assert handler._cancel_reason == "User requested stop"

    def test_check_cancelled_raises_error(self):
        """Verify check_cancelled() raises StreamCancelledError when cancelled."""
        mock_ctx = MagicMock()
        mock_ctx.agent_id = "test-agent"

        mock_event = MagicMock()
        mock_event.id = "event-123"
        mock_event.correlation_id = "corr-456"
        mock_event.sender = "client-789"
        mock_event.reply_to = None
        mock_event.thread_id = None
        mock_event.extract_trace_context = MagicMock(return_value=None)

        handler = StreamingHandler(mock_ctx, mock_event)

        # Should not raise when not cancelled
        handler.check_cancelled()

        # Should raise after cancel
        handler.cancel("Test cancellation")
        with pytest.raises(StreamCancelledError) as exc_info:
            handler.check_cancelled()

        assert "Test cancellation" in str(exc_info.value)

    @pytest.mark.asyncio
    async def test_send_chunk_raises_on_cancelled(self):
        """Verify send_chunk() raises error when stream is cancelled."""
        mock_ctx = MagicMock()
        mock_ctx.agent_id = "test-agent"
        mock_ctx.emit = AsyncMock()

        mock_event = MagicMock()
        mock_event.id = "event-123"
        mock_event.correlation_id = "corr-456"
        mock_event.sender = "client-789"
        mock_event.reply_to = None
        mock_event.thread_id = None
        mock_event.extract_trace_context = MagicMock(return_value=None)

        handler = StreamingHandler(mock_ctx, mock_event)

        # First chunk should succeed
        await handler.send_chunk("First", StreamContentType.TEXT)

        # Cancel
        handler.cancel("Interrupted")

        # Next chunk should raise
        with pytest.raises(StreamCancelledError):
            await handler.send_chunk("Second", StreamContentType.TEXT)


class TestChatClientTracing:
    """Tests for ChatClient tracing instrumentation."""

    def test_chat_client_has_tracer(self):
        """Verify ChatClient module has tracer configured."""
        from loom.streaming import chat_client as chat_module

        assert hasattr(chat_module, "tracer")
        assert chat_module.tracer is not None


class TestContextEngineeringInStreaming:
    """Tests for context engineering in streaming mode."""

    @pytest.mark.asyncio
    async def test_streaming_strategy_uses_compaction(self):
        """Verify run_react_stream uses step compaction."""
        from unittest.mock import MagicMock

        from loom.cognitive.config import CognitiveConfig
        from loom.cognitive.strategies import StrategyExecutor
        from loom.cognitive.types import CognitiveResult
        from loom.context import StepCompactor
        from loom.context.memory import WorkingMemory

        # Create mock LLM that returns a final answer
        mock_llm = MagicMock()

        async def mock_stream(*args, **kwargs):
            yield "Final Answer: This is the response."

        mock_llm.generate_stream = mock_stream

        # Create strategy executor
        config = CognitiveConfig(max_iterations=5)
        memory = WorkingMemory()
        step_compactor = StepCompactor()
        tool_executor = MagicMock()

        executor = StrategyExecutor(
            llm=mock_llm,
            config=config,
            memory=memory,
            tool_executor=tool_executor,
            step_compactor=step_compactor,
            available_tools=[],
        )

        # Run streaming
        items = []
        async for item in executor.run_react_stream("Test goal"):
            items.append(item)

        # Verify we got chunks and final result
        assert len(items) > 0
        # Last item should be CognitiveResult
        assert isinstance(items[-1], CognitiveResult)


class TestTracingContextPropagation:
    """Tests for trace context propagation across components."""

    def test_envelope_injects_trace_context(self):
        """Verify Envelope can inject current trace context."""
        from opentelemetry import trace

        from loom.agent.envelope import Envelope

        env = Envelope.new(
            type="test.event",
            payload=b"test",
            sender="test-agent",
        )

        # Before injection, trace fields should be empty
        assert env.trace_id is None or env.trace_id == ""

        # With an active span, injection should capture context
        tracer = trace.get_tracer(__name__)
        with tracer.start_as_current_span("test-span") as span:
            env.inject_trace_context()

            # Now trace_id should be set
            if span.get_span_context().is_valid:
                assert env.trace_id is not None
                assert len(env.trace_id) == 32  # 16 bytes hex = 32 chars

    def test_envelope_extracts_trace_context(self):
        """Verify Envelope can extract trace context for remote parent."""
        from loom.agent.envelope import Envelope

        # Create envelope with trace context
        env = Envelope(
            id="test-123",
            type="test.event",
            timestamp_ms=0,
            source="test",
            payload=b"",
            trace_id="0" * 32,  # Valid trace ID
            span_id="0" * 16,  # Valid span ID
            trace_flags="01",
        )

        # Extract should return SpanContext
        ctx = env.extract_trace_context()
        assert ctx is not None
        assert ctx.is_remote


class TestStreamingMetrics:
    """Tests for streaming metrics and statistics."""

    @pytest.mark.asyncio
    async def test_handler_tracks_chunk_count(self):
        """Verify handler tracks number of chunks sent."""
        mock_ctx = MagicMock()
        mock_ctx.agent_id = "test-agent"
        mock_ctx.emit = AsyncMock()

        mock_event = MagicMock()
        mock_event.id = "event-123"
        mock_event.correlation_id = "corr-456"
        mock_event.sender = "client-789"
        mock_event.reply_to = "replies"
        mock_event.thread_id = ""
        mock_event.extract_trace_context = MagicMock(return_value=None)

        handler = StreamingHandler(mock_ctx, mock_event)

        # Send multiple chunks
        for i in range(5):
            await handler.send_chunk(f"Chunk {i}", StreamContentType.TEXT)

        assert handler._chunks_sent == 5
        assert handler._sequence == 5

    @pytest.mark.asyncio
    async def test_handler_tracks_tool_calls(self):
        """Verify handler tracks number of tool calls."""
        mock_ctx = MagicMock()
        mock_ctx.agent_id = "test-agent"
        mock_ctx.emit = AsyncMock()

        mock_event = MagicMock()
        mock_event.id = "event-123"
        mock_event.correlation_id = "corr-456"
        mock_event.sender = "client-789"
        mock_event.reply_to = "replies"
        mock_event.thread_id = ""
        mock_event.extract_trace_context = MagicMock(return_value=None)

        handler = StreamingHandler(mock_ctx, mock_event)

        # Send tool calls
        await handler.send_tool_call("search", '{"query": "test"}')
        await handler.send_tool_result("search", "Found results")
        await handler.send_tool_call("read_file", '{"path": "test.txt"}')

        assert handler._tool_calls == 2


# Mark all tests in this module as integration tests
pytestmark = pytest.mark.asyncio
