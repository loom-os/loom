"""Unit tests for LLM provider module - including streaming."""

from unittest.mock import AsyncMock, MagicMock, patch

import pytest

# ============================================================================
# LLM Provider Tests
# ============================================================================


class TestLLMConfig:
    """Tests for LLMConfig."""

    def test_default_config(self):
        """Test default LLMConfig values."""
        from loom.llm.config import LLMConfig

        config = LLMConfig(
            base_url="http://localhost:8000/v1",
            model="test-model",
        )

        assert config.base_url == "http://localhost:8000/v1"
        assert config.model == "test-model"
        assert config.api_key is None
        assert config.temperature == 0.7
        assert config.max_tokens == 4096
        assert config.timeout_ms == 30000

    def test_custom_config(self):
        """Test custom LLMConfig values."""
        from loom.llm.config import LLMConfig

        config = LLMConfig(
            base_url="https://api.openai.com/v1",
            model="gpt-4",
            api_key="sk-test",
            temperature=0.5,
            max_tokens=2048,
            timeout_ms=60000,
        )

        assert config.api_key == "sk-test"
        assert config.temperature == 0.5
        assert config.max_tokens == 2048
        assert config.timeout_ms == 60000


class TestLLMProviderPresets:
    """Tests for LLMProvider preset configurations."""

    def test_from_name_deepseek(self):
        """Test loading deepseek preset."""
        from loom.llm.provider import LLMProvider

        # Mock context
        mock_ctx = MagicMock()
        mock_ctx.agent_id = "test-agent"

        provider = LLMProvider.from_name(mock_ctx, "deepseek")

        assert "deepseek" in provider.config.base_url.lower()
        assert "deepseek" in provider.config.model.lower()

    def test_from_name_openai(self):
        """Test loading openai preset."""
        from loom.llm.provider import LLMProvider

        mock_ctx = MagicMock()
        mock_ctx.agent_id = "test-agent"

        provider = LLMProvider.from_name(mock_ctx, "openai")

        assert "openai" in provider.config.base_url.lower()

    def test_from_name_local(self):
        """Test loading local preset."""
        from loom.llm.provider import LLMProvider

        mock_ctx = MagicMock()
        mock_ctx.agent_id = "test-agent"

        provider = LLMProvider.from_name(mock_ctx, "local")

        assert "localhost" in provider.config.base_url.lower()

    def test_from_name_unknown_raises(self):
        """Test that unknown provider name raises ValueError."""
        from loom.llm.provider import LLMProvider

        mock_ctx = MagicMock()
        mock_ctx.agent_id = "test-agent"

        with pytest.raises(ValueError, match="Unknown provider"):
            LLMProvider.from_name(mock_ctx, "unknown_provider")


class TestLLMProviderGenerate:
    """Tests for LLMProvider.generate() method."""

    @pytest.mark.asyncio
    async def test_generate_basic(self):
        """Test basic generate call."""
        from loom.llm.config import LLMConfig
        from loom.llm.provider import LLMProvider

        mock_ctx = MagicMock()
        mock_ctx.agent_id = "test-agent"

        config = LLMConfig(
            base_url="http://test.local/v1",
            model="test-model",
        )
        provider = LLMProvider(mock_ctx, config)

        # Mock httpx response
        mock_response_data = {
            "choices": [{"message": {"content": "Hello, world!"}}],
            "usage": {"prompt_tokens": 10, "completion_tokens": 5},
        }

        with patch("httpx.AsyncClient") as mock_client_class:
            # Create mock response
            mock_response = MagicMock()
            mock_response.json.return_value = mock_response_data
            mock_response.raise_for_status = MagicMock()

            # Create mock client instance
            mock_client = AsyncMock()
            mock_client.post = AsyncMock(return_value=mock_response)

            # Setup async context manager
            mock_client_class.return_value.__aenter__ = AsyncMock(return_value=mock_client)
            mock_client_class.return_value.__aexit__ = AsyncMock(return_value=None)

            result = await provider.generate("Say hello")

            assert result == "Hello, world!"
            mock_client.post.assert_called_once()

    @pytest.mark.asyncio
    async def test_generate_with_system_prompt(self):
        """Test generate with system prompt."""
        from loom.llm.config import LLMConfig
        from loom.llm.provider import LLMProvider

        mock_ctx = MagicMock()
        mock_ctx.agent_id = "test-agent"

        config = LLMConfig(
            base_url="http://test.local/v1",
            model="test-model",
        )
        provider = LLMProvider(mock_ctx, config)

        mock_response_data = {
            "choices": [{"message": {"content": "I am a helpful assistant."}}],
        }

        with patch("httpx.AsyncClient") as mock_client_class:
            mock_response = MagicMock()
            mock_response.json.return_value = mock_response_data
            mock_response.raise_for_status = MagicMock()

            mock_client = AsyncMock()
            mock_client.post = AsyncMock(return_value=mock_response)

            mock_client_class.return_value.__aenter__ = AsyncMock(return_value=mock_client)
            mock_client_class.return_value.__aexit__ = AsyncMock(return_value=None)

            await provider.generate(
                "Who are you?",
                system="You are a helpful assistant.",
            )

            # Check that system message was included in the request
            call_args = mock_client.post.call_args
            payload = call_args.kwargs.get("json", {})
            messages = payload.get("messages", [])

            assert len(messages) == 2
            assert messages[0]["role"] == "system"
            assert messages[1]["role"] == "user"


class TestLLMProviderGenerateStream:
    """Tests for LLMProvider.generate_stream() method."""

    @pytest.mark.asyncio
    async def test_generate_stream_yields_chunks(self):
        """Test that generate_stream yields text chunks."""
        from loom.llm.config import LLMConfig
        from loom.llm.provider import LLMProvider

        mock_ctx = MagicMock()
        mock_ctx.agent_id = "test-agent"

        config = LLMConfig(
            base_url="http://test.local/v1",
            model="test-model",
        )
        provider = LLMProvider(mock_ctx, config)

        # Mock SSE stream data
        sse_lines = [
            'data: {"choices": [{"delta": {"content": "Hello"}}]}',
            'data: {"choices": [{"delta": {"content": " "}}]}',
            'data: {"choices": [{"delta": {"content": "World"}}]}',
            'data: {"choices": [{"delta": {"content": "!"}}]}',
            "data: [DONE]",
        ]

        async def mock_aiter_lines():
            for line in sse_lines:
                yield line

        with patch("httpx.AsyncClient") as mock_client_class:
            # Create mock stream response
            mock_stream_response = MagicMock()
            mock_stream_response.raise_for_status = MagicMock()
            mock_stream_response.aiter_lines = mock_aiter_lines

            # Create async context manager for stream
            mock_stream_cm = MagicMock()
            mock_stream_cm.__aenter__ = AsyncMock(return_value=mock_stream_response)
            mock_stream_cm.__aexit__ = AsyncMock(return_value=None)

            # Create mock client
            mock_client = MagicMock()
            mock_client.stream = MagicMock(return_value=mock_stream_cm)

            # Setup outer async context manager
            mock_client_class.return_value.__aenter__ = AsyncMock(return_value=mock_client)
            mock_client_class.return_value.__aexit__ = AsyncMock(return_value=None)

            chunks = []
            async for chunk in provider.generate_stream("Say hello"):
                chunks.append(chunk)

            assert chunks == ["Hello", " ", "World", "!"]
            assert "".join(chunks) == "Hello World!"

    @pytest.mark.asyncio
    async def test_generate_stream_handles_empty_chunks(self):
        """Test that generate_stream skips empty chunks."""
        from loom.llm.config import LLMConfig
        from loom.llm.provider import LLMProvider

        mock_ctx = MagicMock()
        mock_ctx.agent_id = "test-agent"

        config = LLMConfig(
            base_url="http://test.local/v1",
            model="test-model",
        )
        provider = LLMProvider(mock_ctx, config)

        # Some chunks have empty content
        sse_lines = [
            'data: {"choices": [{"delta": {"content": "Hi"}}]}',
            'data: {"choices": [{"delta": {}}]}',  # No content
            'data: {"choices": [{"delta": {"content": ""}}]}',  # Empty content
            'data: {"choices": [{"delta": {"content": "!"}}]}',
            "data: [DONE]",
        ]

        async def mock_aiter_lines():
            for line in sse_lines:
                yield line

        with patch("httpx.AsyncClient") as mock_client_class:
            mock_stream_response = MagicMock()
            mock_stream_response.raise_for_status = MagicMock()
            mock_stream_response.aiter_lines = mock_aiter_lines

            mock_stream_cm = MagicMock()
            mock_stream_cm.__aenter__ = AsyncMock(return_value=mock_stream_response)
            mock_stream_cm.__aexit__ = AsyncMock(return_value=None)

            mock_client = MagicMock()
            mock_client.stream = MagicMock(return_value=mock_stream_cm)

            mock_client_class.return_value.__aenter__ = AsyncMock(return_value=mock_client)
            mock_client_class.return_value.__aexit__ = AsyncMock(return_value=None)

            chunks = []
            async for chunk in provider.generate_stream("Say hi"):
                chunks.append(chunk)

            # Should skip empty chunks
            assert chunks == ["Hi", "!"]

    @pytest.mark.asyncio
    async def test_generate_stream_handles_malformed_json(self):
        """Test that generate_stream handles malformed JSON gracefully."""
        from loom.llm.config import LLMConfig
        from loom.llm.provider import LLMProvider

        mock_ctx = MagicMock()
        mock_ctx.agent_id = "test-agent"

        config = LLMConfig(
            base_url="http://test.local/v1",
            model="test-model",
        )
        provider = LLMProvider(mock_ctx, config)

        # Some malformed data
        sse_lines = [
            'data: {"choices": [{"delta": {"content": "OK"}}]}',
            "data: {malformed json}",
            "",  # Empty line
            "not a data line",
            'data: {"choices": [{"delta": {"content": "!"}}]}',
            "data: [DONE]",
        ]

        async def mock_aiter_lines():
            for line in sse_lines:
                yield line

        with patch("httpx.AsyncClient") as mock_client_class:
            mock_stream_response = MagicMock()
            mock_stream_response.raise_for_status = MagicMock()
            mock_stream_response.aiter_lines = mock_aiter_lines

            mock_stream_cm = MagicMock()
            mock_stream_cm.__aenter__ = AsyncMock(return_value=mock_stream_response)
            mock_stream_cm.__aexit__ = AsyncMock(return_value=None)

            mock_client = MagicMock()
            mock_client.stream = MagicMock(return_value=mock_stream_cm)

            mock_client_class.return_value.__aenter__ = AsyncMock(return_value=mock_client)
            mock_client_class.return_value.__aexit__ = AsyncMock(return_value=None)

            chunks = []
            async for chunk in provider.generate_stream("Test"):
                chunks.append(chunk)

            # Should get valid chunks, skip malformed
            assert chunks == ["OK", "!"]


class TestLLMProviderChat:
    """Tests for LLMProvider.chat() method."""

    @pytest.mark.asyncio
    async def test_chat_with_history(self):
        """Test chat with message history."""
        from loom.llm.config import LLMConfig
        from loom.llm.provider import LLMProvider

        mock_ctx = MagicMock()
        mock_ctx.agent_id = "test-agent"

        config = LLMConfig(
            base_url="http://test.local/v1",
            model="test-model",
        )
        provider = LLMProvider(mock_ctx, config)

        mock_response_data = {
            "choices": [{"message": {"content": "I said hello earlier."}}],
        }

        with patch("httpx.AsyncClient") as mock_client_class:
            mock_response = MagicMock()
            mock_response.json.return_value = mock_response_data
            mock_response.raise_for_status = MagicMock()

            mock_client = AsyncMock()
            mock_client.post = AsyncMock(return_value=mock_response)

            mock_client_class.return_value.__aenter__ = AsyncMock(return_value=mock_client)
            mock_client_class.return_value.__aexit__ = AsyncMock(return_value=None)

            messages = [
                {"role": "user", "content": "Hello"},
                {"role": "assistant", "content": "Hi there!"},
                {"role": "user", "content": "What did I say?"},
            ]

            result = await provider.chat(messages)

            assert result == "I said hello earlier."

            # Verify all messages were sent
            call_args = mock_client.post.call_args
            payload = call_args.kwargs.get("json", {})
            assert payload["messages"] == messages
