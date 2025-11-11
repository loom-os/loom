from __future__ import annotations
from dataclasses import dataclass, field
from typing import Any, Dict, Optional
import time
import uuid

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
    ) -> "Envelope":
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
        )

    @classmethod
    def from_proto(cls, ev) -> "Envelope":  # ev is loom.v1.Event
        meta = dict(ev.metadata)
        def get_opt(key: str) -> Optional[str]:
            return meta.get(f"{META_PREFIX}.{key}")
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
            ttl_ms=int(get_opt("ttl_ms")) if get_opt("ttl_ms") else None,
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
