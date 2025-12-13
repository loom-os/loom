"""Unit tests for StreamingClient and ChatClient."""

from unittest.mock import AsyncMock, MagicMock, patch

import pytest

from loom.streaming.chat_client import ChatClient, ChatMessage, ChatResponse
from loom.streaming.client import StreamingClient, StreamingError
from loom.streaming.types import (
    StreamComplete,
    StreamStatus,
)


class TestStreamingClient:
    """Tests for StreamingClient class."""

    def test_init_default(self):
        """Test default initialization."""
        client = StreamingClient()

        assert client.agent_id.startswith("stream-client-")
        assert client.backend_topic == "chat.input"
        assert client.bridge_addr is None
        assert not client.connected

    def test_init_custom(self):
        """Test custom initialization."""
        client = StreamingClient(
            agent_id="my-client",
            backend_topic="custom.topic",
            bridge_addr="localhost:9999",
        )

        assert client.agent_id == "my-client"
        assert client.backend_topic == "custom.topic"
        assert client.bridge_addr == "localhost:9999"

    def test_connected_property(self):
        """Test connected property."""
        client = StreamingClient()
        assert not client.connected

        # Simulate connection
        client._connected = True
        client._agent = MagicMock()
        assert client.connected

    @pytest.mark.asyncio
    async def test_connect(self):
        """Test connection to bridge."""
        with patch("loom.agent.Agent") as MockAgent:
            mock_agent = AsyncMock()
            MockAgent.return_value = mock_agent

            client = StreamingClient(agent_id="test-client")
            await client.connect()

            assert client.connected
            MockAgent.assert_called_once()
            mock_agent.start.assert_called_once()

    @pytest.mark.asyncio
    async def test_disconnect(self):
        """Test disconnection."""
        with patch("loom.agent.Agent") as MockAgent:
            mock_agent = AsyncMock()
            MockAgent.return_value = mock_agent

            client = StreamingClient()
            await client.connect()
            await client.disconnect()

            assert not client.connected
            mock_agent.stop.assert_called_once()

    @pytest.mark.asyncio
    async def test_stream_not_connected(self):
        """Test streaming without connection raises error."""
        client = StreamingClient()

        with pytest.raises(RuntimeError, match="not connected"):
            async for _ in client.stream("Hello"):
                pass

    @pytest.mark.asyncio
    async def test_send_message_not_connected(self):
        """Test send_message without connection raises error."""
        client = StreamingClient()

        with pytest.raises(RuntimeError, match="not connected"):
            await client.send_message("Hello")

    def test_get_stream_result_none(self):
        """Test getting non-existent stream result."""
        client = StreamingClient()
        result = client.get_stream_result("non-existent")
        assert result is None

    def test_get_stream_result_exists(self):
        """Test getting existing stream result."""
        client = StreamingClient()
        complete = StreamComplete(
            correlation_id="test-123",
            status=StreamStatus.OK,
        )
        client._stream_results["test-123"] = complete

        result = client.get_stream_result("test-123")
        assert result == complete


class TestChatClient:
    """Tests for ChatClient class."""

    def test_init_default(self):
        """Test default initialization."""
        client = ChatClient()

        assert client.client_id.startswith("chat-client-")
        assert not client.connected
        assert client.current_thread is None

    def test_init_custom(self):
        """Test custom initialization."""
        client = ChatClient(
            backend_topic="my.topic",
            bridge_addr="localhost:8080",
            client_id="my-client",
        )

        assert client.client_id == "my-client"
        assert client._streaming.backend_topic == "my.topic"

    def test_new_thread(self):
        """Test creating new thread."""
        client = ChatClient()

        thread1 = client.new_thread()
        thread2 = client.new_thread()

        assert thread1 != thread2
        assert client.current_thread == thread2
        assert thread1 in client._threads
        assert thread2 in client._threads

    def test_get_thread_history_empty(self):
        """Test getting history of empty thread."""
        client = ChatClient()
        thread_id = client.new_thread()

        history = client.get_thread_history(thread_id)
        assert history == []

    def test_get_thread_history_none(self):
        """Test getting history without thread."""
        client = ChatClient()
        history = client.get_thread_history()
        assert history == []

    def test_clear_thread(self):
        """Test clearing thread history."""
        client = ChatClient()
        thread_id = client.new_thread()

        # Add a message manually
        msg = ChatMessage(role="user", content="Hello", thread_id=thread_id)
        client._threads[thread_id].append(msg)

        assert len(client.get_thread_history(thread_id)) == 1

        client.clear_thread(thread_id)
        assert len(client.get_thread_history(thread_id)) == 0

    @pytest.mark.asyncio
    async def test_chat_not_connected(self):
        """Test chat without connection raises error."""
        client = ChatClient()

        with pytest.raises(RuntimeError, match="Not connected"):
            await client.chat("Hello")

    @pytest.mark.asyncio
    async def test_context_manager(self):
        """Test async context manager."""
        with patch.object(ChatClient, "connect", new_callable=AsyncMock) as mock_connect:
            with patch.object(ChatClient, "disconnect", new_callable=AsyncMock) as mock_disconnect:
                async with ChatClient():
                    mock_connect.assert_called_once()

                mock_disconnect.assert_called_once()


class TestChatMessage:
    """Tests for ChatMessage dataclass."""

    def test_creation(self):
        """Test basic creation."""
        msg = ChatMessage(
            role="user",
            content="Hello, AI!",
            thread_id="thread-123",
        )

        assert msg.role == "user"
        assert msg.content == "Hello, AI!"
        assert msg.thread_id == "thread-123"

    def test_defaults(self):
        """Test default values."""
        msg = ChatMessage(role="assistant", content="Hi!")

        assert msg.thread_id == ""
        assert msg.timestamp_ms == 0
        assert msg.metadata == {}

    def test_with_metadata(self):
        """Test message with metadata."""
        msg = ChatMessage(
            role="assistant",
            content="Result",
            metadata={"model": "gpt-4", "tokens": 50},
        )

        assert msg.metadata["model"] == "gpt-4"
        assert msg.metadata["tokens"] == 50


class TestChatResponse:
    """Tests for ChatResponse dataclass."""

    def test_creation(self):
        """Test basic creation."""
        response = ChatResponse(
            content="Hello!",
            thread_id="thread-123",
            status=StreamStatus.OK,
        )

        assert response.content == "Hello!"
        assert response.thread_id == "thread-123"
        assert response.status == StreamStatus.OK

    def test_defaults(self):
        """Test default values."""
        response = ChatResponse(content="Hi", thread_id="t1")

        assert response.stats is None
        assert response.tool_calls == []
        assert response.thinking == []
        assert response.status == StreamStatus.OK

    def test_with_extras(self):
        """Test response with thinking and tool calls."""
        response = ChatResponse(
            content="Final answer",
            thread_id="t1",
            thinking=["Step 1", "Step 2"],
            tool_calls=["search", "calculate"],
        )

        assert len(response.thinking) == 2
        assert len(response.tool_calls) == 2


class TestStreamingError:
    """Tests for StreamingError exception."""

    def test_creation(self):
        """Test error creation."""
        error = StreamingError("TEST_CODE", "Test message")

        assert error.code == "TEST_CODE"
        assert error.message == "Test message"
        assert str(error) == "TEST_CODE: Test message"

    def test_raise(self):
        """Test raising the error."""
        with pytest.raises(StreamingError) as exc_info:
            raise StreamingError("ERR_001", "Something failed")

        assert exc_info.value.code == "ERR_001"
        assert "Something failed" in str(exc_info.value)
