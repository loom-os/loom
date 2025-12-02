"""Unit tests for loom.Context."""

import asyncio
from unittest.mock import AsyncMock, Mock

import pytest

from loom import Context, Envelope
from loom.client import BridgeClient


@pytest.fixture
def mock_client() -> BridgeClient:
    """Create a mock BridgeClient."""
    client = Mock(spec=BridgeClient)
    client.forward_tool_call = AsyncMock()
    return client  # type: ignore[return-value]


@pytest.fixture
def context(mock_client: BridgeClient) -> Context:
    """Create a Context instance with mock client."""
    return Context(agent_id="test-agent", client=mock_client)


class TestContext:
    """Test Context class functionality."""

    @pytest.mark.asyncio
    async def test_emit(self, context: Context, mock_client: BridgeClient) -> None:
        """Test emitting an event."""
        # Bind context to a queue
        queue: asyncio.Queue = asyncio.Queue()
        context._bind(queue)

        await context.emit("test.topic", type="test.event", payload=b"test data")

        # Check that event was queued
        assert not queue.empty()
        client_event = await asyncio.wait_for(queue.get(), timeout=1.0)
        assert client_event.HasField("publish")
        assert client_event.publish.topic == "test.topic"

    @pytest.mark.asyncio
    async def test_emit_with_envelope(self, context: Context, mock_client: BridgeClient) -> None:
        """Test emitting with pre-built envelope."""
        queue: asyncio.Queue = asyncio.Queue()
        context._bind(queue)

        envelope = Envelope.new(
            type="custom.event",
            payload=b"custom data",
            thread_id="thread-1",
        )

        await context.emit("test.topic", type="override", payload=b"", envelope=envelope)

        assert not queue.empty()
        client_event = await asyncio.wait_for(queue.get(), timeout=1.0)
        assert client_event.HasField("publish")
        event = client_event.publish.event
        assert event.metadata.get("loom.thread_id") == "thread-1"

    @pytest.mark.asyncio
    async def test_reply(self, context: Context) -> None:
        """Test replying to an event."""
        queue: asyncio.Queue = asyncio.Queue()
        context._bind(queue)

        original = Envelope.new(
            type="request.type",
            payload=b"request",
            correlation_id="corr-123",
            reply_to="reply.topic",
        )

        await context.reply(original, type="response.type", payload=b"response")

        assert not queue.empty()
        client_event = await asyncio.wait_for(queue.get(), timeout=1.0)
        assert client_event.HasField("publish")
        assert client_event.publish.topic == "reply.topic"
        event = client_event.publish.event
        assert event.metadata.get("loom.correlation_id") == "corr-123"

    @pytest.mark.asyncio
    async def test_tool_invocation(self, context: Context, mock_client: BridgeClient) -> None:
        """Test tool invocation via forward_tool_call."""
        # Mock the forward_tool_call response
        from loom.proto.generated import action_pb2

        mock_result = action_pb2.ToolResult(
            status=action_pb2.ToolStatus.TOOL_OK,
            output='{"result": "success"}',
        )
        mock_client.forward_tool_call.return_value = mock_result  # type: ignore[attr-defined]

        result = await context.tool("test.tool", payload={"query": "test"})

        assert result == '{"result": "success"}'
        mock_client.forward_tool_call.assert_called_once()  # type: ignore[attr-defined]

    @pytest.mark.asyncio
    async def test_join_thread(self, context: Context) -> None:
        """Test joining a thread."""
        # join_thread is a placeholder in MVP
        await context.join_thread("thread-123")
        # Should not raise an error


@pytest.mark.asyncio
async def test_request_timeout() -> None:
    """Test request with timeout."""
    mock_client = Mock(spec=BridgeClient)
    context = Context(agent_id="test-agent", client=mock_client)
    queue: asyncio.Queue = asyncio.Queue()
    context._bind(queue)

    # Make request without providing a response - should timeout
    with pytest.raises(asyncio.TimeoutError):
        await context.request("test.topic", type="test.request", timeout_ms=100)
