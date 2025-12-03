"""Cognitive configuration - Settings for cognitive loop behavior.

This module defines configuration options for cognitive agents:
- ThinkingStrategy: How the agent reasons (single-shot, ReAct, CoT)
- CognitiveConfig: All cognitive loop parameters
"""

from __future__ import annotations

from dataclasses import dataclass
from enum import Enum
from typing import Optional


class ThinkingStrategy(Enum):
    """Strategy for cognitive processing.

    - SINGLE_SHOT: One LLM call, no tools
    - REACT: ReAct pattern (Thought -> Action -> Observation loop)
    - CHAIN_OF_THOUGHT: Step-by-step reasoning without tools
    """

    SINGLE_SHOT = "single_shot"
    REACT = "react"
    CHAIN_OF_THOUGHT = "cot"


@dataclass
class CognitiveConfig:
    """Configuration for cognitive loop.

    Attributes:
        system_prompt: Custom system prompt (optional, has defaults per strategy)
        thinking_strategy: Which reasoning strategy to use
        max_iterations: Maximum ReAct iterations before giving up
        max_tools_per_step: Maximum tool calls per iteration
        temperature: LLM temperature setting
        stop_on_final_answer: Whether to stop when "FINAL ANSWER" is detected
    """

    system_prompt: Optional[str] = None
    thinking_strategy: ThinkingStrategy = ThinkingStrategy.REACT
    max_iterations: int = 10
    max_tools_per_step: int = 3
    temperature: float = 0.7
    stop_on_final_answer: bool = True


__all__ = [
    "ThinkingStrategy",
    "CognitiveConfig",
]
