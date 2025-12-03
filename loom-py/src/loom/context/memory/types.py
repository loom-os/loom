"""Memory types - Data structures for memory management.

This module defines types used in memory management:
- MemoryItem: A single memory entry
- MemoryTier: Classification of memory persistence
"""

from __future__ import annotations

from dataclasses import dataclass, field
from enum import Enum
from typing import Any, Optional


class MemoryTier(Enum):
    """Memory tier classification (from ROADMAP).

    - WORKING: Current task context (cleared per run)
    - SHORT_TERM: Session-scoped (~1 hour)
    - LONG_TERM: Persistent across restarts (RocksDB)
    """

    WORKING = "working"
    SHORT_TERM = "short_term"
    LONG_TERM = "long_term"


@dataclass
class MemoryItem:
    """A single memory entry.

    Attributes:
        role: Message role (user, assistant, system, tool)
        content: Main content
        timestamp_ms: When this was created
        metadata: Additional structured data
        tier: Memory persistence tier
        embedding: Optional vector embedding for retrieval
    """

    role: str
    content: str
    timestamp_ms: int = 0
    metadata: dict[str, Any] = field(default_factory=dict)
    tier: MemoryTier = MemoryTier.WORKING
    embedding: Optional[list[float]] = None

    def to_message(self) -> dict[str, str]:
        """Convert to chat message format."""
        return {"role": self.role, "content": self.content}


__all__ = [
    "MemoryItem",
    "MemoryTier",
]
