"""Agent module - Core Agent functionality.

This module provides the primary agent classes:
- Agent: Main class for connecting to Loom Runtime
- EventContext: Agent's interface to Rust Core Event Bus
- Envelope: Message wrapper for event communication
"""

from .base import Agent
from .envelope import Envelope
from .event import EventContext

# Backward compatibility alias
Context = EventContext

__all__ = [
    "Agent",
    "EventContext",
    "Context",  # Backward compatibility
    "Envelope",
]
