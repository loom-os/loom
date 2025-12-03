"""Tools module - Tool definition and management.

This module provides tool functionality for Loom agents:
- Tool: Class representing a tool with metadata
- @tool: Decorator for declaring Python functions as tools

Tools are the "Hands" that agents use to interact with the world.
In the Brain/Hand separation, tools execute in Rust Core (sandboxed),
while Python agents define and declare them.
"""

from .decorator import Capability, Tool, capability, tool

__all__ = [
    "Tool",
    "tool",
    # Backwards compatibility (deprecated)
    "Capability",
    "capability",
]
