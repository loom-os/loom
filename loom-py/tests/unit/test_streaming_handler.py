"""Unit tests for StreamingHandler."""

from unittest.mock import AsyncMock, MagicMock

import pytest

from loom.streaming.handler import StreamingHandler
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
