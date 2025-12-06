"""Step types for Context Engineering.

This module defines the core data structures for context reduction:
- Step: A reduced representation of a tool execution
- CompactStep: Ultra-minimal representation for prompt inclusion

These are generated from cognitive.types.ThoughtStep after applying
tool-specific reduction rules.
"""

from __future__ import annotations

import hashlib
import time
from dataclasses import dataclass, field
from typing import Any, Optional


@dataclass
class Step:
    """A single cognitive step with reduction applied.

    This is the "reduced" form of a tool execution, keeping only what's
    needed to reconstruct or reference the full result.

    Attributes:
        id: Unique step identifier (e.g., "step_001")
        tool_name: Name of the tool executed (e.g., "fs:read_file")
        minimal_args: Reduced arguments (large content removed)
        observation: One-line result summary
        outcome_ref: Path to offloaded full output (if any)
        timestamp_ms: When this step was executed
        success: Whether the tool execution succeeded
        error: Error message if failed
        metadata: Additional tool-specific metadata
    """

    id: str
    tool_name: str
    minimal_args: dict[str, Any]
    observation: str
    success: bool
    timestamp_ms: int = field(default_factory=lambda: int(time.time() * 1000))
    outcome_ref: Optional[str] = None
    error: Optional[str] = None
    metadata: dict[str, Any] = field(default_factory=dict)

    def to_compact(self) -> CompactStep:
        """Convert to ultra-minimal CompactStep."""
        return CompactStep(id=self.id, summary=self.observation)

    def to_dict(self) -> dict[str, Any]:
        """Serialize to dictionary for storage."""
        return {
            "id": self.id,
            "tool_name": self.tool_name,
            "minimal_args": self.minimal_args,
            "observation": self.observation,
            "success": self.success,
            "timestamp_ms": self.timestamp_ms,
            "outcome_ref": self.outcome_ref,
            "error": self.error,
            "metadata": self.metadata,
        }

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> Step:
        """Deserialize from dictionary."""
        return cls(
            id=data["id"],
            tool_name=data["tool_name"],
            minimal_args=data.get("minimal_args", {}),
            observation=data.get("observation", ""),
            success=data.get("success", True),
            timestamp_ms=data.get("timestamp_ms", 0),
            outcome_ref=data.get("outcome_ref"),
            error=data.get("error"),
            metadata=data.get("metadata", {}),
        )


@dataclass
class CompactStep:
    """Ultra-minimal step representation for prompt inclusion.

    Used when compacting old steps to save tokens. Contains only
    what's needed to remind the LLM what happened.

    Attributes:
        id: Step identifier for cross-reference
        summary: One-line description (e.g., "Read config.json (1.2KB)")
    """

    id: str
    summary: str

    def __str__(self) -> str:
        return f"• {self.summary}"


def generate_step_id(counter: int) -> str:
    """Generate a step ID from counter.

    Args:
        counter: Step number (1-based)

    Returns:
        Formatted step ID like "step_001"
    """
    return f"step_{counter:03d}"


def compute_content_hash(content: str) -> str:
    """Compute SHA256 hash of content for deduplication.

    Args:
        content: Content to hash

    Returns:
        First 16 chars of hex digest
    """
    return hashlib.sha256(content.encode()).hexdigest()[:16]


__all__ = [
    "Step",
    "CompactStep",
    "generate_step_id",
    "compute_content_hash",
]
