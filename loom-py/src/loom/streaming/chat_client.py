"""Chat Client - High-level client for conversational AI interactions.

This module provides ChatClient, a user-friendly interface for chatting
with backend cognitive agents. It builds on StreamingClient with
conversation management and rich callback support.

Architecture Position:
    StreamingClient:  Low-level streaming over event bus
    ChatClient:  High-level chat abstraction with conversation state
    CLI/Dashboard:  UI layer using ChatClient

Key Features:
    - Conversation history management
    - Streaming with typed callbacks
    - Automatic reconnection
    - Thread/session management

Example:
    ```python
    from loom.streaming import ChatClient

    async def on_chunk(content: str, content_type: StreamContentType):
        if content_type == StreamContentType.TEXT:
            print(content, end="", flush=True)
        elif content_type == StreamContentType.THINKING:
            print(f"\\n[Thinking] {content}")

    async def main():
        client = ChatClient()
        await client.connect()

        # Start a new conversation
        thread_id = client.new_thread()

        # Chat with streaming callback
        response = await client.chat(
            "Explain quantum entanglement",
            thread_id=thread_id,
            on_chunk=on_chunk,
        )
        print(f"\\n\\n[Complete: {response.stats.duration_ms}ms]")

        await client.disconnect()
    ```
"""

from __future__ import annotations

import time
import uuid
from dataclasses import dataclass, field
from typing import Any, Awaitable, Callable, Dict, List, Optional

from opentelemetry import trace

from .client import StreamingClient
from .types import (
    StreamContentType,
    StreamOptions,
    StreamStats,
    StreamStatus,
)

# Tracer for chat client spans
tracer = trace.get_tracer(__name__)


@dataclass
class ChatMessage:
    """A message in a conversation."""

    role: str  # "user" or "assistant"
    content: str
    thread_id: str = ""
    timestamp_ms: int = 0
    metadata: Dict[str, Any] = field(default_factory=dict)


@dataclass
class ChatResponse:
    """Response from a chat request."""

    content: str
    thread_id: str
    stats: Optional[StreamStats] = None
    tool_calls: List[str] = field(default_factory=list)
    thinking: List[str] = field(default_factory=list)
    status: StreamStatus = StreamStatus.OK


# Callback type for streaming chunks
ChunkCallback = Callable[[str, StreamContentType], Awaitable[None]]


class ChatClient:
    """High-level client for conversational AI interactions.

    Provides a user-friendly interface for chatting with backend agents,
    with support for conversation management and streaming output.

    Attributes:
        connected: Whether client is connected
        current_thread: Current active thread ID
    """

    def __init__(
        self,
        backend_topic: str = "chat.input",
        bridge_addr: Optional[str] = None,
        client_id: Optional[str] = None,
    ):
        """Initialize chat client.

        Args:
            backend_topic: Topic where backend agent listens
            bridge_addr: Bridge address (default from env)
            client_id: Unique client ID (auto-generated if not provided)
        """
        self._client_id = client_id or f"chat-client-{uuid.uuid4().hex[:8]}"
        self._streaming = StreamingClient(
            agent_id=self._client_id,
            backend_topic=backend_topic,
            bridge_addr=bridge_addr,
        )

        # Conversation state
        self._threads: Dict[str, List[ChatMessage]] = {}
        self._current_thread: Optional[str] = None

    @property
    def connected(self) -> bool:
        """Check if client is connected."""
        return self._streaming.connected

    @property
    def current_thread(self) -> Optional[str]:
        """Get current active thread ID."""
        return self._current_thread

    @property
    def client_id(self) -> str:
        """Get client ID."""
        return self._client_id

    async def connect(self) -> None:
        """Connect to Bridge."""
        await self._streaming.connect()

    async def disconnect(self) -> None:
        """Disconnect from Bridge."""
        await self._streaming.disconnect()

    def new_thread(self) -> str:
        """Start a new conversation thread.

        Returns:
            New thread ID
        """
        thread_id = str(uuid.uuid4())
        self._threads[thread_id] = []
        self._current_thread = thread_id
        return thread_id

    def get_thread_history(self, thread_id: Optional[str] = None) -> List[ChatMessage]:
        """Get conversation history for a thread.

        Args:
            thread_id: Thread ID (uses current if not provided)

        Returns:
            List of messages in the thread
        """
        tid = thread_id or self._current_thread
        if not tid:
            return []
        return self._threads.get(tid, [])

    def clear_thread(self, thread_id: Optional[str] = None) -> None:
        """Clear conversation history for a thread.

        Args:
            thread_id: Thread ID (uses current if not provided)
        """
        tid = thread_id or self._current_thread
        if tid and tid in self._threads:
            self._threads[tid] = []

    async def chat(
        self,
        message: str,
        thread_id: Optional[str] = None,
        on_chunk: Optional[ChunkCallback] = None,
        include_thinking: bool = False,
        include_tool_calls: bool = True,
        timeout: float = 120.0,
    ) -> ChatResponse:
        """Send a message and get a response.

        Args:
            message: The message to send
            thread_id: Thread ID (uses current if not provided)
            on_chunk: Async callback for streaming chunks
            include_thinking: Include thinking content in stream
            include_tool_calls: Include tool calls in stream
            timeout: Request timeout in seconds

        Returns:
            ChatResponse with complete response and metadata

        Raises:
            RuntimeError: If not connected
            TimeoutError: If request times out
        """
        if not self.connected:
            raise RuntimeError("Not connected. Call connect() first.")

        # Use provided thread or current
        tid = thread_id or self._current_thread or self.new_thread()

        # Create tracing span for the chat request
        with tracer.start_as_current_span(
            "chat.request",
            attributes={
                "chat.client_id": self._client_id,
                "chat.thread_id": tid,
                "chat.message_length": len(message),
                "chat.include_thinking": include_thinking,
                "chat.include_tool_calls": include_tool_calls,
            },
        ) as span:
            start_time = time.time()

            # Store user message in history
            user_msg = ChatMessage(
                role="user",
                content=message,
                thread_id=tid,
                timestamp_ms=int(time.time() * 1000),
            )
            if tid not in self._threads:
                self._threads[tid] = []
            self._threads[tid].append(user_msg)

            # Build stream options
            options = StreamOptions(
                include_thinking=include_thinking,
                include_tool_calls=include_tool_calls,
            )

            # Collect response
            content_parts: List[str] = []
            thinking_parts: List[str] = []
            tool_calls: List[str] = []
            chunks_received = 0

            try:
                async for chunk in self._streaming.stream(
                    message,
                    thread_id=tid,
                    options=options,
                    timeout=timeout,
                ):
                    chunks_received += 1

                    # Call user callback if provided
                    if on_chunk:
                        await on_chunk(chunk.content, chunk.content_type)

                    # Collect content by type
                    if chunk.content_type == StreamContentType.TEXT:
                        content_parts.append(chunk.content)
                    elif chunk.content_type == StreamContentType.THINKING:
                        thinking_parts.append(chunk.content)
                    elif chunk.content_type == StreamContentType.TOOL_CALL:
                        tool_calls.append(chunk.content)

            except Exception as e:
                span.record_exception(e)
                span.set_attribute("chat.error", str(e))
                raise

            # Build response
            full_content = "".join(content_parts)
            duration_ms = int((time.time() - start_time) * 1000)

            # Record metrics on span
            span.set_attribute("chat.response_length", len(full_content))
            span.set_attribute("chat.chunks_received", chunks_received)
            span.set_attribute("chat.thinking_steps", len(thinking_parts))
            span.set_attribute("chat.tool_calls", len(tool_calls))
            span.set_attribute("chat.duration_ms", duration_ms)

            # Store assistant response in history
            assistant_msg = ChatMessage(
                role="assistant",
                content=full_content,
                thread_id=tid,
                timestamp_ms=int(time.time() * 1000),
                metadata={
                    "thinking": thinking_parts,
                    "tool_calls": tool_calls,
                },
            )
            self._threads[tid].append(assistant_msg)

            # Get stats from streaming result if available
            stream_result = self._streaming.get_stream_result(
                list(self._streaming._pending_streams.keys())[-1]
                if self._streaming._pending_streams
                else ""
            )
            stats = stream_result.stats if stream_result else StreamStats(duration_ms=duration_ms)

            return ChatResponse(
                content=full_content,
                thread_id=tid,
                thinking=thinking_parts,
                tool_calls=tool_calls,
                status=StreamStatus.OK,
                stats=stats,
            )

    async def chat_sync(
        self,
        message: str,
        thread_id: Optional[str] = None,
        timeout: float = 120.0,
    ) -> str:
        """Send a message and get response synchronously (no streaming).

        Args:
            message: The message to send
            thread_id: Thread ID (uses current if not provided)
            timeout: Request timeout

        Returns:
            Complete response text
        """
        response = await self.chat(message, thread_id=thread_id, timeout=timeout)
        return response.content

    async def __aenter__(self) -> "ChatClient":
        """Async context manager entry."""
        await self.connect()
        return self

    async def __aexit__(self, exc_type, exc_val, exc_tb) -> None:
        """Async context manager exit."""
        await self.disconnect()
