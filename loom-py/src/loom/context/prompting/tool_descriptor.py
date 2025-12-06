"""Tool descriptor - Enhanced tool representation with parameters and examples.

This module provides structured tool descriptions that help LLMs understand
tool capabilities, parameters, and usage patterns.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any, Optional


@dataclass
class ToolParameter:
    """Describes a tool parameter."""

    name: str
    type: str  # "string", "number", "boolean", "object", "array"
    description: str
    required: bool = False
    default: Optional[Any] = None
    examples: list[Any] = field(default_factory=list)

    def to_string(self) -> str:
        """Format parameter for system prompt."""
        parts = [f"{self.name}: {self.type}"]
        if self.required:
            parts[0] += " (required)"
        if self.description:
            parts.append(f"- {self.description}")
        if self.default is not None:
            parts.append(f"- default: {self.default}")
        if self.examples:
            examples_str = ", ".join(str(ex) for ex in self.examples[:2])
            parts.append(f"- e.g. {examples_str}")
        return " ".join(parts)


@dataclass
class ToolDescriptor:
    """Complete tool description with parameters and usage examples."""

    name: str
    description: str
    parameters: list[ToolParameter] = field(default_factory=list)
    examples: list[str] = field(default_factory=list)  # Full usage examples
    category: Optional[str] = None  # "filesystem", "shell", "web", etc.

    def get_signature(self) -> str:
        """Get tool signature: name(param1, param2, ...)"""
        if not self.parameters:
            return f"{self.name}()"

        param_names = []
        for param in self.parameters:
            if param.required:
                param_names.append(param.name)
            else:
                param_names.append(f"{param.name}?")

        return f"{self.name}({', '.join(param_names)})"

    def to_compact_string(self) -> str:
        """Format as single-line compact description."""
        sig = self.get_signature()
        return f"{sig} - {self.description}"

    def to_detailed_string(self) -> str:
        """Format as multi-line detailed description."""
        lines = [
            f"Tool: {self.name}",
            f"Description: {self.description}",
        ]

        if self.parameters:
            lines.append("Parameters:")
            for param in self.parameters:
                lines.append(f"  • {param.to_string()}")

        if self.examples:
            lines.append("Examples:")
            for example in self.examples[:2]:  # Limit to 2 examples
                lines.append(f"  {example}")

        return "\n".join(lines)

    @staticmethod
    def from_simple_name(name: str, description: str = "") -> ToolDescriptor:
        """Create minimal descriptor from just a name."""
        return ToolDescriptor(
            name=name,
            description=description or f"Execute {name}",
        )


class ToolRegistry:
    """Registry for tool descriptors with discovery and formatting."""

    def __init__(self):
        self._tools: dict[str, ToolDescriptor] = {}
        self._categories: dict[str, list[str]] = {}  # category -> tool names

    def register(self, descriptor: ToolDescriptor) -> None:
        """Register a tool descriptor."""
        self._tools[descriptor.name] = descriptor

        if descriptor.category:
            if descriptor.category not in self._categories:
                self._categories[descriptor.category] = []
            self._categories[descriptor.category].append(descriptor.name)

    def register_simple(self, name: str, description: str = "") -> None:
        """Register a simple tool with just name and description."""
        descriptor = ToolDescriptor.from_simple_name(name, description)
        self.register(descriptor)

    def get(self, name: str) -> Optional[ToolDescriptor]:
        """Get descriptor by name."""
        return self._tools.get(name)

    def get_all(self) -> list[ToolDescriptor]:
        """Get all registered descriptors."""
        return list(self._tools.values())

    def get_by_category(self, category: str) -> list[ToolDescriptor]:
        """Get all tools in a category."""
        tool_names = self._categories.get(category, [])
        return [self._tools[name] for name in tool_names if name in self._tools]

    def format_for_prompt(
        self,
        tool_names: Optional[list[str]] = None,
        detailed: bool = False,
        group_by_category: bool = False,
    ) -> str:
        """Format tools for system prompt.

        Args:
            tool_names: Specific tools to include (None = all)
            detailed: Use detailed format vs compact
            group_by_category: Group tools by category

        Returns:
            Formatted string for system prompt
        """
        if tool_names is None:
            descriptors = self.get_all()
        else:
            descriptors = [
                self._tools.get(name) or ToolDescriptor.from_simple_name(name)
                for name in tool_names
            ]

        if not descriptors:
            return "No tools available."

        if group_by_category and not detailed:
            # Group by category in compact format
            categories: dict[str, list[ToolDescriptor]] = {}
            for desc in descriptors:
                cat = desc.category or "general"
                if cat not in categories:
                    categories[cat] = []
                categories[cat].append(desc)

            lines = []
            for cat, tools in sorted(categories.items()):
                lines.append(f"{cat.upper()}:")
                for tool in tools:
                    lines.append(f"  • {tool.to_compact_string()}")
            return "\n".join(lines)

        # Simple list format
        if detailed:
            return "\n\n".join(desc.to_detailed_string() for desc in descriptors)
        else:
            return "\n".join(f"• {desc.to_compact_string()}" for desc in descriptors)


def create_default_registry() -> ToolRegistry:
    """Create a registry with common tool descriptors."""
    registry = ToolRegistry()

    # Filesystem tools
    registry.register(
        ToolDescriptor(
            name="fs:read_file",
            description="Read contents of a file",
            category="filesystem",
            parameters=[
                ToolParameter(
                    "path",
                    "string",
                    "Path to the file",
                    required=True,
                    examples=["/home/user/data.txt"],
                ),
                ToolParameter("start_line", "number", "Starting line number", default=1),
                ToolParameter(
                    "end_line", "number", "Ending line number (or -1 for EOF)", default=-1
                ),
            ],
            examples=[
                '{"tool": "fs:read_file", "args": {"path": "/etc/config.json"}}',
                '{"tool": "fs:read_file", "args": {"path": "README.md", "start_line": 1, "end_line": 50}}',
            ],
        )
    )

    registry.register(
        ToolDescriptor(
            name="fs:write_file",
            description="Write content to a file",
            category="filesystem",
            parameters=[
                ToolParameter("path", "string", "Path to the file", required=True),
                ToolParameter("content", "string", "Content to write", required=True),
                ToolParameter("append", "boolean", "Append instead of overwrite", default=False),
            ],
            examples=[
                '{"tool": "fs:write_file", "args": {"path": "/tmp/result.txt", "content": "Hello World"}}',
            ],
        )
    )

    registry.register(
        ToolDescriptor(
            name="fs:search",
            description="Search for text patterns in files",
            category="filesystem",
            parameters=[
                ToolParameter("query", "string", "Text pattern to search", required=True),
                ToolParameter("path", "string", "Directory or file to search in", default="."),
                ToolParameter(
                    "include",
                    "string",
                    "File pattern to include (glob)",
                    examples=["*.py", "**/*.js"],
                ),
            ],
            examples=[
                '{"tool": "fs:search", "args": {"query": "TODO", "path": "src/"}}',
            ],
        )
    )

    # Shell tools
    registry.register(
        ToolDescriptor(
            name="shell:run",
            description="Execute a shell command",
            category="shell",
            parameters=[
                ToolParameter(
                    "command",
                    "string",
                    "Command to execute",
                    required=True,
                    examples=["ls -la", "npm install"],
                ),
                ToolParameter("cwd", "string", "Working directory", default="."),
                ToolParameter("timeout", "number", "Timeout in seconds", default=30),
            ],
            examples=[
                '{"tool": "shell:run", "args": {"command": "git status"}}',
                '{"tool": "shell:run", "args": {"command": "pytest tests/", "cwd": "/project"}}',
            ],
        )
    )

    # Web tools
    registry.register(
        ToolDescriptor(
            name="web:fetch",
            description="Fetch content from a URL",
            category="web",
            parameters=[
                ToolParameter(
                    "url",
                    "string",
                    "URL to fetch",
                    required=True,
                    examples=["https://api.github.com/repos/..."],
                ),
                ToolParameter("method", "string", "HTTP method", default="GET"),
                ToolParameter(
                    "headers",
                    "object",
                    "HTTP headers",
                    examples=[{"Authorization": "Bearer token"}],
                ),
            ],
            examples=[
                '{"tool": "web:fetch", "args": {"url": "https://docs.python.org/3/"}}',
            ],
        )
    )

    registry.register(
        ToolDescriptor(
            name="web:search",
            description="Search the web for information",
            category="web",
            parameters=[
                ToolParameter(
                    "query",
                    "string",
                    "Search query",
                    required=True,
                    examples=["Python asyncio tutorial"],
                ),
                ToolParameter("max_results", "number", "Maximum results to return", default=5),
            ],
            examples=[
                '{"tool": "web:search", "args": {"query": "OpenAI GPT-4 API documentation"}}',
            ],
        )
    )

    return registry
