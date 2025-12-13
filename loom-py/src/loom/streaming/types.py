"""Streaming Types - Python dataclasses mirroring protobuf definitions.

These types provide a Pythonic interface to the streaming protocol
defined in loom-proto/proto/streaming.proto.
"""

from __future__ import annotations

import time
import uuid
from dataclasses import dataclass, field
from enum import IntEnum
from typing import Any, Dict, List, Optional


class StreamContentType(IntEnum):
    """Content type for stream chunks."""

    TEXT = 0  # Default text content (LLM response)
    THINKING = 1  # Thinking/reasoning content (for CoT display)
    TOOL_CALL = 2  # Tool call being made
    TOOL_RESULT = 3  # Tool result returned
    ERROR = 4  # Error content
    STATUS = 5  # Status/progress update
    MARKDOWN = 6  # Markdown formatted content
    CODE = 7  # Code block content
    PERMISSION_REQUEST = 8  # Permission request from backend
    PERMISSION_RESPONSE = 9  # Permission response from client


class StreamStatus(IntEnum):
    """Stream completion status."""

    OK = 0  # Successfully completed
    CANCELLED = 1  # Cancelled by client
    TIMEOUT = 2  # Timed out
    ERROR = 3  # Error occurred
    TRUNCATED = 4  # Completed but truncated (hit token limit)


class StreamStateKind(IntEnum):
    """Stream state kinds for monitoring."""

    STARTED = 0  # Stream started
    PROCESSING = 1  # Processing input
    GENERATING = 2  # Generating response
    TOOL_EXECUTING = 3  # Executing tool
    WAITING = 4  # Waiting for input (e.g., permission)
    COMPLETED = 5  # Completed
    FAILED = 6  # Failed


@dataclass
class StreamError:
    """Error details for streaming."""

    code: str
    message: str
    retryable: bool = False
    details: Dict[str, str] = field(default_factory=dict)

    def to_proto(self, proto_class: Any) -> Any:
        """Convert to protobuf message."""
        return proto_class(
            code=self.code,
            message=self.message,
            retryable=self.retryable,
            details=self.details,
        )

    @classmethod
    def from_proto(cls, proto: Any) -> "StreamError":
        """Create from protobuf message."""
        return cls(
            code=proto.code,
            message=proto.message,
            retryable=proto.retryable,
            details=dict(proto.details),
        )


@dataclass
class StreamStats:
    """Execution statistics for completed streams."""

    total_tokens: int = 0
    duration_ms: int = 0
    tool_calls: int = 0
    iterations: int = 0
    first_chunk_latency_ms: int = 0  # Time to first token

    def to_proto(self, proto_class: Any) -> Any:
        """Convert to protobuf message."""
        return proto_class(
            total_tokens=self.total_tokens,
            duration_ms=self.duration_ms,
            tool_calls=self.tool_calls,
            iterations=self.iterations,
            first_chunk_latency_ms=self.first_chunk_latency_ms,
        )

    @classmethod
    def from_proto(cls, proto: Any) -> "StreamStats":
        """Create from protobuf message."""
        return cls(
            total_tokens=proto.total_tokens,
            duration_ms=proto.duration_ms,
            tool_calls=proto.tool_calls,
            iterations=proto.iterations,
            first_chunk_latency_ms=proto.first_chunk_latency_ms,
        )


@dataclass
class PermissionRequest:
    """Request for user permission before executing a tool."""

    request_id: str  # Unique ID for this permission request
    tool_name: str  # Tool requesting permission
    tool_args: Dict[str, Any]  # Arguments the tool will be called with
    reason: str  # Human-readable reason for the request

    def to_json(self) -> str:
        """Serialize to JSON string."""
        import json

        return json.dumps(
            {
                "request_id": self.request_id,
                "tool_name": self.tool_name,
                "tool_args": self.tool_args,
                "reason": self.reason,
            }
        )

    @classmethod
    def from_json(cls, data: str) -> "PermissionRequest":
        """Deserialize from JSON string."""
        import json

        d = json.loads(data)
        return cls(
            request_id=d["request_id"],
            tool_name=d["tool_name"],
            tool_args=d.get("tool_args", {}),
            reason=d.get("reason", ""),
        )


@dataclass
class PermissionResponse:
    """Response to a permission request."""

    request_id: str  # Must match the request_id from PermissionRequest
    approved: bool  # True if user approved, False if denied

    def to_json(self) -> str:
        """Serialize to JSON string."""
        import json

        return json.dumps(
            {
                "request_id": self.request_id,
                "approved": self.approved,
            }
        )

    @classmethod
    def from_json(cls, data: str) -> "PermissionResponse":
        """Deserialize from JSON string."""
        import json

        d = json.loads(data)
        return cls(
            request_id=d["request_id"],
            approved=d.get("approved", False),
        )


@dataclass
class StreamChunk:
    """A chunk of streaming content from an LLM or agent response."""

    correlation_id: str
    content: str
    sequence: int = 0
    content_type: StreamContentType = StreamContentType.TEXT
    metadata: Dict[str, str] = field(default_factory=dict)
    timestamp_ms: int = field(default_factory=lambda: int(time.time() * 1000))

    def to_proto(self, proto_class: Any) -> Any:
        """Convert to protobuf message."""
        return proto_class(
            correlation_id=self.correlation_id,
            content=self.content,
            sequence=self.sequence,
            content_type=self.content_type.value,
            metadata=self.metadata,
            timestamp_ms=self.timestamp_ms,
        )

    @classmethod
    def from_proto(cls, proto: Any) -> "StreamChunk":
        """Create from protobuf message."""
        return cls(
            correlation_id=proto.correlation_id,
            content=proto.content,
            sequence=proto.sequence,
            content_type=StreamContentType(proto.content_type),
            metadata=dict(proto.metadata),
            timestamp_ms=proto.timestamp_ms,
        )


@dataclass
class StreamComplete:
    """Signals completion of a streaming response."""

    correlation_id: str
    final_content: str = ""
    total_chunks: int = 0
    status: StreamStatus = StreamStatus.OK
    error: Optional[StreamError] = None
    stats: Optional[StreamStats] = None

    def to_proto(self, proto_class: Any, error_class: Any, stats_class: Any) -> Any:
        """Convert to protobuf message."""
        return proto_class(
            correlation_id=self.correlation_id,
            final_content=self.final_content,
            total_chunks=self.total_chunks,
            status=self.status.value,
            error=self.error.to_proto(error_class) if self.error else None,
            stats=self.stats.to_proto(stats_class) if self.stats else None,
        )

    @classmethod
    def from_proto(cls, proto: Any) -> "StreamComplete":
        """Create from protobuf message."""
        return cls(
            correlation_id=proto.correlation_id,
            final_content=proto.final_content,
            total_chunks=proto.total_chunks,
            status=StreamStatus(proto.status),
            error=StreamError.from_proto(proto.error) if proto.HasField("error") else None,
            stats=StreamStats.from_proto(proto.stats) if proto.HasField("stats") else None,
        )


@dataclass
class StreamOptions:
    """Options for streaming requests."""

    max_tokens: int = 0  # 0 = default
    include_thinking: bool = False
    include_tool_calls: bool = True
    timeout_ms: int = 0  # 0 = default
    content_types: List[StreamContentType] = field(default_factory=list)

    def to_proto(self, proto_class: Any) -> Any:
        """Convert to protobuf message."""
        return proto_class(
            max_tokens=self.max_tokens,
            include_thinking=self.include_thinking,
            include_tool_calls=self.include_tool_calls,
            timeout_ms=self.timeout_ms,
            content_types=[ct.value for ct in self.content_types],
        )

    @classmethod
    def from_proto(cls, proto: Any) -> "StreamOptions":
        """Create from protobuf message."""
        return cls(
            max_tokens=proto.max_tokens,
            include_thinking=proto.include_thinking,
            include_tool_calls=proto.include_tool_calls,
            timeout_ms=proto.timeout_ms,
            content_types=[StreamContentType(ct) for ct in proto.content_types],
        )


@dataclass
class StreamRequest:
    """Request to start a streaming conversation."""

    id: str = field(default_factory=lambda: str(uuid.uuid4()))
    message: str = ""
    thread_id: str = ""
    sender: str = ""
    reply_to: str = ""
    context: List[str] = field(default_factory=list)
    options: Optional[StreamOptions] = None

    def to_proto(self, proto_class: Any, options_class: Any) -> Any:
        """Convert to protobuf message."""
        return proto_class(
            id=self.id,
            message=self.message,
            thread_id=self.thread_id,
            sender=self.sender,
            reply_to=self.reply_to,
            context=self.context,
            options=self.options.to_proto(options_class) if self.options else None,
        )

    @classmethod
    def from_proto(cls, proto: Any) -> "StreamRequest":
        """Create from protobuf message."""
        return cls(
            id=proto.id,
            message=proto.message,
            thread_id=proto.thread_id,
            sender=proto.sender,
            reply_to=proto.reply_to,
            context=list(proto.context),
            options=StreamOptions.from_proto(proto.options) if proto.HasField("options") else None,
        )


@dataclass
class StreamState:
    """Stream state notification (for monitoring)."""

    correlation_id: str
    state: StreamStateKind
    message: str = ""
    timestamp_ms: int = field(default_factory=lambda: int(time.time() * 1000))

    def to_proto(self, proto_class: Any) -> Any:
        """Convert to protobuf message."""
        return proto_class(
            correlation_id=self.correlation_id,
            state=self.state.value,
            message=self.message,
            timestamp_ms=self.timestamp_ms,
        )

    @classmethod
    def from_proto(cls, proto: Any) -> "StreamState":
        """Create from protobuf message."""
        return cls(
            correlation_id=proto.correlation_id,
            state=StreamStateKind(proto.state),
            message=proto.message,
            timestamp_ms=proto.timestamp_ms,
        )
