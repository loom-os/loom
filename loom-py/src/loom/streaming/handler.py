"""Streaming Handler - Backend agent handler for producing streams.

This module provides StreamingHandler for backend agents (cognitive agents)
to send streaming responses to clients through the Bridge.

The handler manages:
- Chunk sequencing and correlation
- Statistics tracking
- Proper completion signaling
- Error handling

Example:
    ```python
    from loom.streaming import StreamingHandler, StreamContentType

    async def on_event(ctx, topic, event):
        if event.type == "stream.request":
            handler = StreamingHandler(ctx, event)

            try:
                # Process with streaming LLM
                async for chunk in cognitive.run_stream(event.payload):
                    if isinstance(chunk, str):
                        await handler.send_chunk(chunk)
                    elif hasattr(chunk, "tool_call"):
                        await handler.send_chunk(
                            str(chunk.tool_call),
                            content_type=StreamContentType.TOOL_CALL
                        )

                await handler.send_complete()
            except Exception as e:
                await handler.send_error(str(e))
    ```
"""

from __future__ import annotations

import time
from typing import TYPE_CHECKING, Dict, Optional

from opentelemetry import trace
from opentelemetry.trace import set_span_in_context

from .types import (
    StreamChunk,
    StreamComplete,
    StreamContentType,
    StreamError,
    StreamState,
    StreamStateKind,
    StreamStats,
    StreamStatus,
)

# Tracer for streaming handler spans
tracer = trace.get_tracer(__name__)


class StreamCancelledError(Exception):
    """Raised when a stream is cancelled by the client."""

    def __init__(self, reason: str = ""):
        self.reason = reason
        super().__init__(f"Stream cancelled: {reason}" if reason else "Stream cancelled")


if TYPE_CHECKING:
    from ..agent import EventContext
    from ..agent.envelope import Envelope


class StreamingHandler:
    """Handler for backend agents to produce streaming responses.

    Manages the lifecycle of a streaming response, including:
    - Sequencing chunks
    - Tracking statistics
    - Sending completion signals
    - Error handling

    Attributes:
        ctx: Event context for sending messages
        correlation_id: ID linking all chunks to the original request
        reply_to: Topic to send chunks to
        thread_id: Conversation thread ID
    """

    def __init__(
        self,
        ctx: "EventContext",
        original_event: "Envelope",
        include_state_updates: bool = False,
    ):
        """Initialize streaming handler.

        Args:
            ctx: Event context for sending messages
            original_event: The original request event to respond to
            include_state_updates: Whether to send state notifications
        """
        self.ctx = ctx
        self.correlation_id = original_event.correlation_id or original_event.id
        self.reply_to = original_event.reply_to or f"agent.{original_event.sender}.replies"
        self.thread_id = original_event.thread_id or ""
        self.include_state_updates = include_state_updates

        # Cancellation support
        self._cancelled = False
        self._cancel_reason = ""

        # Extract trace context from incoming request for distributed tracing
        self._parent_context = original_event.extract_trace_context()
        self._span: Optional[trace.Span] = None

        # Tracking
        self._sequence = 0
        self._start_time = time.time()
        self._first_chunk_time: Optional[float] = None
        self._chunks_sent = 0
        self._tool_calls = 0
        self._total_content: list[str] = []
        self._total_tokens = 0

    def _ensure_span(self) -> trace.Span:
        """Ensure we have an active span for this stream."""
        if self._span is None:
            # Create span with parent context from incoming request
            ctx = None
            if self._parent_context:
                parent_span = trace.NonRecordingSpan(self._parent_context)
                ctx = set_span_in_context(parent_span)

            self._span = tracer.start_span(
                "streaming.handler.process",
                context=ctx,
                attributes={
                    "streaming.correlation_id": self.correlation_id,
                    "streaming.reply_to": self.reply_to,
                    "streaming.thread_id": self.thread_id,
                    "streaming.agent_id": self.ctx.agent_id,
                },
            )
        return self._span

    @property
    def is_cancelled(self) -> bool:
        """Check if the stream has been cancelled."""
        return self._cancelled

    def cancel(self, reason: str = "") -> None:
        """Mark the stream as cancelled.

        This should be called when a stream.cancel event is received.

        Args:
            reason: Cancellation reason
        """
        self._cancelled = True
        self._cancel_reason = reason

    def check_cancelled(self) -> None:
        """Check if cancelled and raise CancelledError if so.

        Use this in loops to allow graceful cancellation:

            for chunk in generate_response():
                handler.check_cancelled()
                await handler.send_chunk(chunk)

        Raises:
            StreamCancelledError: If the stream was cancelled
        """
        if self._cancelled:
            raise StreamCancelledError(self._cancel_reason)

    async def send_chunk(
        self,
        content: str,
        content_type: StreamContentType = StreamContentType.TEXT,
        metadata: Optional[Dict[str, str]] = None,
    ) -> None:
        """Send a content chunk to the client.

        Args:
            content: The content to send
            content_type: Type of content (TEXT, THINKING, TOOL_CALL, etc.)
            metadata: Optional metadata for this chunk

        Raises:
            StreamCancelledError: If the stream was cancelled
        """
        # Check for cancellation before sending
        self.check_cancelled()

        # Ensure span is started
        span = self._ensure_span()

        # Track first chunk latency
        if self._first_chunk_time is None:
            self._first_chunk_time = time.time()
            span.add_event("first_chunk", {"content_type": content_type.name})

        chunk = StreamChunk(
            correlation_id=self.correlation_id,
            content=content,
            sequence=self._sequence,
            content_type=content_type,
            metadata=metadata or {},
        )

        # Send via event bus
        await self._emit_chunk(chunk)

        # Update tracking
        self._sequence += 1
        self._chunks_sent += 1
        if content_type == StreamContentType.TEXT:
            self._total_content.append(content)
        if content_type == StreamContentType.TOOL_CALL:
            self._tool_calls += 1

    async def send_thinking(self, content: str) -> None:
        """Send thinking/reasoning content.

        Args:
            content: The thinking content
        """
        await self.send_chunk(content, StreamContentType.THINKING)

    async def send_tool_call(self, tool_name: str, arguments: str) -> None:
        """Send a tool call notification.

        Args:
            tool_name: Name of the tool being called
            arguments: JSON arguments to the tool
        """
        await self.send_chunk(
            f"{tool_name}: {arguments}",
            StreamContentType.TOOL_CALL,
            metadata={"tool_name": tool_name},
        )

    async def send_tool_result(self, tool_name: str, result: str) -> None:
        """Send a tool result notification.

        Args:
            tool_name: Name of the tool
            result: Tool result content
        """
        await self.send_chunk(
            result,
            StreamContentType.TOOL_RESULT,
            metadata={"tool_name": tool_name},
        )

    async def send_status(self, message: str) -> None:
        """Send a status/progress update.

        Args:
            message: Status message
        """
        await self.send_chunk(message, StreamContentType.STATUS)

    async def send_state(self, state: StreamStateKind, message: str = "") -> None:
        """Send a state notification (for monitoring).

        Args:
            state: The current state
            message: Optional state message
        """
        if not self.include_state_updates:
            return

        state_msg = StreamState(
            correlation_id=self.correlation_id,
            state=state,
            message=message,
        )
        await self._emit_state(state_msg)

    async def send_complete(
        self,
        final_content: Optional[str] = None,
        tokens: int = 0,
        iterations: int = 0,
    ) -> None:
        """Send completion signal.

        Args:
            final_content: Optional aggregated final content
            tokens: Total tokens generated (if known)
            iterations: Number of reasoning iterations
        """
        duration_ms = int((time.time() - self._start_time) * 1000)
        first_chunk_latency = 0
        if self._first_chunk_time:
            first_chunk_latency = int((self._first_chunk_time - self._start_time) * 1000)

        # Use provided final_content or aggregate from chunks
        content = final_content if final_content is not None else "".join(self._total_content)

        complete = StreamComplete(
            correlation_id=self.correlation_id,
            final_content=content,
            total_chunks=self._chunks_sent,
            status=StreamStatus.OK,
            stats=StreamStats(
                total_tokens=tokens or self._total_tokens,
                duration_ms=duration_ms,
                tool_calls=self._tool_calls,
                iterations=iterations,
                first_chunk_latency_ms=first_chunk_latency,
            ),
        )
        await self._emit_complete(complete)

        # End span with success metrics
        if self._span:
            self._span.set_attribute("streaming.chunks_sent", self._chunks_sent)
            self._span.set_attribute("streaming.tool_calls", self._tool_calls)
            self._span.set_attribute("streaming.duration_ms", duration_ms)
            self._span.set_attribute("streaming.total_tokens", tokens or self._total_tokens)
            self._span.set_attribute("streaming.iterations", iterations)
            self._span.set_attribute("streaming.first_chunk_latency_ms", first_chunk_latency)
            self._span.set_attribute("streaming.status", "ok")
            self._span.end()

    async def send_error(
        self,
        message: str,
        code: str = "STREAM_ERROR",
        retryable: bool = False,
    ) -> None:
        """Send error completion.

        Args:
            message: Error message
            code: Error code
            retryable: Whether the error is retryable
        """
        # Record error in span
        if self._span:
            self._span.set_attribute("streaming.status", "error")
            self._span.set_attribute("streaming.error_code", code)
            self._span.set_attribute("streaming.error_message", message)
            self._span.set_attribute("streaming.error_retryable", retryable)
        duration_ms = int((time.time() - self._start_time) * 1000)

        complete = StreamComplete(
            correlation_id=self.correlation_id,
            total_chunks=self._chunks_sent,
            status=StreamStatus.ERROR,
            error=StreamError(
                code=code,
                message=message,
                retryable=retryable,
            ),
            stats=StreamStats(
                duration_ms=duration_ms,
                tool_calls=self._tool_calls,
            ),
        )
        await self._emit_complete(complete)

        # End span with error status
        if self._span:
            self._span.set_attribute("streaming.duration_ms", duration_ms)
            from opentelemetry.trace import Status, StatusCode

            self._span.set_status(Status(StatusCode.ERROR, message))
            self._span.end()

    async def send_cancelled(self, reason: str = "") -> None:
        """Send cancellation completion.

        Args:
            reason: Cancellation reason
        """
        # Record cancellation in span
        if self._span:
            self._span.set_attribute("streaming.status", "cancelled")
            self._span.set_attribute("streaming.cancel_reason", reason)

        complete = StreamComplete(
            correlation_id=self.correlation_id,
            total_chunks=self._chunks_sent,
            status=StreamStatus.CANCELLED,
            error=StreamError(
                code="CANCELLED",
                message=reason or "Stream cancelled by client",
                retryable=False,
            ),
        )
        await self._emit_complete(complete)

        # End span with cancelled status
        if self._span:
            self._span.set_attribute(
                "streaming.duration_ms", int((time.time() - self._start_time) * 1000)
            )
            self._span.end()

    # Private methods for emitting proto messages
    async def _emit_chunk(self, chunk: StreamChunk) -> None:
        """Emit a chunk via the event bus."""
        from ..agent.envelope import Envelope

        # Create envelope with streaming metadata
        env = Envelope.new(
            type="stream.chunk",
            payload=chunk.content.encode("utf-8"),
            sender=self.ctx.agent_id,
            correlation_id=self.correlation_id,
            thread_id=self.thread_id,
            metadata={
                "stream.sequence": str(chunk.sequence),
                "stream.content_type": str(chunk.content_type.value),
                **chunk.metadata,
            },
        )

        await self.ctx.emit(
            self.reply_to,
            type="stream.chunk",
            payload=chunk.content.encode("utf-8"),
            envelope=env,
        )

    async def _emit_complete(self, complete: StreamComplete) -> None:
        """Emit completion signal via the event bus."""

        from ..agent.envelope import Envelope

        # Serialize stats/error to metadata
        metadata = {
            "stream.status": str(complete.status.value),
            "stream.total_chunks": str(complete.total_chunks),
        }
        if complete.stats:
            metadata["stream.duration_ms"] = str(complete.stats.duration_ms)
            metadata["stream.tool_calls"] = str(complete.stats.tool_calls)
            metadata["stream.first_chunk_latency_ms"] = str(complete.stats.first_chunk_latency_ms)
        if complete.error:
            metadata["stream.error_code"] = complete.error.code
            metadata["stream.error_message"] = complete.error.message

        env = Envelope.new(
            type="stream.complete",
            payload=complete.final_content.encode("utf-8"),
            sender=self.ctx.agent_id,
            correlation_id=self.correlation_id,
            thread_id=self.thread_id,
            metadata=metadata,
        )

        await self.ctx.emit(
            self.reply_to,
            type="stream.complete",
            payload=complete.final_content.encode("utf-8"),
            envelope=env,
        )

    async def _emit_state(self, state: StreamState) -> None:
        """Emit state notification via the event bus."""
        from ..agent.envelope import Envelope

        env = Envelope.new(
            type="stream.state",
            payload=state.message.encode("utf-8"),
            sender=self.ctx.agent_id,
            correlation_id=self.correlation_id,
            thread_id=self.thread_id,
            metadata={
                "stream.state": str(state.state.value),
            },
        )

        await self.ctx.emit(
            self.reply_to,
            type="stream.state",
            payload=state.message.encode("utf-8"),
            envelope=env,
        )
