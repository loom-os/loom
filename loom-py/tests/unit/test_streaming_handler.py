"""Unit tests for StreamingHandler."""

from unittest.mock import AsyncMock, MagicMock

import pytest

from loom.streaming.handler import StreamCancelledError, StreamingHandler
from loom.streaming.types import (
    StreamContentType,
    StreamStateKind,
)


@pytest.fixture
def mock_ctx():
    """Create a mock EventContext."""
    ctx = MagicMock()
    ctx.agent_id = "test-agent"
    ctx.emit = AsyncMock()
    return ctx


@pytest.fixture
def mock_event():
    """Create a mock Envelope event."""
    event = MagicMock()
    event.id = "event-123"
    event.correlation_id = "corr-456"
    event.sender = "client-789"
    event.reply_to = "agent.client-789.replies"
    event.thread_id = "thread-abc"
    # Return None for trace context to avoid SpanContext type errors
    event.extract_trace_context.return_value = None
    return event


class TestStreamingHandler:
    """Tests for StreamingHandler class."""

    def test_init(self, mock_ctx, mock_event):
        """Test handler initialization."""
        handler = StreamingHandler(mock_ctx, mock_event)

        assert handler.ctx == mock_ctx
        assert handler.correlation_id == "corr-456"
        assert handler.reply_to == "agent.client-789.replies"
        assert handler.thread_id == "thread-abc"
        assert handler._sequence == 0
        assert handler._chunks_sent == 0

    def test_init_without_correlation_id(self, mock_ctx):
        """Test handler with event without correlation_id."""
        event = MagicMock()
        event.id = "event-123"
        event.correlation_id = None
        event.sender = "client-789"
        event.reply_to = None
        event.thread_id = None

        handler = StreamingHandler(mock_ctx, event)

        assert handler.correlation_id == "event-123"
        assert handler.reply_to == "agent.client-789.replies"

    @pytest.mark.asyncio
    async def test_send_chunk_basic(self, mock_ctx, mock_event):
        """Test sending a basic text chunk."""
        handler = StreamingHandler(mock_ctx, mock_event)

        await handler.send_chunk("Hello, world!")

        assert handler._sequence == 1
        assert handler._chunks_sent == 1
        assert handler._total_content == ["Hello, world!"]
        mock_ctx.emit.assert_called_once()

    @pytest.mark.asyncio
    async def test_send_chunk_sequence(self, mock_ctx, mock_event):
        """Test chunk sequencing."""
        handler = StreamingHandler(mock_ctx, mock_event)

        await handler.send_chunk("First")
        await handler.send_chunk("Second")
        await handler.send_chunk("Third")

        assert handler._sequence == 3
        assert handler._chunks_sent == 3
        assert handler._total_content == ["First", "Second", "Third"]

    @pytest.mark.asyncio
    async def test_send_chunk_with_type(self, mock_ctx, mock_event):
        """Test sending chunks with different content types."""
        handler = StreamingHandler(mock_ctx, mock_event)

        await handler.send_chunk("Analyzing...", StreamContentType.THINKING)
        await handler.send_chunk("Result", StreamContentType.TEXT)

        assert handler._chunks_sent == 2
        # Only TEXT type is added to _total_content
        assert handler._total_content == ["Result"]

    @pytest.mark.asyncio
    async def test_send_thinking(self, mock_ctx, mock_event):
        """Test send_thinking helper."""
        handler = StreamingHandler(mock_ctx, mock_event)

        await handler.send_thinking("Let me think...")

        assert handler._chunks_sent == 1
        # Thinking content not in total_content
        assert handler._total_content == []

    @pytest.mark.asyncio
    async def test_send_tool_call(self, mock_ctx, mock_event):
        """Test send_tool_call helper."""
        handler = StreamingHandler(mock_ctx, mock_event)

        await handler.send_tool_call("search", '{"query": "test"}')

        assert handler._chunks_sent == 1
        assert handler._tool_calls == 1

    @pytest.mark.asyncio
    async def test_send_tool_result(self, mock_ctx, mock_event):
        """Test send_tool_result helper."""
        handler = StreamingHandler(mock_ctx, mock_event)

        await handler.send_tool_result("search", "Found 10 results")

        assert handler._chunks_sent == 1

    @pytest.mark.asyncio
    async def test_send_status(self, mock_ctx, mock_event):
        """Test send_status helper."""
        handler = StreamingHandler(mock_ctx, mock_event)

        await handler.send_status("Processing...")

        assert handler._chunks_sent == 1

    @pytest.mark.asyncio
    async def test_send_state_disabled(self, mock_ctx, mock_event):
        """Test that state updates are disabled by default."""
        handler = StreamingHandler(mock_ctx, mock_event, include_state_updates=False)

        await handler.send_state(StreamStateKind.GENERATING, "Generating...")

        # Should not emit anything
        mock_ctx.emit.assert_not_called()

    @pytest.mark.asyncio
    async def test_send_state_enabled(self, mock_ctx, mock_event):
        """Test state updates when enabled."""
        handler = StreamingHandler(mock_ctx, mock_event, include_state_updates=True)

        await handler.send_state(StreamStateKind.GENERATING, "Generating...")

        mock_ctx.emit.assert_called_once()

    @pytest.mark.asyncio
    async def test_send_complete(self, mock_ctx, mock_event):
        """Test sending completion."""
        handler = StreamingHandler(mock_ctx, mock_event)

        await handler.send_chunk("Hello")
        await handler.send_chunk(" World")
        await handler.send_complete()

        # Should have 3 emit calls: 2 chunks + 1 complete
        assert mock_ctx.emit.call_count == 3

    @pytest.mark.asyncio
    async def test_send_complete_with_custom_content(self, mock_ctx, mock_event):
        """Test completion with custom final content."""
        handler = StreamingHandler(mock_ctx, mock_event)

        await handler.send_chunk("Partial")
        await handler.send_complete(final_content="Full response here")

        assert mock_ctx.emit.call_count == 2

    @pytest.mark.asyncio
    async def test_send_complete_with_stats(self, mock_ctx, mock_event):
        """Test completion with statistics."""
        handler = StreamingHandler(mock_ctx, mock_event)

        await handler.send_chunk("Test")
        await handler.send_tool_call("tool1", "{}")
        await handler.send_complete(tokens=100, iterations=3)

        assert mock_ctx.emit.call_count == 3

    @pytest.mark.asyncio
    async def test_send_error(self, mock_ctx, mock_event):
        """Test sending error completion."""
        handler = StreamingHandler(mock_ctx, mock_event)

        await handler.send_error("Something went wrong", code="TEST_ERROR")

        mock_ctx.emit.assert_called_once()
        call_args = mock_ctx.emit.call_args
        assert call_args[1]["type"] == "stream.complete"

    @pytest.mark.asyncio
    async def test_send_error_retryable(self, mock_ctx, mock_event):
        """Test sending retryable error."""
        handler = StreamingHandler(mock_ctx, mock_event)

        await handler.send_error(
            "Rate limited",
            code="RATE_LIMIT",
            retryable=True,
        )

        mock_ctx.emit.assert_called_once()

    @pytest.mark.asyncio
    async def test_send_cancelled(self, mock_ctx, mock_event):
        """Test sending cancellation."""
        handler = StreamingHandler(mock_ctx, mock_event)

        await handler.send_cancelled("User cancelled")

        mock_ctx.emit.assert_called_once()
        call_args = mock_ctx.emit.call_args
        assert call_args[1]["type"] == "stream.complete"

    @pytest.mark.asyncio
    async def test_first_chunk_latency_tracking(self, mock_ctx, mock_event):
        """Test that first chunk latency is tracked."""
        handler = StreamingHandler(mock_ctx, mock_event)

        assert handler._first_chunk_time is None

        await handler.send_chunk("First")
        first_time = handler._first_chunk_time

        assert first_time is not None

        await handler.send_chunk("Second")
        # Should not change after first chunk
        assert handler._first_chunk_time == first_time

    @pytest.mark.asyncio
    async def test_tool_call_counting(self, mock_ctx, mock_event):
        """Test tool call counting."""
        handler = StreamingHandler(mock_ctx, mock_event)

        await handler.send_tool_call("tool1", "{}")
        await handler.send_tool_call("tool2", "{}")
        await handler.send_chunk("Result", StreamContentType.TOOL_CALL)

        assert handler._tool_calls == 3

    @pytest.mark.asyncio
    async def test_chunk_metadata_in_emit(self, mock_ctx, mock_event):
        """Test that chunk metadata is properly included."""
        handler = StreamingHandler(mock_ctx, mock_event)

        await handler.send_chunk(
            "Code here",
            content_type=StreamContentType.CODE,
            metadata={"language": "python"},
        )

        call_args = mock_ctx.emit.call_args
        # Check that envelope was passed
        assert "envelope" in call_args[1]


class TestStreamingHandlerCancellation:
    """Tests for cancellation support in StreamingHandler."""

    def test_is_cancelled_default(self, mock_ctx, mock_event):
        """Test that is_cancelled is False by default."""
        handler = StreamingHandler(mock_ctx, mock_event)
        assert not handler.is_cancelled

    def test_cancel_sets_flag(self, mock_ctx, mock_event):
        """Test that cancel() sets the cancelled flag."""
        handler = StreamingHandler(mock_ctx, mock_event)

        handler.cancel("User requested")
        assert handler.is_cancelled
        assert handler._cancel_reason == "User requested"

    def test_check_cancelled_does_not_raise_when_not_cancelled(self, mock_ctx, mock_event):
        """Test check_cancelled() doesn't raise when not cancelled."""
        handler = StreamingHandler(mock_ctx, mock_event)
        # Should not raise
        handler.check_cancelled()

    def test_check_cancelled_raises_when_cancelled(self, mock_ctx, mock_event):
        """Test check_cancelled() raises StreamCancelledError when cancelled."""
        handler = StreamingHandler(mock_ctx, mock_event)

        handler.cancel("Stop now")

        with pytest.raises(StreamCancelledError) as exc_info:
            handler.check_cancelled()

        assert "Stop now" in str(exc_info.value)
        assert exc_info.value.reason == "Stop now"

    @pytest.mark.asyncio
    async def test_send_chunk_raises_when_cancelled(self, mock_ctx, mock_event):
        """Test send_chunk() raises when stream is cancelled."""
        handler = StreamingHandler(mock_ctx, mock_event)

        # First chunk succeeds
        await handler.send_chunk("Hello")

        # Cancel
        handler.cancel("Cancelled")

        # Next chunk should raise
        with pytest.raises(StreamCancelledError):
            await handler.send_chunk("World")

    @pytest.mark.asyncio
    async def test_send_cancelled_sends_completion(self, mock_ctx, mock_event):
        """Test send_cancelled() sends proper completion."""
        handler = StreamingHandler(mock_ctx, mock_event)

        await handler.send_cancelled("User interrupted")

        mock_ctx.emit.assert_called_once()
        call_args = mock_ctx.emit.call_args
        assert call_args[1]["type"] == "stream.complete"


class TestStreamingHandlerTracing:
    """Tests for tracing in StreamingHandler."""

    def test_span_not_created_initially(self, mock_ctx, mock_event):
        """Test that span is not created until first chunk."""
        handler = StreamingHandler(mock_ctx, mock_event)
        assert handler._span is None

    @pytest.mark.asyncio
    async def test_span_created_on_first_chunk(self, mock_ctx, mock_event):
        """Test that span is created on first send_chunk."""
        handler = StreamingHandler(mock_ctx, mock_event)

        await handler.send_chunk("Hello")

        assert handler._span is not None

    @pytest.mark.asyncio
    async def test_span_reused_for_subsequent_chunks(self, mock_ctx, mock_event):
        """Test that same span is reused for all chunks."""
        handler = StreamingHandler(mock_ctx, mock_event)

        await handler.send_chunk("First")
        first_span = handler._span

        await handler.send_chunk("Second")
        second_span = handler._span

        assert first_span is second_span

    def test_parent_context_extracted(self, mock_ctx):
        """Test that parent trace context is extracted from event."""
        mock_event = MagicMock()
        mock_event.id = "event-123"
        mock_event.correlation_id = "corr-456"
        mock_event.sender = "client-789"
        mock_event.reply_to = None
        mock_event.thread_id = None

        # Mock a valid trace context
        from opentelemetry.trace import SpanContext, TraceFlags, TraceState

        mock_context = SpanContext(
            trace_id=0x123456789ABCDEF0123456789ABCDEF0,
            span_id=0x123456789ABCDEF0,
            is_remote=True,
            trace_flags=TraceFlags(1),
            trace_state=TraceState(),
        )
        mock_event.extract_trace_context = MagicMock(return_value=mock_context)

        handler = StreamingHandler(mock_ctx, mock_event)

        assert handler._parent_context is not None
        assert handler._parent_context.trace_id == mock_context.trace_id
