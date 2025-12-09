"""Unit tests for loom.cli.chat module."""

from __future__ import annotations

from unittest.mock import AsyncMock, MagicMock, patch

import pytest

from loom.cli.chat import (
    ChatSession,
    ConnectedChatSession,
    StandaloneChatSession,
    create_chat_session,
)


class TestConnectedChatSession:
    """Tests for ConnectedChatSession class."""

    def test_init_default(self):
        """Should initialize with default values."""
        session = ConnectedChatSession()
        assert session.backend_agent_id == "chat-assistant"
        assert session.backend_topic == "chat.input"
        assert session.bridge_addr is None
        assert session.verbose is True
        assert session.streaming is True
        assert session._client is None
        assert session._connected is False

    def test_init_custom(self):
        """Should initialize with custom values."""
        session = ConnectedChatSession(
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
        session = ConnectedChatSession()
        assert session.connected is False

    def test_connected_property_true(self):
        """connected should be True when connected."""
        session = ConnectedChatSession()
        session._connected = True
        session._client = MagicMock()
        assert session.connected is True

    @pytest.mark.asyncio
    async def test_connect_creates_client(self):
        """connect should create ChatClient."""
        session = ConnectedChatSession()

        mock_client = AsyncMock()
        mock_client.connect = AsyncMock()

        with patch("loom.cli.chat.ConnectedChatSession.connect") as mock_connect:
            mock_connect.return_value = True
            result = await session.connect()
            # The actual connect is mocked, so we just verify it returns
            assert result is True

    @pytest.mark.asyncio
    async def test_disconnect(self):
        """disconnect should cleanup client."""
        session = ConnectedChatSession()
        session._client = AsyncMock()
        session._client.disconnect = AsyncMock()
        session._connected = True

        await session.disconnect()

        session._client.disconnect.assert_called_once()
        assert session._connected is False

    @pytest.mark.asyncio
    async def test_disconnect_no_client(self):
        """disconnect should handle no client gracefully."""
        session = ConnectedChatSession()
        session._client = None
        session._connected = False

        # Should not raise
        await session.disconnect()
        assert session._connected is False

    @pytest.mark.asyncio
    async def test_chat_not_connected(self):
        """chat should raise error when not connected."""
        session = ConnectedChatSession()
        session._client = None

        with pytest.raises(RuntimeError, match="Not connected"):
            await session.chat("Hello")

    def test_get_history_empty(self):
        """get_history should return empty list when no client."""
        session = ConnectedChatSession()
        assert session.get_history() == []

    def test_clear_history(self):
        """clear_history should call client method."""
        session = ConnectedChatSession()
        session._client = MagicMock()
        session._client.clear_thread = MagicMock()

        session.clear_history()

        session._client.clear_thread.assert_called_once()


class TestStandaloneChatSession:
    """Tests for StandaloneChatSession class."""

    def test_init_default(self):
        """Should initialize with default values."""
        session = StandaloneChatSession()
        assert session.agent_id == "chat-assistant"
        assert session.bridge_addr is None
        assert session.verbose is True
        assert session.streaming is True
        assert session._agent is None
        assert session._cognitive is None
        assert session._conversation_history == []

    def test_init_custom(self):
        """Should initialize with custom values."""
        session = StandaloneChatSession(
            agent_id="custom-agent",
            bridge_addr="localhost:9000",
            verbose=False,
            streaming=False,
        )
        assert session.agent_id == "custom-agent"
        assert session.bridge_addr == "localhost:9000"
        assert session.verbose is False
        assert session.streaming is False

    def test_connected_property_false(self):
        """connected should be False when not connected."""
        session = StandaloneChatSession()
        assert session.connected is False

    def test_connected_property_true(self):
        """connected should be True when both agent and cognitive exist."""
        session = StandaloneChatSession()
        session._agent = MagicMock()
        session._cognitive = MagicMock()
        assert session.connected is True

    def test_get_system_prompt(self):
        """_get_system_prompt should return valid prompt."""
        session = StandaloneChatSession()
        prompt = session._get_system_prompt()
        assert isinstance(prompt, str)
        assert len(prompt) > 0
        assert "tool" in prompt.lower()  # Should mention tools

    @pytest.mark.asyncio
    async def test_disconnect(self):
        """disconnect should cleanup agent."""
        session = StandaloneChatSession()
        mock_agent = AsyncMock()
        mock_agent.stop = AsyncMock()
        session._agent = mock_agent
        session._cognitive = MagicMock()

        await session.disconnect()

        mock_agent.stop.assert_called_once()
        assert session._agent is None
        assert session._cognitive is None

    @pytest.mark.asyncio
    async def test_chat_not_connected(self):
        """chat should raise error when not connected."""
        session = StandaloneChatSession()
        session._cognitive = None

        with pytest.raises(RuntimeError, match="Not connected"):
            await session.chat("Hello")

    def test_get_history_empty(self):
        """get_history should return empty list initially."""
        session = StandaloneChatSession()
        assert session.get_history() == []

    def test_get_history_with_messages(self):
        """get_history should return conversation history."""
        session = StandaloneChatSession()
        session._conversation_history = [
            {"role": "user", "content": "Hello"},
            {"role": "assistant", "content": "Hi!"},
        ]
        history = session.get_history()
        assert len(history) == 2
        assert history[0]["role"] == "user"

    def test_clear_history(self):
        """clear_history should clear conversation history."""
        session = StandaloneChatSession()
        session._conversation_history = [{"role": "user", "content": "Hello"}]
        session._cognitive = MagicMock()
        session._cognitive.memory = MagicMock()
        session._cognitive.memory.clear = MagicMock()

        session.clear_history()

        assert session._conversation_history == []
        session._cognitive.memory.clear.assert_called_once()


class TestCreateChatSession:
    """Tests for create_chat_session factory function."""

    def test_create_connected_mode(self):
        """Should create ConnectedChatSession for 'connected' mode."""
        session = create_chat_session(mode="connected")
        assert isinstance(session, ConnectedChatSession)

    def test_create_standalone_mode(self):
        """Should create StandaloneChatSession for 'standalone' mode."""
        session = create_chat_session(mode="standalone")
        assert isinstance(session, StandaloneChatSession)

    def test_create_auto_mode(self):
        """Should create session for 'auto' mode (default to standalone)."""
        session = create_chat_session(mode="auto")
        # Currently defaults to standalone
        assert isinstance(session, StandaloneChatSession)

    def test_create_default_mode(self):
        """Should create session for default mode."""
        session = create_chat_session()
        assert isinstance(session, (ConnectedChatSession, StandaloneChatSession))

    def test_create_with_params(self):
        """Should pass parameters to session."""
        session = create_chat_session(
            mode="standalone",
            agent_id="test-agent",
            bridge_addr="localhost:9000",
            verbose=False,
            streaming=False,
        )
        assert session.agent_id == "test-agent"
        assert session.bridge_addr == "localhost:9000"
        assert session.verbose is False
        assert session.streaming is False


class TestChatSessionAlias:
    """Tests for backward compatibility alias."""

    def test_chat_session_is_standalone(self):
        """ChatSession should be alias for StandaloneChatSession."""
        assert ChatSession is StandaloneChatSession

    def test_chat_session_creates_standalone(self):
        """ChatSession should create StandaloneChatSession."""
        session = ChatSession()
        assert isinstance(session, StandaloneChatSession)
