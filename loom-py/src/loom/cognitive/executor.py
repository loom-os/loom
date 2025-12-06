"""Tool execution and approval management for cognitive agents.

This module handles:
- Tool execution with human-in-the-loop approval
- Permission management for destructive operations
- Result processing with data offloading and reduction
"""

from __future__ import annotations

import json
import os
import subprocess
import time
from pathlib import Path
from typing import TYPE_CHECKING, Optional

from opentelemetry import trace

from ..context import Step
from .types import Observation, ToolCall

if TYPE_CHECKING:
    from ..agent import EventContext
    from ..context import DataOffloader, StepReducer

# Get tracer for tool execution spans
tracer = trace.get_tracer(__name__)

# Tools that require human approval before execution (destructive operations)
TOOLS_REQUIRING_APPROVAL = {
    "fs:write_file",  # Can overwrite files
    "fs:delete",  # Can delete files
}


class ToolExecutor:
    """Handles tool execution with approval management and result processing."""

    def __init__(
        self,
        ctx: "EventContext",
        step_reducer: "StepReducer",
        data_offloader: "DataOffloader",
        permission_callback: Optional[callable] = None,
    ):
        """Initialize tool executor.

        Args:
            ctx: Agent context for tool calls
            step_reducer: Step reducer for context engineering
            data_offloader: Data offloader for large outputs
            permission_callback: Optional callback for permission requests
        """
        self.ctx = ctx
        self.step_reducer = step_reducer
        self.data_offloader = data_offloader
        self.permission_callback = permission_callback

        # Track approved commands/tools for this session
        self._approved_commands: set[str] = set()
        self._approved_tools: set[str] = set()

    async def execute_tool(self, tool_call: ToolCall) -> Observation:
        """Execute a tool call with optional human-in-the-loop approval.

        Args:
            tool_call: The tool call to execute

        Returns:
            Observation with execution results
        """
        with tracer.start_as_current_span(
            "cognitive.tool_call",
            attributes={
                "tool.name": tool_call.name,
                "tool.arguments": json.dumps(tool_call.arguments)[:500],
            },
        ) as span:
            start = time.time()

            # Check if this tool requires human approval BEFORE execution
            if self._requires_approval(tool_call):
                reason = self._get_approval_reason(tool_call)
                approved = await self._request_permission(tool_call, reason)

                if not approved:
                    latency_ms = int((time.time() - start) * 1000)
                    span.set_attribute("tool.success", False)
                    span.set_attribute("tool.denied_by_user", True)
                    return Observation(
                        tool_name=tool_call.name,
                        success=False,
                        output="",
                        error="Action denied by user",
                        latency_ms=latency_ms,
                    )

                # User approved - execute directly in Python (bypass Rust sandbox)
                return await self._execute_with_approval(tool_call, span, start)

            # Normal execution through Rust bridge
            return await self._execute_via_bridge(tool_call, span, start)

    def _requires_approval(self, tool_call: ToolCall) -> bool:
        """Check if a tool requires human approval before execution."""
        if tool_call.name in self._approved_tools:
            return False
        return tool_call.name in TOOLS_REQUIRING_APPROVAL

    def _get_approval_reason(self, tool_call: ToolCall) -> str:
        """Generate a human-readable reason for the approval request."""
        if tool_call.name == "fs:write_file":
            path = tool_call.arguments.get("path", "unknown")
            content = tool_call.arguments.get("content", "")
            preview = content[:100] + "..." if len(content) > 100 else content
            return f"Write to file '{path}' (content: {preview})"
        elif tool_call.name == "fs:delete":
            path = tool_call.arguments.get("path", "unknown")
            return f"Delete file or directory '{path}'"
        else:
            return f"Destructive operation: {tool_call.name}"

    async def _request_permission(self, tool_call: ToolCall, error_msg: str) -> bool:
        """Request permission from user for a denied action."""
        if not self.permission_callback:
            return False

        try:
            result = self.permission_callback(tool_call.name, tool_call.arguments, error_msg)
            if hasattr(result, "__await__"):
                return await result
            return bool(result)
        except Exception:
            return False

    async def _execute_via_bridge(
        self, tool_call: ToolCall, span, start_time: float
    ) -> Observation:
        """Execute tool through Rust bridge (normal path)."""
        try:
            result = await self.ctx.tool(
                tool_call.name,
                payload=tool_call.arguments,
                timeout_ms=30000,
            )

            latency_ms = int((time.time() - start_time) * 1000)

            # Parse result if JSON
            try:
                if isinstance(result, bytes):
                    result = result.decode("utf-8")
                parsed = json.loads(result)
                raw_output = json.dumps(parsed, indent=2)
            except (json.JSONDecodeError, AttributeError):
                raw_output = str(result)

            # Process through offloader and reducer
            processed_output, reduced_step = self._process_tool_result(
                tool_call=tool_call,
                raw_output=raw_output,
                success=True,
            )

            span.set_attribute("tool.success", True)
            span.set_attribute("tool.latency_ms", latency_ms)
            span.set_attribute("tool.output.size", len(raw_output))
            if reduced_step and reduced_step.outcome_ref:
                span.set_attribute("tool.offloaded", True)
                span.set_attribute("tool.offload_path", reduced_step.outcome_ref)

            return Observation(
                tool_name=tool_call.name,
                success=True,
                output=processed_output[:2000],
                latency_ms=latency_ms,
                reduced_step=reduced_step,
            )

        except Exception as e:
            error_str = str(e)
            latency_ms = int((time.time() - start_time) * 1000)

            # Check if this is a permission denied error and we have a callback
            if "Permission denied" in error_str and self.permission_callback:
                approved = await self._request_permission(tool_call, error_str)
                if approved:
                    return await self._execute_with_approval(tool_call, span, start_time)

            # Process error through reducer
            _, reduced_step = self._process_tool_result(
                tool_call=tool_call,
                raw_output="",
                success=False,
                error=error_str,
            )

            span.set_attribute("tool.success", False)
            span.set_attribute("tool.latency_ms", latency_ms)
            span.set_attribute("tool.error", error_str)
            span.record_exception(e)

            return Observation(
                tool_name=tool_call.name,
                success=False,
                output="",
                error=error_str,
                latency_ms=latency_ms,
                reduced_step=reduced_step,
            )

    async def _execute_with_approval(
        self, tool_call: ToolCall, span, start_time: float
    ) -> Observation:
        """Execute a tool that requires approval, directly in Python."""
        # Mark this tool as approved for this session
        self._approved_tools.add(tool_call.name)

        try:
            if tool_call.name == "system:shell":
                output = await self._execute_shell_command(tool_call, start_time)
            elif tool_call.name == "fs:write_file":
                output = await self._execute_write_file(tool_call, start_time)
            elif tool_call.name == "fs:delete":
                output = await self._execute_delete(tool_call, start_time)
            else:
                return Observation(
                    tool_name=tool_call.name,
                    success=False,
                    output="",
                    error="Cannot approve this tool type dynamically",
                    latency_ms=int((time.time() - start_time) * 1000),
                )

            latency_ms = int((time.time() - start_time) * 1000)
            span.set_attribute("tool.success", True)
            span.set_attribute("tool.approved_by_user", True)
            span.set_attribute("tool.latency_ms", latency_ms)

            return Observation(
                tool_name=tool_call.name,
                success=True,
                output=output[:2000],
                latency_ms=latency_ms,
            )

        except subprocess.TimeoutExpired:
            return Observation(
                tool_name=tool_call.name,
                success=False,
                output="",
                error="Command timed out after 30 seconds",
                latency_ms=int((time.time() - start_time) * 1000),
            )
        except Exception as e:
            return Observation(
                tool_name=tool_call.name,
                success=False,
                output="",
                error=str(e),
                latency_ms=int((time.time() - start_time) * 1000),
            )

    async def _execute_shell_command(self, tool_call: ToolCall, start_time: float) -> str:
        """Execute shell command directly (user approved)."""
        command = tool_call.arguments.get("command", "")
        args = tool_call.arguments.get("args", [])
        self._approved_commands.add(command)

        # Build command list - never use shell=True to prevent injection
        cmd_list = [command] + args if args else [command]
        proc_result = subprocess.run(
            cmd_list,
            capture_output=True,
            text=True,
            timeout=30,
            shell=False,  # Security: prevent shell injection
        )

        return json.dumps(
            {
                "stdout": proc_result.stdout,
                "stderr": proc_result.stderr,
                "exit_code": proc_result.returncode,
                "approved_by_user": True,
            },
            indent=2,
        )

    async def _execute_write_file(self, tool_call: ToolCall, start_time: float) -> str:
        """Write file directly in Python (user approved)."""
        file_path = tool_call.arguments.get("path", "")
        content = tool_call.arguments.get("content", "")

        # Security: Validate path stays within workspace
        validated_path = self._validate_path(file_path)
        if not validated_path:
            raise ValueError("Path traversal detected: path escapes workspace")

        # Create parent directories if needed
        Path(validated_path).parent.mkdir(parents=True, exist_ok=True)

        # Write content
        with open(validated_path, "w", encoding="utf-8") as f:
            bytes_written = f.write(content)

        return json.dumps(
            {
                "path": validated_path,
                "bytes_written": bytes_written,
                "approved_by_user": True,
            },
            indent=2,
        )

    async def _execute_delete(self, tool_call: ToolCall, start_time: float) -> str:
        """Delete file or directory directly in Python (user approved)."""
        file_path = tool_call.arguments.get("path", "")

        # Security: Validate path stays within workspace
        validated_path = self._validate_path(file_path)
        if not validated_path:
            raise ValueError("Path traversal detected: path escapes workspace")

        path_obj = Path(validated_path)
        if path_obj.is_dir():
            os.rmdir(validated_path)  # Only deletes empty directories
            deleted_type = "directory (empty)"
        elif path_obj.exists():
            os.remove(validated_path)
            deleted_type = "file"
        else:
            raise FileNotFoundError(f"Path does not exist: {validated_path}")

        return json.dumps(
            {
                "path": validated_path,
                "deleted": deleted_type,
                "approved_by_user": True,
            },
            indent=2,
        )

    def _validate_path(self, file_path: str) -> Optional[str]:
        """Validate path stays within workspace (prevent path traversal attacks).

        Args:
            file_path: Path to validate

        Returns:
            Absolute validated path if safe, None if path escapes workspace
        """
        workspace_root = os.path.abspath(os.getcwd())
        if not os.path.isabs(file_path):
            file_path = os.path.join(workspace_root, file_path)
        file_path = os.path.abspath(file_path)

        # Check for path traversal attack
        if not file_path.startswith(workspace_root + os.sep) and file_path != workspace_root:
            return None

        return file_path

    def _process_tool_result(
        self,
        tool_call: ToolCall,
        raw_output: str,
        success: bool,
        error: Optional[str] = None,
    ) -> tuple[str, Optional[Step]]:
        """Process tool output through offloader and reducer.

        Args:
            tool_call: The tool call that was executed
            raw_output: Raw tool output string
            success: Whether execution succeeded
            error: Error message if failed

        Returns:
            Tuple of (processed_output, reduced_step)
        """
        # For failed calls, no need to offload
        if not success:
            step = self.step_reducer.reduce(
                tool_name=tool_call.name,
                args=tool_call.arguments,
                result=None,
                success=False,
                error=error,
            )
            return raw_output, step

        # Check if output should be offloaded
        offload_result = self.data_offloader.offload(
            content=raw_output,
            category=self._get_offload_category(tool_call.name),
            identifier=self._get_offload_identifier(tool_call),
        )

        # Use offloaded preview if it was offloaded
        output_for_observation = offload_result.content if offload_result.offloaded else raw_output

        # Reduce to Step
        step = self.step_reducer.reduce(
            tool_name=tool_call.name,
            args=tool_call.arguments,
            result=output_for_observation,
            success=True,
        )

        # Attach offload reference if offloaded
        if offload_result.offloaded:
            step.outcome_ref = offload_result.file_path

        return output_for_observation, step

    def _get_offload_category(self, tool_name: str) -> str:
        """Determine offload category from tool name."""
        name_lower = tool_name.lower()
        if "read" in name_lower or "file" in name_lower:
            return "file_read"
        elif "shell" in name_lower or "run" in name_lower:
            return "shell_output"
        elif "search" in name_lower or "grep" in name_lower:
            return "search"
        elif "web" in name_lower or "http" in name_lower:
            return "web"
        else:
            return "tool_output"

    def _get_offload_identifier(self, tool_call: ToolCall) -> str:
        """Generate identifier for offloaded file."""
        # Try common path arguments
        for key in ["path", "file_path", "file", "url"]:
            if key in tool_call.arguments:
                value = tool_call.arguments[key]
                if isinstance(value, str):
                    return value
        # Fallback to tool name + timestamp
        return f"{tool_call.name}_{int(time.time())}"


__all__ = [
    "ToolExecutor",
    "TOOLS_REQUIRING_APPROVAL",
]
