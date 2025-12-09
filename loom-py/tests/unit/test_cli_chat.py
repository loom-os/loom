"""Unit tests for loom.cli.chat module."""

from __future__ import annotations

from unittest.mock import AsyncMock, MagicMock, patch

import pytest

from loom.cli.chat import ChatSession


class TestChatSession:
    """Tests for ChatSession class."""

    def test_init_default(self):
        """Should initialize with default values."""
        session = ChatSession()
        assert session.backend_agent_id == "chat-assistant"
        assert session.backend_topic == "chat.input"
        assert session.bridge_addr is None
        assert session.verbose is True
        assert session.streaming is True
        assert session._client is None
        assert session._connected is False

    def test_init_custom(self):
        """Should initialize with custom values."""
        session = ChatSession(
            backend_agent_id="custom-agent",
            backend_topic="custom.topic",
            bridge_addr="localhost:9000",
            verbose=False,
            streaming=False,
        )
        assert session.backend_agent_id == "custom-agent"
        assert session.backend_topic == "custom.topic"
        assert session.bridge_addr == "localhost:9000"
        assert session.verbose is False
        assert session.streaming is False

    def test_connected_property_false(self):
        """connected should be False when not connected."""
        session = ChatSession()
        assert session.connected is False

    def test_connected_property_true(self):
        """connected should be True when connected."""
        session = ChatSession()
        session._connected = True
        session._client = MagicMock()
        assert session.connected is True

    @pytest.mark.asyncio
    async def test_connect_success(self):
        """connect should create ChatClient and connect."""
        session = ChatSession()

        mock_client_class = MagicMock()
        mock_client = AsyncMock()
        mock_client.connect = AsyncMock()
        mock_client_class.return_value = mock_client

        with patch("loom.cli.chat.ChatSession.connect") as mock_connect:
            mock_connect.return_value = True
            result = await session.connect()
            assert result is True

    @pytest.mark.asyncio
    async def test_connect_failure(self):
        """connect should return False on failure."""
        session = ChatSession()

        mock_client_class = MagicMock()
        mock_client_class.side_effect = Exception("Connection failed")

        with patch("loom.streaming.ChatClient", mock_client_class):
            result = await session.connect()
            assert result is False
            assert session._last_error == "Connection failed"

    @pytest.mark.asyncio
    async def test_disconnect(self):
        """disconnect should cleanup client."""
        session = ChatSession()
        session._client = AsyncMock()
        session._client.disconnect = AsyncMock()
        session._connected = True

        await session.disconnect()

        session._client.disconnect.assert_called_once()
        assert session._connected is False

    @pytest.mark.asyncio
    async def test_disconnect_no_client(self):
        """disconnect should handle no client gracefully."""
        session = ChatSession()
        session._client = None
        session._connected = False

        # Should not raise
        await session.disconnect()
        assert session._connected is False

    @pytest.mark.asyncio
    async def test_chat_not_connected(self):
        """chat should raise error when not connected."""
        session = ChatSession()
        session._client = None

        with pytest.raises(RuntimeError, match="Not connected"):
            await session.chat("Hello")

    def test_get_history_empty(self):
        """get_history should return empty list when no client."""
        session = ChatSession()
        assert session.get_history() == []

    def test_get_history_with_client(self):
        """get_history should return messages from client."""
        session = ChatSession()

        mock_msg1 = MagicMock()
        mock_msg1.role = "user"
        mock_msg1.content = "Hello"

        mock_msg2 = MagicMock()
        mock_msg2.role = "assistant"
        mock_msg2.content = "Hi!"

        session._client = MagicMock()
        session._client.get_thread_history.return_value = [mock_msg1, mock_msg2]

        history = session.get_history()
        assert len(history) == 2
        assert history[0]["role"] == "user"
        assert history[0]["content"] == "Hello"
        assert history[1]["role"] == "assistant"
        assert history[1]["content"] == "Hi!"

    def test_clear_history(self):
        """clear_history should call client method."""
        session = ChatSession()
        session._client = MagicMock()
        session._client.clear_thread = MagicMock()

        session.clear_history()

        session._client.clear_thread.assert_called_once()

    def test_clear_history_no_client(self):
        """clear_history should handle no client gracefully."""
        session = ChatSession()
        session._client = None

        # Should not raise
        session.clear_history()


class TestPrintStreamStepComplete:
    """Tests for print_stream_step_complete function."""

    def test_handles_step_without_tool_call(self, capsys):
        """Should handle step without tool call."""
        from loom.cli.chat import print_stream_step_complete

        step = MagicMock()
        step.tool_call = None

        print_stream_step_complete(step)

        # Should not output anything
        captured = capsys.readouterr()
        assert captured.out == ""

    def test_handles_step_with_tool_call(self, capsys):
        """Should print step with tool call."""
        from loom.cli.chat import print_stream_step_complete

        step = MagicMock()
        step.step = 1
        step.reasoning = "Thinking about this"
        step.tool_call = MagicMock()
        step.tool_call.name = "test:tool"
        step.observation = MagicMock()
        step.observation.success = True
        step.observation.output = "Tool result"
        step.reduced_step = None

        print_stream_step_complete(step)

        captured = capsys.readouterr()
        assert "Step 1" in captured.out
        assert "test:tool" in captured.out

    def test_handles_offloaded_data(self, capsys):
        """Should display offloaded data reference."""
        from loom.cli.chat import print_stream_step_complete

        step = MagicMock()
        step.step = 1
        step.reasoning = "Reading file"
        step.tool_call = MagicMock()
        step.tool_call.name = "fs:read_file"
        step.observation = MagicMock()
        step.observation.success = True
        step.reduced_step = MagicMock()
        step.reduced_step.outcome_ref = ".loom/cache/file.txt"
        step.reduced_step.observation = "Read 100 lines"

        print_stream_step_complete(step)

        captured = capsys.readouterr()
        assert "offloaded" in captured.out.lower()
        assert ".loom/cache/file.txt" in captured.out

    def test_handles_error(self, capsys):
        """Should display error."""
        from loom.cli.chat import print_stream_step_complete

        step = MagicMock()
        step.step = 1
        step.reasoning = "Trying something"
        step.tool_call = MagicMock()
        step.tool_call.name = "test:tool"
        step.observation = MagicMock()
        step.observation.success = False
        step.observation.error = "Something went wrong"
        step.reduced_step = None

        print_stream_step_complete(step)

        captured = capsys.readouterr()
        assert "Error" in captured.out
        assert "Something went wrong" in captured.out
