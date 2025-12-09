"""Streaming Module - Real-time response streaming for Loom agents.

This module provides streaming infrastructure for real-time LLM output
and agent communication. It is part of loom-py SDK (not apps/).

Architecture Position:
    loom-proto:  Defines StreamChunk, StreamComplete, etc. (protocol layer)
    loom-py/streaming:  Python streaming types and client (SDK layer)
    loom-core/bridge:  Routes streaming messages (runtime layer)
    apps/*:  Business logic uses streaming (application layer)

Key Components:
    - StreamChunk: Individual content chunk from LLM/agent
    - StreamComplete: Completion signal with statistics
    - StreamingClient: Client for receiving/sending streams
    - StreamingHandler: Handler for backend agents producing streams

Example (Client receiving stream):
    ```python
    from loom.streaming import StreamingClient

    client = StreamingClient(bridge_addr="127.0.0.1:50051")
    await client.connect()

    # Send message and receive streaming response
    async for chunk in client.stream_message("Explain quantum computing"):
        print(chunk.content, end="", flush=True)
    ```

Example (Backend producing stream):
    ```python
    from loom.streaming import StreamingHandler

    handler = StreamingHandler(ctx)

    # In event handler
    async def on_event(ctx, topic, event):
        if event.type == "user.message":
            async for llm_chunk in cognitive.run_stream(event.payload):
                await handler.send_chunk(event, llm_chunk)
            await handler.send_complete(event)
    ```
"""

from .chat_client import ChatClient, ChatMessage, ChatResponse, ChunkCallback
from .client import StreamingClient, StreamingError
from .handler import StreamingHandler
from .types import (
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

__all__ = [
    # Types
    "StreamChunk",
    "StreamComplete",
    "StreamContentType",
    "StreamError",
    "StreamOptions",
    "StreamRequest",
    "StreamState",
    "StreamStateKind",
    "StreamStats",
    "StreamStatus",
    # Client
    "StreamingClient",
    "StreamingError",
    # Handler
    "StreamingHandler",
    # Chat Client
    "ChatClient",
    "ChatMessage",
    "ChatResponse",
    "ChunkCallback",
]
