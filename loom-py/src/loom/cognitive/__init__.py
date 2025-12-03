"""Cognitive module - Brain 🧠 for Python agents.

This module implements the cognitive loop pattern for autonomous agent reasoning:
- CognitiveAgent: Main class for perceive-think-act loops
- CognitiveConfig: Configuration for cognitive behavior
- ThinkingStrategy: Reasoning strategies (ReAct, CoT, single-shot)

The cognitive module is the "Brain" in Loom's Brain/Hand separation:
- Brain (Python): LLM calls, reasoning, context engineering
- Hands (Rust Core): Tool execution, event bus, persistence
"""

# Re-export WorkingMemory for convenience (lives in context.memory)
from ..context.memory import WorkingMemory
from .agent import CognitiveAgent
from .config import CognitiveConfig, ThinkingStrategy
from .types import CognitiveResult, Observation, ThoughtStep, ToolCall

__all__ = [
    # Main class
    "CognitiveAgent",
    # Configuration
    "CognitiveConfig",
    "ThinkingStrategy",
    # Result types
    "CognitiveResult",
    "ToolCall",
    "Observation",
    "ThoughtStep",
    # Memory (re-export)
    "WorkingMemory",
]
