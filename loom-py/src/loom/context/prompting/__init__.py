"""Prompting - Prompt construction and tool metadata.

This submodule provides prompt building and tool description utilities:
- **builder**: Assemble prompts from context and memory
- **few_shot**: Example-based learning
- **tool_descriptor**: Tool metadata and registry
"""

from .builder import ContextBuilder, ContextWindow
from .few_shot import FewShotExample, FewShotLibrary, get_default_library
from .tool_descriptor import (
    ToolDescriptor,
    ToolParameter,
    ToolRegistry,
    create_default_registry,
)

__all__ = [
    # Builder
    "ContextBuilder",
    "ContextWindow",
    # Few-shot Examples
    "FewShotExample",
    "FewShotLibrary",
    "get_default_library",
    # Tool Descriptor
    "ToolDescriptor",
    "ToolParameter",
    "ToolRegistry",
    "create_default_registry",
]
