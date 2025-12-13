"""Streaming Client - Client for receiving streaming responses.

This module provides StreamingClient for clients (CLI, Dashboard) to
receive streaming responses from backend agents through the Bridge.

The client manages:
- Connection to Bridge
- Sending stream requests
- Receiving and aggregating chunks
- Handling completion and errors

Example:
    ```python
    from loom.streaming import StreamingClient, StreamOptions

    async def main():
        client = StreamingClient(agent_id="my-client")
        await client.connect()

        # Stream a message
        async for chunk in client.stream("Explain quantum computing"):
            print(chunk.content, end="", flush=True)

        # Or with options
        options = StreamOptions(include_thinking=True)
        async for chunk in client.stream("Solve this problem", options=options):
            if chunk.content_type == StreamContentType.THINKING:
                print(f"[Thinking] {chunk.content}")
            else:
                print(chunk.content, end="")

        await client.disconnect()
    ```
"""

from __future__ import annotations

import asyncio
import uuid
from typing import TYPE_CHECKING, AsyncIterator, Optional

from opentelemetry import trace

from .types import (
    PermissionResponse,
    StreamChunk,
    StreamComplete,
    StreamContentType,
    StreamOptions,
    StreamRequest,
    StreamStatus,
)

# Tracer for streaming client spans
tracer = trace.get_tracer(__name__)

if TYPE_CHECKING:
    from ..agent import Agent, EventContext
    from ..agent.envelope import Envelope


class StreamingClient:
    """Client for receiving streaming responses from backend agents.

    This is the recommended way for CLI tools and dashboards to
    interact with cognitive agents that produce streaming output.

    Attributes:
        agent_id: Unique ID for this client
        backend_topic: Topic to send requests to
        connected: Whether client is connected
    """

    def __init__(
        self,
        agent_id: Optional[str] = None,
        backend_topic: str = "chat.input",
        bridge_addr: Optional[str] = None,
    ):
        """Initialize streaming client.

        Args:
            agent_id: Unique client ID (auto-generated if not provided)
            backend_topic: Topic where backend agent listens
            bridge_addr: Bridge address (default from env or 127.0.0.1:50051)
        """
        self.agent_id = agent_id or f"stream-client-{uuid.uuid4().hex[:8]}"
        self.backend_topic = backend_topic
        self.bridge_addr = bridge_addr

        self._agent: Optional["Agent"] = None
        self._pending_streams: dict[str, asyncio.Queue] = {}
        self._stream_results: dict[str, StreamComplete] = {}
        self._connected = False

    @property
    def connected(self) -> bool:
        """Check if client is connected."""
        return self._connected and self._agent is not None

    async def connect(self) -> None:
        """Connect to Bridge and start receiving events."""
        from ..agent import Agent

        self._agent = Agent(
            agent_id=self.agent_id,
            topics=[],  # We listen on our reply topic (auto-subscribed)
            address=self.bridge_addr,
        )

        # Set up event handler for streaming responses
        self._agent._on_event = self._handle_event
        await self._agent.start()
        self._connected = True

    async def disconnect(self) -> None:
        """Disconnect from Bridge."""
        if self._agent:
            await self._agent.stop()
            self._agent = None
        self._connected = False
        self._pending_streams.clear()
        self._stream_results.clear()

    async def stream(
        self,
        message: str,
        thread_id: Optional[str] = None,
        options: Optional[StreamOptions] = None,
        timeout: float = 120.0,
    ) -> AsyncIterator[StreamChunk]:
        """Send a message and stream the response.

        Args:
            message: The message to send
            thread_id: Optional conversation thread ID
            options: Stream options
            timeout: Total timeout in seconds

        Yields:
            StreamChunk objects as they arrive

        Raises:
            RuntimeError: If not connected
            TimeoutError: If stream times out
            StreamError: If backend returns an error
        """
        if not self.connected:
            raise RuntimeError("Client not connected. Call connect() first.")

        # Create request
        request = StreamRequest(
            message=message,
            thread_id=thread_id or "",
            sender=self.agent_id,
            reply_to=f"agent.{self.agent_id}.replies",
            options=options,
        )

        # Set up queue for this stream
        stream_queue: asyncio.Queue = asyncio.Queue()
        self._pending_streams[request.id] = stream_queue

        # Create tracing span for the entire stream
        with tracer.start_as_current_span(
            "streaming.client.stream",
            attributes={
                "streaming.client_id": self.agent_id,
                "streaming.backend_topic": self.backend_topic,
                "streaming.message_length": len(message),
                "streaming.correlation_id": request.id,
                "streaming.thread_id": thread_id or "",
                "streaming.timeout_sec": timeout,
            },
        ) as span:
            try:
                # Send request via event bus
                from ..agent.envelope import Envelope

                env = Envelope.new(
                    type="stream.request",
                    payload=message.encode("utf-8"),
                    sender=self.agent_id,
                    correlation_id=request.id,
                    thread_id=request.thread_id,
                    reply_to=request.reply_to,
                    metadata={
                        "stream.include_thinking": str(
                            options.include_thinking if options else False
                        ),
                        "stream.include_tool_calls": str(
                            options.include_tool_calls if options else True
                        ),
                    },
                )

                # Inject trace context for distributed tracing
                env.inject_trace_context()

                await self._agent.ctx.emit(
                    self.backend_topic,
                    type="stream.request",
                    payload=message.encode("utf-8"),
                    envelope=env,
                )

                span.add_event("request_sent")

                # Receive chunks until complete
                deadline = asyncio.get_event_loop().time() + timeout
                chunks_received = 0
                first_chunk_received = False

                while True:
                    remaining = deadline - asyncio.get_event_loop().time()
                    if remaining <= 0:
                        span.set_attribute("streaming.timed_out", True)
                        raise TimeoutError(f"Stream timed out after {timeout}s")

                    try:
                        item = await asyncio.wait_for(stream_queue.get(), timeout=remaining)
                    except asyncio.TimeoutError as exc:
                        span.set_attribute("streaming.timed_out", True)
                        raise TimeoutError(f"Stream timed out after {timeout}s") from exc

                    if isinstance(item, StreamChunk):
                        if not first_chunk_received:
                            span.add_event("first_chunk_received")
                            first_chunk_received = True
                        chunks_received += 1
                        yield item
                    elif isinstance(item, StreamComplete):
                        # Store result and record stats
                        self._stream_results[request.id] = item
                        span.set_attribute("streaming.chunks_received", chunks_received)
                        span.set_attribute("streaming.status", item.status.name)
                        if item.stats:
                            span.set_attribute("streaming.duration_ms", item.stats.duration_ms)
                            span.set_attribute("streaming.total_tokens", item.stats.total_tokens)
                            span.set_attribute("streaming.tool_calls", item.stats.tool_calls)
                        if item.status == StreamStatus.ERROR and item.error:
                            span.set_attribute("streaming.error_code", item.error.code)
                            span.record_exception(
                                StreamingError(item.error.code, item.error.message)
                            )
                            raise StreamingError(item.error.code, item.error.message)
                        break
                    elif isinstance(item, Exception):
                        span.record_exception(item)
                        raise item

            finally:
                self._pending_streams.pop(request.id, None)

    async def send_message(
        self,
        message: str,
        thread_id: Optional[str] = None,
        timeout: float = 120.0,
    ) -> str:
        """Send a message and get the complete response (non-streaming).

        Args:
            message: The message to send
            thread_id: Optional conversation thread ID
            timeout: Timeout in seconds

        Returns:
            Complete response text

        Raises:
            RuntimeError: If not connected
            TimeoutError: If request times out
        """
        chunks = []
        async for chunk in self.stream(message, thread_id=thread_id, timeout=timeout):
            if chunk.content_type == StreamContentType.TEXT:
                chunks.append(chunk.content)
        return "".join(chunks)

    async def send_permission_response(self, response: "PermissionResponse") -> None:
        """Send a permission response back to the backend.

        Args:
            response: The permission response to send
        """
        if not self.connected:
            return

        from ..agent.envelope import Envelope

        env = Envelope.new(
            type="permission.response",
            payload=response.to_json().encode("utf-8"),
            sender=self.agent_id,
            metadata={"request_id": response.request_id},
        )

        await self._agent.ctx.emit(
            self.backend_topic,
            type="permission.response",
            payload=response.to_json().encode("utf-8"),
            envelope=env,
        )

    async def cancel_stream(self, correlation_id: str, reason: str = "") -> None:
        """Cancel an ongoing stream.

        Args:
            correlation_id: ID of the stream to cancel
            reason: Cancellation reason
        """
        if not self.connected:
            return

        from ..agent.envelope import Envelope

        env = Envelope.new(
            type="stream.cancel",
            payload=reason.encode("utf-8"),
            sender=self.agent_id,
            correlation_id=correlation_id,
        )

        await self._agent.ctx.emit(
            self.backend_topic,
            type="stream.cancel",
            payload=reason.encode("utf-8"),
            envelope=env,
        )

    def get_stream_result(self, correlation_id: str) -> Optional[StreamComplete]:
        """Get the completion result for a finished stream.

        Args:
            correlation_id: ID of the completed stream

        Returns:
            StreamComplete if available, None otherwise
        """
        return self._stream_results.get(correlation_id)

    async def _handle_event(
        self,
        ctx: "EventContext",
        topic: str,
        event: "Envelope",
    ) -> None:
        """Handle incoming streaming events."""
        # Find the pending stream for this correlation
        correlation_id = event.correlation_id
        if not correlation_id:
            return

        queue = self._pending_streams.get(correlation_id)
        if not queue:
            # No pending stream for this correlation - could be old or unknown
            return

        try:
            if event.type == "stream.chunk":
                # Parse chunk from metadata
                chunk = StreamChunk(
                    correlation_id=correlation_id,
                    content=event.payload.decode("utf-8"),
                    sequence=int(event.metadata.get("stream.sequence", "0")),
                    content_type=StreamContentType(
                        int(event.metadata.get("stream.content_type", "0"))
                    ),
                    metadata={
                        k: v
                        for k, v in event.metadata.items()
                        if not k.startswith("stream.") and not k.startswith("loom.")
                    },
                    timestamp_ms=event.timestamp_ms,
                )
                await queue.put(chunk)

            elif event.type == "stream.complete":
                # Parse completion from metadata
                from .types import StreamError, StreamStats

                error = None
                if event.metadata.get("stream.error_code"):
                    error = StreamError(
                        code=event.metadata.get("stream.error_code", ""),
                        message=event.metadata.get("stream.error_message", ""),
                    )

                stats = StreamStats(
                    duration_ms=int(event.metadata.get("stream.duration_ms", "0")),
                    tool_calls=int(event.metadata.get("stream.tool_calls", "0")),
                    first_chunk_latency_ms=int(
                        event.metadata.get("stream.first_chunk_latency_ms", "0")
                    ),
                )

                complete = StreamComplete(
                    correlation_id=correlation_id,
                    final_content=event.payload.decode("utf-8"),
                    total_chunks=int(event.metadata.get("stream.total_chunks", "0")),
                    status=StreamStatus(int(event.metadata.get("stream.status", "0"))),
                    error=error,
                    stats=stats,
                )
                await queue.put(complete)

            elif event.type == "stream.state":
                # State updates - could log or pass through
                pass

            # Also handle legacy message types for backward compatibility
            elif event.type == "assistant.message":
                # Legacy non-streaming response - convert to single chunk + complete
                content = event.payload.decode("utf-8")
                chunk = StreamChunk(
                    correlation_id=correlation_id,
                    content=content,
                    sequence=0,
                    content_type=StreamContentType.TEXT,
                )
                await queue.put(chunk)
                await queue.put(
                    StreamComplete(
                        correlation_id=correlation_id,
                        final_content=content,
                        total_chunks=1,
                        status=StreamStatus.OK,
                    )
                )

            elif event.type == "assistant.error":
                # Legacy error response
                error_msg = event.payload.decode("utf-8")
                await queue.put(
                    StreamComplete(
                        correlation_id=correlation_id,
                        status=StreamStatus.ERROR,
                        error=StreamError(
                            code="LEGACY_ERROR",
                            message=error_msg,
                        ),
                    )
                )

        except Exception as e:
            # Put exception in queue so stream() can raise it
            await queue.put(e)


class StreamingError(Exception):
    """Error from streaming response."""

    def __init__(self, code: str, message: str):
        self.code = code
        self.message = message
        super().__init__(f"{code}: {message}")
