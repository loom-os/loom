from __future__ import annotations

import time
import uuid
from dataclasses import dataclass, field
from typing import Any, Dict, Optional

from opentelemetry.trace import SpanContext, TraceFlags, TraceState, get_current_span

META_PREFIX = "loom"


@dataclass
class Envelope:
    id: str
    type: str
    timestamp_ms: int
    source: str
    payload: bytes
    metadata: Dict[str, str] = field(default_factory=dict)
    tags: list[str] = field(default_factory=list)
    priority: int = 50
    # Extended fields (Roadmap unified envelope)
    thread_id: Optional[str] = None
    correlation_id: Optional[str] = None
    sender: Optional[str] = None
    reply_to: Optional[str] = None
    ttl_ms: Optional[int] = None
    # OpenTelemetry trace context for distributed tracing
    trace_id: Optional[str] = None
    span_id: Optional[str] = None
    trace_flags: Optional[str] = None

    @classmethod
    def new(
        cls,
        type: str,
        payload: bytes = b"",
        source: str = "python",
        thread_id: Optional[str] = None,
        correlation_id: Optional[str] = None,
        sender: Optional[str] = None,
        reply_to: Optional[str] = None,
        ttl_ms: Optional[int] = None,
        metadata: Optional[Dict[str, str]] = None,
    ) -> Envelope:
        now = int(time.time() * 1000)
        # Use random UUID for envelope id to avoid collisions across processes
        eid = str(uuid.uuid4())
        meta = metadata.copy() if metadata else {}

        def set_opt(key: str, value: Optional[str | int]):
            if value is not None:
                meta[f"{META_PREFIX}.{key}"] = str(value)

        set_opt("thread_id", thread_id)
        set_opt("correlation_id", correlation_id)
        set_opt("sender", sender)
        set_opt("reply_to", reply_to)
        set_opt("ttl_ms", ttl_ms)
        return cls(
            id=eid,
            type=type,
            timestamp_ms=now,
            source=source,
            payload=payload,
            metadata=meta,
            thread_id=thread_id,
            correlation_id=correlation_id,
            sender=sender,
            reply_to=reply_to,
            ttl_ms=ttl_ms,
        )

    @classmethod
    def from_proto(cls, ev) -> Envelope:  # ev is loom.v1.Event
        meta = dict(ev.metadata)

        def get_opt(key: str) -> Optional[str]:
            return meta.get(key)

        return cls(
            id=ev.id,
            type=ev.type,
            timestamp_ms=ev.timestamp_ms,
            source=ev.source,
            payload=ev.payload,
            metadata=meta,
            tags=list(ev.tags),
            priority=ev.priority,
            thread_id=get_opt("thread_id"),
            correlation_id=get_opt("correlation_id"),
            sender=get_opt("sender"),
            reply_to=get_opt("reply_to"),
            ttl_ms=int(get_opt("ttl")) if get_opt("ttl") is not None else None,
            trace_id=get_opt("trace_id"),
            span_id=get_opt("span_id"),
            trace_flags=get_opt("trace_flags"),
        )

    def to_proto(self, pb_event_cls) -> Any:
        # pb_event_cls: generated protobuf Event class
        ev = pb_event_cls(
            id=self.id,
            type=self.type,
            timestamp_ms=self.timestamp_ms,
            source=self.source,
            metadata=self.metadata,
            payload=self.payload,
            confidence=1.0,
            tags=self.tags,
            priority=self.priority,
        )
        return ev

    def inject_trace_context(self) -> None:
        """Inject current OpenTelemetry trace context into envelope metadata.

        Extracts trace_id, span_id, and trace_flags from the current span
        and stores them in the envelope for propagation across process boundaries.
        """
        span = get_current_span()
        if span and span.get_span_context().is_valid:
            ctx = span.get_span_context()
            self.trace_id = format(ctx.trace_id, "032x")
            self.span_id = format(ctx.span_id, "016x")
            self.trace_flags = format(ctx.trace_flags, "02x")
            # Also store in metadata for Rust side
            self.metadata["trace_id"] = self.trace_id
            self.metadata["span_id"] = self.span_id
            self.metadata["trace_flags"] = self.trace_flags

    def extract_trace_context(self) -> Optional[SpanContext]:
        """Extract OpenTelemetry trace context from envelope metadata.

        Parses trace_id, span_id, and trace_flags from the envelope and creates
        a remote parent span context. This enables distributed tracing across
        process boundaries.

        Returns the SpanContext if valid, otherwise None.
        """
        if not self.trace_id or not self.span_id:
            return None

        try:
            trace_id = int(self.trace_id, 16)
            span_id = int(self.span_id, 16)
            trace_flags_int = int(self.trace_flags or "00", 16)

            return SpanContext(
                trace_id=trace_id,
                span_id=span_id,
                is_remote=True,
                trace_flags=TraceFlags(trace_flags_int),
                trace_state=TraceState(),
            )
        except (ValueError, TypeError):
            return None
