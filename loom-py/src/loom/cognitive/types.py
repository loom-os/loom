"""Cognitive types - Data structures for the cognitive loop.

This module contains the core data types used in cognitive processing:
- ToolCall: Represents a tool invocation request
- Observation: Result of tool execution
- ThoughtStep: A single reasoning step
- CognitiveResult: Final result of a cognitive run
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any, Optional


@dataclass
class ToolCall:
    """A tool call to be executed."""

    name: str
    arguments: dict[str, Any]

    def to_dict(self) -> dict:
        return {"tool": self.name, "args": self.arguments}


@dataclass
class Observation:
    """Result of a tool execution."""

    tool_name: str
    success: bool
    output: str
    error: Optional[str] = None
    latency_ms: int = 0


@dataclass
class ThoughtStep:
    """A single step in the reasoning process."""

    step: int
    reasoning: str
    tool_call: Optional[ToolCall] = None
    observation: Optional[Observation] = None


@dataclass
class CognitiveResult:
    """Result of a cognitive loop execution."""

    answer: str
    steps: list[ThoughtStep] = field(default_factory=list)
    iterations: int = 0
    success: bool = True
    error: Optional[str] = None
    total_latency_ms: int = 0


__all__ = [
    "ToolCall",
    "Observation",
    "ThoughtStep",
    "CognitiveResult",
]
