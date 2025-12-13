"""Unit tests for streaming types."""

from unittest.mock import MagicMock

from loom.streaming.types import (
    StreamChunk,
    StreamComplete,
    StreamContentType,
    StreamError,
    StreamOptions,
    StreamRequest,
    StreamState,
    StreamStateKind,
    StreamStats,
    StreamStatus,
)


class TestStreamContentType:
    """Tests for StreamContentType enum."""

    def test_values(self):
        """Test enum values match expected integers."""
        assert StreamContentType.TEXT == 0
        assert StreamContentType.THINKING == 1
        assert StreamContentType.TOOL_CALL == 2
        assert StreamContentType.TOOL_RESULT == 3
        assert StreamContentType.ERROR == 4
        assert StreamContentType.STATUS == 5
        assert StreamContentType.MARKDOWN == 6
        assert StreamContentType.CODE == 7

    def test_from_int(self):
        """Test creating from integer."""
        assert StreamContentType(0) == StreamContentType.TEXT
        assert StreamContentType(2) == StreamContentType.TOOL_CALL


class TestStreamStatus:
    """Tests for StreamStatus enum."""

    def test_values(self):
        """Test enum values."""
        assert StreamStatus.OK == 0
        assert StreamStatus.CANCELLED == 1
        assert StreamStatus.TIMEOUT == 2
        assert StreamStatus.ERROR == 3
        assert StreamStatus.TRUNCATED == 4


class TestStreamStateKind:
    """Tests for StreamStateKind enum."""

    def test_values(self):
        """Test all state values."""
        assert StreamStateKind.STARTED == 0
        assert StreamStateKind.PROCESSING == 1
        assert StreamStateKind.GENERATING == 2
        assert StreamStateKind.TOOL_EXECUTING == 3
        assert StreamStateKind.WAITING == 4
        assert StreamStateKind.COMPLETED == 5
        assert StreamStateKind.FAILED == 6


class TestStreamError:
    """Tests for StreamError dataclass."""

    def test_creation(self):
        """Test basic creation."""
        error = StreamError(
            code="TEST_ERROR",
            message="Something went wrong",
            retryable=True,
            details={"key": "value"},
        )
        assert error.code == "TEST_ERROR"
        assert error.message == "Something went wrong"
        assert error.retryable is True
        assert error.details == {"key": "value"}

    def test_defaults(self):
        """Test default values."""
        error = StreamError(code="ERR", message="msg")
        assert error.retryable is False
        assert error.details == {}

    def test_to_proto(self):
        """Test conversion to protobuf."""
        error = StreamError(
            code="TEST",
            message="Test message",
            retryable=True,
            details={"a": "b"},
        )

        mock_proto = MagicMock()
        error.to_proto(mock_proto)

        mock_proto.assert_called_once_with(
            code="TEST",
            message="Test message",
            retryable=True,
            details={"a": "b"},
        )


class TestStreamStats:
    """Tests for StreamStats dataclass."""

    def test_creation(self):
        """Test basic creation."""
        stats = StreamStats(
            total_tokens=100,
            duration_ms=5000,
            tool_calls=3,
            iterations=2,
            first_chunk_latency_ms=150,
        )
        assert stats.total_tokens == 100
        assert stats.duration_ms == 5000
        assert stats.tool_calls == 3
        assert stats.iterations == 2
        assert stats.first_chunk_latency_ms == 150

    def test_defaults(self):
        """Test default values."""
        stats = StreamStats()
        assert stats.total_tokens == 0
        assert stats.duration_ms == 0
        assert stats.tool_calls == 0
        assert stats.iterations == 0
        assert stats.first_chunk_latency_ms == 0


class TestStreamChunk:
    """Tests for StreamChunk dataclass."""

    def test_creation(self):
        """Test basic creation."""
        chunk = StreamChunk(
            correlation_id="test-123",
            content="Hello, world!",
            sequence=0,
            content_type=StreamContentType.TEXT,
        )
        assert chunk.correlation_id == "test-123"
        assert chunk.content == "Hello, world!"
        assert chunk.sequence == 0
        assert chunk.content_type == StreamContentType.TEXT

    def test_defaults(self):
        """Test default values."""
        chunk = StreamChunk(correlation_id="test", content="hi")
        assert chunk.sequence == 0
        assert chunk.content_type == StreamContentType.TEXT
        assert chunk.metadata == {}
        assert chunk.timestamp_ms > 0

    def test_different_content_types(self):
        """Test chunks with different content types."""
        thinking = StreamChunk(
            correlation_id="t1",
            content="Analyzing...",
            content_type=StreamContentType.THINKING,
        )
        assert thinking.content_type == StreamContentType.THINKING

        tool_call = StreamChunk(
            correlation_id="t1",
            content='{"name": "search"}',
            content_type=StreamContentType.TOOL_CALL,
        )
        assert tool_call.content_type == StreamContentType.TOOL_CALL

    def test_metadata(self):
        """Test chunk with metadata."""
        chunk = StreamChunk(
            correlation_id="t1",
            content="code",
            content_type=StreamContentType.CODE,
            metadata={"language": "python", "filename": "test.py"},
        )
        assert chunk.metadata["language"] == "python"
        assert chunk.metadata["filename"] == "test.py"


class TestStreamComplete:
    """Tests for StreamComplete dataclass."""

    def test_creation_success(self):
        """Test successful completion."""
        complete = StreamComplete(
            correlation_id="test-123",
            final_content="Complete response",
            total_chunks=5,
            status=StreamStatus.OK,
            stats=StreamStats(duration_ms=1000, total_tokens=50),
        )
        assert complete.correlation_id == "test-123"
        assert complete.final_content == "Complete response"
        assert complete.total_chunks == 5
        assert complete.status == StreamStatus.OK
        assert complete.error is None
        assert complete.stats.duration_ms == 1000

    def test_creation_error(self):
        """Test error completion."""
        complete = StreamComplete(
            correlation_id="test-123",
            status=StreamStatus.ERROR,
            error=StreamError(code="LLM_ERROR", message="Rate limited"),
        )
        assert complete.status == StreamStatus.ERROR
        assert complete.error.code == "LLM_ERROR"

    def test_defaults(self):
        """Test default values."""
        complete = StreamComplete(correlation_id="test")
        assert complete.final_content == ""
        assert complete.total_chunks == 0
        assert complete.status == StreamStatus.OK
        assert complete.error is None
        assert complete.stats is None


class TestStreamOptions:
    """Tests for StreamOptions dataclass."""

    def test_creation(self):
        """Test basic creation."""
        options = StreamOptions(
            max_tokens=1000,
            include_thinking=True,
            include_tool_calls=True,
            timeout_ms=30000,
        )
        assert options.max_tokens == 1000
        assert options.include_thinking is True
        assert options.include_tool_calls is True
        assert options.timeout_ms == 30000

    def test_defaults(self):
        """Test default values."""
        options = StreamOptions()
        assert options.max_tokens == 0
        assert options.include_thinking is False
        assert options.include_tool_calls is True
        assert options.timeout_ms == 0
        assert options.content_types == []

    def test_content_type_filtering(self):
        """Test content type filtering."""
        options = StreamOptions(
            content_types=[
                StreamContentType.TEXT,
                StreamContentType.THINKING,
            ]
        )
        assert len(options.content_types) == 2
        assert StreamContentType.TEXT in options.content_types


class TestStreamRequest:
    """Tests for StreamRequest dataclass."""

    def test_creation(self):
        """Test basic creation."""
        request = StreamRequest(
            message="Hello AI",
            thread_id="thread-123",
            sender="user-456",
            reply_to="agent.user-456.replies",
        )
        assert request.message == "Hello AI"
        assert request.thread_id == "thread-123"
        assert request.sender == "user-456"
        assert len(request.id) > 0  # Auto-generated

    def test_auto_id(self):
        """Test auto-generated ID."""
        r1 = StreamRequest(message="test1")
        r2 = StreamRequest(message="test2")
        assert r1.id != r2.id

    def test_with_options(self):
        """Test request with options."""
        request = StreamRequest(
            message="Explain this",
            options=StreamOptions(include_thinking=True),
        )
        assert request.options.include_thinking is True

    def test_with_context(self):
        """Test request with context."""
        request = StreamRequest(
            message="Continue",
            context=["Previous message 1", "Previous message 2"],
        )
        assert len(request.context) == 2


class TestStreamState:
    """Tests for StreamState dataclass."""

    def test_creation(self):
        """Test basic creation."""
        state = StreamState(
            correlation_id="test-123",
            state=StreamStateKind.GENERATING,
            message="Processing your request...",
        )
        assert state.correlation_id == "test-123"
        assert state.state == StreamStateKind.GENERATING
        assert state.message == "Processing your request..."

    def test_defaults(self):
        """Test default values."""
        state = StreamState(
            correlation_id="test",
            state=StreamStateKind.STARTED,
        )
        assert state.message == ""
        assert state.timestamp_ms > 0

    def test_state_progression(self):
        """Test typical state progression."""
        states = [
            StreamStateKind.STARTED,
            StreamStateKind.PROCESSING,
            StreamStateKind.GENERATING,
            StreamStateKind.COMPLETED,
        ]
        for i, s in enumerate(states):
            state = StreamState(correlation_id="test", state=s)
            assert state.state.value == i if i < 3 else state.state.value == 5
