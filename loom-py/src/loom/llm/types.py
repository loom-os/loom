"""LLM Types - Data structures for LLM interactions.

This module defines types used in LLM communication.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any, Optional


@dataclass
class Message:
    """A chat message.

    Attributes:
        role: Message role (system, user, assistant)
        content: Message content
        name: Optional name for the message sender
    """

    role: str
    content: str
    name: Optional[str] = None

    def to_dict(self) -> dict[str, str]:
        """Convert to API format."""
        d = {"role": self.role, "content": self.content}
        if self.name:
            d["name"] = self.name
        return d


@dataclass
class LLMResponse:
    """Response from an LLM call.

    Attributes:
        content: Generated text content
        model: Model that generated the response
        usage: Token usage statistics
        finish_reason: Why generation stopped
        raw: Raw API response
    """

    content: str
    model: Optional[str] = None
    usage: dict[str, int] = field(default_factory=dict)
    finish_reason: Optional[str] = None
    raw: Optional[dict[str, Any]] = None


__all__ = [
    "Message",
    "LLMResponse",
]
