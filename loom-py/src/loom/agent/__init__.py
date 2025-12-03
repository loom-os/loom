"""Agent module - Core Agent functionality.

This module provides the primary agent classes:
- Agent: Main class for connecting to Loom Runtime
- Context: Agent's interface to Rust Core
- Envelope: Message wrapper for event communication
"""

from .base import Agent
from .context import Context
from .envelope import Envelope

__all__ = [
    "Agent",
    "Context",
    "Envelope",
]
