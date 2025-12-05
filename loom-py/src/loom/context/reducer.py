"""Step Reducer for Context Engineering.

This module implements per-tool reduction rules that transform raw tool
outputs into minimal Step representations, keeping only what the LLM
needs in future context.

Reduction Philosophy (from Manus learnings):
1. Keep structural info (file exists, lines changed, etc.)
2. Drop raw content that can be re-fetched
3. Preserve error messages and outcomes
4. Generate one-line observations for quick reference
"""

from __future__ import annotations

import os
import re
from abc import ABC, abstractmethod
from dataclasses import dataclass
from typing import Any, Optional

from .step import Step, generate_step_id


class ToolReducer(ABC):
    """Base class for tool-specific reduction rules."""

    @abstractmethod
    def reduce(
        self,
        step_id: str,
        tool_name: str,
        args: dict[str, Any],
        result: Any,
        success: bool,
        error: Optional[str] = None,
    ) -> Step:
        """Reduce a tool execution to a minimal Step.

        Args:
            step_id: Unique step identifier
            tool_name: Full tool name (e.g., "fs:read_file")
            args: Original tool arguments
            result: Raw tool output
            success: Whether execution succeeded
            error: Error message if failed

        Returns:
            Reduced Step with minimal representation
        """
        pass


class FileReadReducer(ToolReducer):
    """Reducer for file read operations (fs:read_file, etc.)."""

    def reduce(
        self,
        step_id: str,
        tool_name: str,
        args: dict[str, Any],
        result: Any,
        success: bool,
        error: Optional[str] = None,
    ) -> Step:
        path = args.get("path", args.get("file_path", "unknown"))
        filename = os.path.basename(path)

        if not success:
            return Step(
                id=step_id,
                tool_name=tool_name,
                minimal_args={"path": path},
                observation=f"Failed to read {filename}: {error}",
                success=False,
                error=error,
            )

        # Calculate content stats
        content = str(result) if result else ""
        lines = content.count("\n") + 1 if content else 0
        size = len(content)
        size_str = _format_size(size)

        return Step(
            id=step_id,
            tool_name=tool_name,
            minimal_args={"path": path},
            observation=f"Read {filename} ({lines} lines, {size_str})",
            success=True,
            metadata={"lines": lines, "size": size},
        )


class FileWriteReducer(ToolReducer):
    """Reducer for file write operations (fs:write_file, etc.)."""

    def reduce(
        self,
        step_id: str,
        tool_name: str,
        args: dict[str, Any],
        result: Any,
        success: bool,
        error: Optional[str] = None,
    ) -> Step:
        path = args.get("path", args.get("file_path", "unknown"))
        content = args.get("content", "")
        filename = os.path.basename(path)

        if not success:
            return Step(
                id=step_id,
                tool_name=tool_name,
                minimal_args={"path": path},
                observation=f"Failed to write {filename}: {error}",
                success=False,
                error=error,
            )

        lines = content.count("\n") + 1 if content else 0
        size = len(content)
        size_str = _format_size(size)

        return Step(
            id=step_id,
            tool_name=tool_name,
            minimal_args={"path": path},
            observation=f"Wrote {filename} ({lines} lines, {size_str})",
            success=True,
            metadata={"lines": lines, "size": size},
        )


class FileEditReducer(ToolReducer):
    """Reducer for file edit operations (fs:edit_file, etc.)."""

    def reduce(
        self,
        step_id: str,
        tool_name: str,
        args: dict[str, Any],
        result: Any,
        success: bool,
        error: Optional[str] = None,
    ) -> Step:
        path = args.get("path", args.get("file_path", "unknown"))
        filename = os.path.basename(path)

        if not success:
            return Step(
                id=step_id,
                tool_name=tool_name,
                minimal_args={"path": path},
                observation=f"Failed to edit {filename}: {error}",
                success=False,
                error=error,
            )

        # Extract edit stats from args
        old_content = args.get("old_content", args.get("search", ""))
        new_content = args.get("new_content", args.get("replace", ""))
        old_lines = old_content.count("\n") + 1 if old_content else 0
        new_lines = new_content.count("\n") + 1 if new_content else 0
        diff = new_lines - old_lines

        if diff > 0:
            change_str = f"+{diff} lines"
        elif diff < 0:
            change_str = f"{diff} lines"
        else:
            change_str = "modified"

        return Step(
            id=step_id,
            tool_name=tool_name,
            minimal_args={"path": path},
            observation=f"Edited {filename} ({change_str})",
            success=True,
            metadata={"old_lines": old_lines, "new_lines": new_lines},
        )


class ShellReducer(ToolReducer):
    """Reducer for shell/command execution."""

    MAX_OUTPUT_PREVIEW = 100  # chars

    def reduce(
        self,
        step_id: str,
        tool_name: str,
        args: dict[str, Any],
        result: Any,
        success: bool,
        error: Optional[str] = None,
    ) -> Step:
        command = args.get("command", args.get("cmd", ""))
        # Truncate long commands
        cmd_preview = command[:80] + "..." if len(command) > 80 else command

        if not success:
            exit_code = args.get("exit_code", 1)
            return Step(
                id=step_id,
                tool_name=tool_name,
                minimal_args={"command": cmd_preview},
                observation=f"Command failed (exit {exit_code}): {cmd_preview}",
                success=False,
                error=error,
                metadata={"exit_code": exit_code},
            )

        # Parse output stats
        output = str(result) if result else ""
        lines = output.count("\n") + 1 if output else 0
        exit_code = 0

        # Generate observation
        if lines > 10:
            observation = f"Ran `{cmd_preview}` → {lines} lines output"
        elif output:
            # Include short output directly
            preview = output[: self.MAX_OUTPUT_PREVIEW]
            if len(output) > self.MAX_OUTPUT_PREVIEW:
                preview += "..."
            observation = f"Ran `{cmd_preview}` → {preview}"
        else:
            observation = f"Ran `{cmd_preview}` → (no output)"

        return Step(
            id=step_id,
            tool_name=tool_name,
            minimal_args={"command": cmd_preview},
            observation=observation,
            success=True,
            metadata={"lines": lines, "exit_code": exit_code},
        )


class SearchReducer(ToolReducer):
    """Reducer for search/grep operations."""

    def reduce(
        self,
        step_id: str,
        tool_name: str,
        args: dict[str, Any],
        result: Any,
        success: bool,
        error: Optional[str] = None,
    ) -> Step:
        query = args.get("query", args.get("pattern", ""))
        path = args.get("path", args.get("directory", "."))

        if not success:
            return Step(
                id=step_id,
                tool_name=tool_name,
                minimal_args={"query": query, "path": path},
                observation=f"Search failed: {error}",
                success=False,
                error=error,
            )

        # Parse results
        if isinstance(result, list):
            match_count = len(result)
        elif isinstance(result, str):
            match_count = result.count("\n") + 1 if result else 0
        else:
            match_count = 0

        return Step(
            id=step_id,
            tool_name=tool_name,
            minimal_args={"query": query, "path": path},
            observation=f"Search '{query}' → {match_count} matches",
            success=True,
            metadata={"matches": match_count},
        )


class WebFetchReducer(ToolReducer):
    """Reducer for web/HTTP operations."""

    def reduce(
        self,
        step_id: str,
        tool_name: str,
        args: dict[str, Any],
        result: Any,
        success: bool,
        error: Optional[str] = None,
    ) -> Step:
        url = args.get("url", "")
        # Extract domain for brevity
        domain = _extract_domain(url)

        if not success:
            return Step(
                id=step_id,
                tool_name=tool_name,
                minimal_args={"url": url},
                observation=f"Failed to fetch {domain}: {error}",
                success=False,
                error=error,
            )

        content = str(result) if result else ""
        size = len(content)
        size_str = _format_size(size)

        return Step(
            id=step_id,
            tool_name=tool_name,
            minimal_args={"url": url},
            observation=f"Fetched {domain} ({size_str})",
            success=True,
            metadata={"size": size, "domain": domain},
        )


class DefaultReducer(ToolReducer):
    """Fallback reducer for unrecognized tools."""

    MAX_RESULT_PREVIEW = 200

    def reduce(
        self,
        step_id: str,
        tool_name: str,
        args: dict[str, Any],
        result: Any,
        success: bool,
        error: Optional[str] = None,
    ) -> Step:
        if not success:
            return Step(
                id=step_id,
                tool_name=tool_name,
                minimal_args=_minimize_args(args),
                observation=f"{tool_name} failed: {error}",
                success=False,
                error=error,
            )

        # Generate generic observation
        result_str = str(result) if result else ""
        if len(result_str) > self.MAX_RESULT_PREVIEW:
            preview = result_str[: self.MAX_RESULT_PREVIEW] + "..."
        else:
            preview = result_str

        return Step(
            id=step_id,
            tool_name=tool_name,
            minimal_args=_minimize_args(args),
            observation=f"{tool_name} → {preview}" if preview else f"{tool_name} completed",
            success=True,
        )


@dataclass
class StepReducer:
    """Main reducer that dispatches to tool-specific reducers.

    Usage:
        reducer = StepReducer()
        step = reducer.reduce(
            step_id="step_001",
            tool_name="fs:read_file",
            args={"path": "/tmp/test.txt"},
            result="file contents...",
            success=True
        )
    """

    # Tool name patterns → reducers
    _reducers: dict[str, ToolReducer]
    _default: ToolReducer
    _step_counter: int

    def __init__(self):
        self._step_counter = 0
        self._default = DefaultReducer()
        self._reducers = {
            # File operations
            "fs:read_file": FileReadReducer(),
            "fs:read": FileReadReducer(),
            "read_file": FileReadReducer(),
            "fs:write_file": FileWriteReducer(),
            "fs:write": FileWriteReducer(),
            "write_file": FileWriteReducer(),
            "fs:edit_file": FileEditReducer(),
            "fs:edit": FileEditReducer(),
            "edit_file": FileEditReducer(),
            # Shell
            "shell:run": ShellReducer(),
            "shell:exec": ShellReducer(),
            "run_command": ShellReducer(),
            "execute": ShellReducer(),
            # Search
            "fs:search": SearchReducer(),
            "fs:grep": SearchReducer(),
            "search": SearchReducer(),
            "grep": SearchReducer(),
            # Web
            "web:fetch": WebFetchReducer(),
            "web:get": WebFetchReducer(),
            "http:get": WebFetchReducer(),
            "fetch_url": WebFetchReducer(),
        }

    def register(self, tool_name: str, reducer: ToolReducer) -> None:
        """Register a custom reducer for a tool.

        Args:
            tool_name: Tool name pattern
            reducer: ToolReducer instance
        """
        self._reducers[tool_name] = reducer

    def reduce(
        self,
        tool_name: str,
        args: dict[str, Any],
        result: Any,
        success: bool,
        error: Optional[str] = None,
        step_id: Optional[str] = None,
    ) -> Step:
        """Reduce a tool execution to a Step.

        Args:
            tool_name: Full tool name
            args: Original arguments
            result: Raw tool output
            success: Whether execution succeeded
            error: Error message if failed
            step_id: Optional step ID (auto-generated if not provided)

        Returns:
            Reduced Step
        """
        if step_id is None:
            self._step_counter += 1
            step_id = generate_step_id(self._step_counter)

        reducer = self._get_reducer(tool_name)
        return reducer.reduce(
            step_id=step_id,
            tool_name=tool_name,
            args=args,
            result=result,
            success=success,
            error=error,
        )

    def _get_reducer(self, tool_name: str) -> ToolReducer:
        """Get the appropriate reducer for a tool."""
        # Exact match
        if tool_name in self._reducers:
            return self._reducers[tool_name]

        # Try without namespace
        if ":" in tool_name:
            short_name = tool_name.split(":")[-1]
            if short_name in self._reducers:
                return self._reducers[short_name]

        return self._default

    def reset_counter(self) -> None:
        """Reset step counter (for new sessions)."""
        self._step_counter = 0


# --- Utility Functions ---


def _format_size(size_bytes: int) -> str:
    """Format byte size to human-readable string."""
    if size_bytes < 1024:
        return f"{size_bytes}B"
    elif size_bytes < 1024 * 1024:
        return f"{size_bytes / 1024:.1f}KB"
    else:
        return f"{size_bytes / (1024 * 1024):.1f}MB"


def _extract_domain(url: str) -> str:
    """Extract domain from URL."""
    match = re.search(r"https?://([^/]+)", url)
    if match:
        return match.group(1)
    return url[:50] if len(url) > 50 else url


def _minimize_args(args: dict[str, Any], max_value_len: int = 100) -> dict[str, Any]:
    """Minimize arguments by truncating large values."""
    result = {}
    for key, value in args.items():
        if isinstance(value, str) and len(value) > max_value_len:
            result[key] = value[:max_value_len] + "..."
        elif isinstance(value, (list, dict)):
            # Just note the type and size
            if isinstance(value, list):
                result[key] = f"[{len(value)} items]"
            else:
                result[key] = f"{{{len(value)} keys}}"
        else:
            result[key] = value
    return result


__all__ = [
    "StepReducer",
    "ToolReducer",
    "FileReadReducer",
    "FileWriteReducer",
    "FileEditReducer",
    "ShellReducer",
    "SearchReducer",
    "WebFetchReducer",
    "DefaultReducer",
]
