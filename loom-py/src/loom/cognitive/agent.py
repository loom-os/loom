"""Cognitive Agent - Autonomous perceive-think-act loop for Python agents.

This module provides a CognitiveAgent class that implements the cognitive loop pattern:
- Perceive: Gather context from events, memory, and environment
- Think: Use LLM to reason and decide on actions
- Act: Execute tools and produce outputs

Example:
    ```python
    from loom import Agent
    from loom.cognitive import CognitiveAgent, CognitiveConfig
    from loom.llm import LLMProvider

    agent = Agent(agent_id="researcher", topics=["research.tasks"])
    await agent.start()

    cognitive = CognitiveAgent(
        ctx=agent._ctx,
        llm=LLMProvider.from_name(agent._ctx, "deepseek"),
        config=CognitiveConfig(
            system_prompt="You are a research assistant...",
            max_iterations=5,
        ),
    )

    result = await cognitive.run("Research the latest AI trends")
    print(result.answer)
    ```
"""

from __future__ import annotations

import json
import time
from pathlib import Path
from typing import TYPE_CHECKING, AsyncIterator, Optional, Union

from opentelemetry import trace

from ..context import (
    DataOffloader,
    OffloadConfig,
    Step,
    StepCompactor,
    StepReducer,
    ToolRegistry,
    create_default_registry,
)
from .config import CognitiveConfig, ThinkingStrategy
from .loop import (
    build_cot_prompt,
    build_react_prompt,
    build_react_system_prompt,
    parse_react_response,
    synthesize_answer,
)
from .types import CognitiveResult, Observation, ThoughtStep, ToolCall

if TYPE_CHECKING:
    from ..agent import Context
    from ..llm import LLMProvider

# Import for backwards compatibility - these are re-exported
from ..context.memory import WorkingMemory

# Get tracer for cognitive spans
tracer = trace.get_tracer(__name__)

# Tools that require human approval before execution (destructive operations)
TOOLS_REQUIRING_APPROVAL = {
    "fs:write_file",  # Can overwrite files
    "fs:delete",  # Can delete files
}


class CognitiveAgent:
    """Autonomous agent with perceive-think-act cognitive loop.

    This class wraps an Agent's Context and LLMProvider to implement
    the cognitive loop pattern for autonomous task execution.
    """

    def __init__(
        self,
        ctx: Context,
        llm: LLMProvider,
        config: Optional[CognitiveConfig] = None,
        available_tools: Optional[list[str]] = None,
        permission_callback: Optional[callable] = None,
        workspace_path: Optional[Union[str, Path]] = None,
        tool_registry: Optional[ToolRegistry] = None,
    ):
        """Initialize cognitive agent.

        Args:
            ctx: Agent context for tool calls
            llm: LLM provider for reasoning
            config: Cognitive configuration
            available_tools: List of available tool names (e.g., ["weather:get", "web:search"])
            permission_callback: Optional async callback for permission requests.
                                 Called with (tool_name, args, error_message) -> bool
                                 If returns True, the tool will be retried with approval.
            workspace_path: Path to workspace for data offloading (default: current directory)
            tool_registry: Optional ToolRegistry for enhanced tool descriptions (default: creates one)
        """
        self.ctx = ctx
        self.llm = llm
        self.config = config or CognitiveConfig()
        self.available_tools = available_tools or []
        self.memory = WorkingMemory()
        self.permission_callback = permission_callback
        # Track commands/tools that have been approved by the user for this session
        self._approved_commands: set[str] = set()
        self._approved_tools: set[str] = set()  # For fs:write_file, fs:delete etc.

        # Context Engineering components
        self.step_reducer = StepReducer()
        self.step_compactor = StepCompactor()
        workspace = Path(workspace_path) if workspace_path else Path.cwd()
        self.data_offloader = DataOffloader(
            workspace,
            OffloadConfig(enabled=True, size_threshold=2048, line_threshold=50),
        )

        # Tool descriptor registry for enhanced system prompts
        self.tool_registry = tool_registry or create_default_registry()

        # Auto-discover tools from registry
        self._auto_discover_tools()

    def _auto_discover_tools(self) -> None:
        """Auto-discover and register tools from available_tools list."""
        if not self.available_tools:
            return

        # Register any tools not already in registry
        for tool_name in self.available_tools:
            if not self.tool_registry.get(tool_name):
                # Auto-register with simple descriptor
                self.tool_registry.register_simple(
                    tool_name,
                    f"Execute {tool_name}",
                )

    def set_available_tools(self, tools: list[str]) -> None:
        """Update the list of available tools and auto-discover new ones."""
        self.available_tools = tools
        self._auto_discover_tools()

    def register_tool(
        self,
        name: str,
        description: str,
        parameters: Optional[list] = None,
        examples: Optional[list[str]] = None,
        category: Optional[str] = None,
    ) -> None:
        """Register a tool with detailed descriptor.

        Args:
            name: Tool name (e.g., "fs:read_file")
            description: What the tool does
            parameters: List of ToolParameter objects
            examples: Usage examples in JSON format
            category: Tool category (filesystem, shell, web, etc.)
        """
        from ..context import ToolDescriptor, ToolParameter

        # Convert dict params to ToolParameter if needed
        if parameters:
            param_objs = []
            for p in parameters:
                if isinstance(p, dict):
                    param_objs.append(ToolParameter(**p))
                else:
                    param_objs.append(p)
            parameters = param_objs

        descriptor = ToolDescriptor(
            name=name,
            description=description,
            parameters=parameters or [],
            examples=examples or [],
            category=category,
        )
        self.tool_registry.register(descriptor)

        # Add to available tools if not already present
        if name not in self.available_tools:
            self.available_tools.append(name)

    async def run(self, goal: str, context: Optional[list[str]] = None) -> CognitiveResult:
        """Execute the cognitive loop to achieve a goal.

        Args:
            goal: The task/goal to accomplish
            context: Optional additional context strings

        Returns:
            CognitiveResult with answer and execution details
        """
        with tracer.start_as_current_span(
            "cognitive.run",
            attributes={
                "agent.id": self.ctx.agent_id,
                "goal": goal[:200],
                "strategy": self.config.thinking_strategy.value,
                "max_iterations": self.config.max_iterations,
            },
        ) as span:
            start_time = time.time()

            # Initialize result
            result = CognitiveResult(answer="", iterations=0)

            # Add goal to memory
            self.memory.add("user", goal)

            # Add context if provided
            if context:
                for ctx_item in context:
                    self.memory.add("system", f"Context: {ctx_item}")

            try:
                if self.config.thinking_strategy == ThinkingStrategy.SINGLE_SHOT:
                    result = await self._run_single_shot(goal)
                elif self.config.thinking_strategy == ThinkingStrategy.REACT:
                    result = await self._run_react(goal)
                else:  # ChainOfThought
                    result = await self._run_cot(goal)

                span.set_attribute("success", result.success)
                span.set_attribute("iterations", result.iterations)
                span.set_attribute("answer.length", len(result.answer))

            except Exception as e:
                result.success = False
                result.error = str(e)
                span.record_exception(e)
                span.set_status(trace.Status(trace.StatusCode.ERROR, str(e)))

            result.total_latency_ms = int((time.time() - start_time) * 1000)
            return result

    async def _run_single_shot(self, goal: str) -> CognitiveResult:
        """Single shot: one LLM call, no tools."""
        system = self.config.system_prompt or "You are a helpful AI assistant."

        response = await self.llm.generate(
            prompt=goal,
            system=system,
            temperature=self.config.temperature,
        )

        self.memory.add("assistant", response)

        return CognitiveResult(
            answer=response,
            iterations=1,
            success=True,
        )

    async def _run_react(self, goal: str) -> CognitiveResult:
        """ReAct pattern: iterative Thought -> Action -> Observation."""
        result = CognitiveResult(answer="", iterations=0)

        system = build_react_system_prompt(
            self.config.system_prompt,
            self.available_tools,
            tool_registry=self.tool_registry,
        )

        for iteration in range(self.config.max_iterations):
            result.iterations = iteration + 1

            with tracer.start_as_current_span(
                "cognitive.react_iteration",
                attributes={
                    "iteration": iteration + 1,
                    "goal": goal[:100],
                },
            ) as iter_span:
                # Build prompt with history
                prompt = build_react_prompt(goal, result.steps)

                # Think - LLM call
                with tracer.start_as_current_span(
                    "cognitive.think",
                    attributes={"prompt.length": len(prompt)},
                ):
                    response = await self.llm.generate(
                        prompt=prompt,
                        system=system,
                        temperature=self.config.temperature,
                    )

                # Parse response
                parsed = parse_react_response(response)
                iter_span.set_attribute("response.type", parsed["type"])

                if parsed["type"] == "final_answer":
                    result.answer = parsed["content"]
                    result.success = True
                    self.memory.add("assistant", result.answer)
                    iter_span.set_attribute("final_answer", True)
                    break

                elif parsed["type"] == "tool_call":
                    step = ThoughtStep(
                        step=iteration + 1,
                        reasoning=parsed.get("thought", ""),
                        tool_call=ToolCall(
                            name=parsed["tool"],
                            arguments=parsed.get("args", {}),
                        ),
                    )

                    # Execute tool
                    observation = await self._execute_tool(step.tool_call)
                    step.observation = observation

                    # Attach reduced step if available
                    if observation.reduced_step:
                        step.reduced_step = observation.reduced_step

                    result.steps.append(step)
                    self.memory.add(
                        "assistant",
                        f"Thought: {step.reasoning}\nAction: {step.tool_call.name}",
                    )
                    self.memory.add(
                        "system",
                        f"Observation: {observation.output if observation.success else observation.error}",
                    )

                else:
                    # Just reasoning, continue
                    result.steps.append(
                        ThoughtStep(
                            step=iteration + 1,
                            reasoning=parsed.get("content", response),
                        )
                    )
                    self.memory.add("assistant", f"Thought: {parsed.get('content', response)}")

        # If we exhausted iterations without final answer
        if not result.answer:
            # Try to synthesize from observations
            result.answer = synthesize_answer(result.steps)
            result.success = bool(result.answer)

        return result

    async def _run_cot(self, goal: str) -> CognitiveResult:
        """Chain of thought: step by step reasoning without tools."""
        system = self.config.system_prompt or (
            "You are a helpful AI assistant. Think through problems step by step. "
            "Show your reasoning process clearly."
        )

        prompt = build_cot_prompt(goal)

        response = await self.llm.generate(
            prompt=prompt,
            system=system,
            temperature=self.config.temperature,
        )

        self.memory.add("assistant", response)

        return CognitiveResult(
            answer=response,
            iterations=1,
            success=True,
        )

    async def run_stream(
        self,
        goal: str,
        *,
        context: Optional[list[str]] = None,
    ) -> AsyncIterator[Union[str, ThoughtStep, CognitiveResult]]:
        """
        Stream the cognitive process, yielding:
        - str: LLM response chunks as they arrive
        - ThoughtStep: Complete thought/action/observation steps
        - CognitiveResult: Final result at the end

        Args:
            goal: The goal/question to process
            context: Optional list of context strings to include
        """
        # Add goal to memory
        self.memory.add("user", goal)

        # Add context if provided
        if context:
            for ctx_item in context:
                self.memory.add("system", f"Context: {ctx_item}")

        if self.config.thinking_strategy != ThinkingStrategy.REACT:
            # For non-ReAct strategies, fall back to non-streaming
            result = await self.run(goal, context=context)
            yield result
            return

        with tracer.start_as_current_span(
            "cognitive.run_stream",
            attributes={
                "strategy": self.config.thinking_strategy.value,
                "goal_length": len(goal),
            },
        ):
            async for item in self._run_react_stream(goal):
                yield item

    async def _run_react_stream(
        self,
        goal: str,
    ) -> AsyncIterator[Union[str, ThoughtStep, CognitiveResult]]:
        """ReAct pattern with streaming: yield chunks and steps as they happen."""
        result = CognitiveResult(answer="", iterations=0)

        system = build_react_system_prompt(
            self.config.system_prompt,
            self.available_tools,
            tool_registry=self.tool_registry,
        )

        for iteration in range(self.config.max_iterations):
            result.iterations = iteration + 1

            with tracer.start_as_current_span(
                "cognitive.react_stream_iteration",
                attributes={
                    "iteration": iteration + 1,
                    "goal": goal[:100],
                    "steps_so_far": len(result.steps),
                },
            ) as iter_span:
                # Build prompt with history
                prompt = build_react_prompt(goal, result.steps)

                # Stream the LLM response with thinking span
                full_response = ""
                with tracer.start_as_current_span(
                    "cognitive.think_stream",
                    attributes={"prompt.length": len(prompt)},
                ) as think_span:
                    async for chunk in self.llm.generate_stream(
                        prompt=prompt,
                        system=system,
                        temperature=self.config.temperature,
                    ):
                        full_response += chunk
                        yield chunk  # Stream each chunk to caller
                    think_span.set_attribute("response.length", len(full_response))

                # Parse the complete response
                parsed = parse_react_response(full_response)
                iter_span.set_attribute("response.type", parsed["type"])

                if parsed["type"] == "final_answer":
                    result.answer = parsed["content"]
                    result.success = True
                    self.memory.add("assistant", result.answer)
                    iter_span.set_attribute("final_answer", True)
                    break

                elif parsed["type"] == "tool_call":
                    step = ThoughtStep(
                        step=iteration + 1,
                        reasoning=parsed.get("thought", ""),
                        tool_call=ToolCall(
                            name=parsed["tool"],
                            arguments=parsed.get("args", {}),
                        ),
                    )

                    iter_span.set_attribute("tool.name", parsed["tool"])

                    # Execute tool
                    observation = await self._execute_tool(step.tool_call)
                    step.observation = observation

                    # Attach reduced step if available
                    if observation.reduced_step:
                        step.reduced_step = observation.reduced_step

                    result.steps.append(step)
                    self.memory.add(
                        "assistant",
                        f"Thought: {step.reasoning}\nAction: {step.tool_call.name}",
                    )
                    self.memory.add(
                        "system",
                        f"Observation: {observation.output if observation.success else observation.error}",
                    )

                    # Yield the complete step
                    yield step

                else:
                    # Just reasoning, continue
                    step = ThoughtStep(
                        step=iteration + 1,
                        reasoning=parsed.get("content", full_response),
                    )
                    result.steps.append(step)
                    self.memory.add("assistant", f"Thought: {step.reasoning}")
                    yield step

        # If we exhausted iterations without final answer
        if not result.answer:
            result.answer = synthesize_answer(result.steps)
            result.success = bool(result.answer)

        # Yield final result
        yield result

    async def _execute_tool(self, tool_call: ToolCall) -> Observation:
        """Execute a tool call via context, with optional human-in-the-loop approval."""
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
                # Generate a descriptive reason for the permission request
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
                return await self._execute_tool_with_approval(tool_call, span, start)

            # Normal execution through Rust bridge
            try:
                result = await self.ctx.tool(
                    tool_call.name,
                    payload=tool_call.arguments,
                    timeout_ms=30000,
                )

                latency_ms = int((time.time() - start) * 1000)

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
                    output=processed_output[:2000],  # Still truncate for safety
                    latency_ms=latency_ms,
                    reduced_step=reduced_step,
                )

            except Exception as e:
                error_str = str(e)
                latency_ms = int((time.time() - start) * 1000)

                # Check if this is a permission denied error and we have a callback
                if "Permission denied" in error_str and self.permission_callback:
                    # Extract the denied command/action from the error
                    approved = await self._request_permission(tool_call, error_str)

                    if approved:
                        # User approved - retry with the approved flag
                        return await self._execute_tool_with_approval(tool_call, span, start)

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

    def _requires_approval(self, tool_call: ToolCall) -> bool:
        """Check if a tool requires human approval before execution."""
        # Already approved in this session
        if tool_call.name in self._approved_tools:
            return False
        # Check if tool is in the list requiring approval
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
            # Call the permission callback (should be async)
            result = self.permission_callback(tool_call.name, tool_call.arguments, error_msg)
            if hasattr(result, "__await__"):
                return await result
            return bool(result)
        except Exception:
            return False

    async def _execute_tool_with_approval(
        self, tool_call: ToolCall, span, start_time
    ) -> Observation:
        """Execute a tool that requires approval, directly in Python.

        For shell commands and file system operations, we execute directly
        in Python since the Rust sandbox won't allow dynamic approval.
        """
        import os
        import subprocess

        # Mark this tool as approved for this session
        self._approved_tools.add(tool_call.name)

        try:
            if tool_call.name == "system:shell":
                # Execute shell command directly (user approved)
                # Always use list form with shell=False to prevent injection attacks
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

                latency_ms = int((time.time() - start_time) * 1000)
                output = json.dumps(
                    {
                        "stdout": proc_result.stdout,
                        "stderr": proc_result.stderr,
                        "exit_code": proc_result.returncode,
                        "approved_by_user": True,
                    },
                    indent=2,
                )

            elif tool_call.name == "fs:write_file":
                # Write file directly in Python
                file_path = tool_call.arguments.get("path", "")
                content = tool_call.arguments.get("content", "")

                # Security: Resolve and validate path stays within workspace
                workspace_root = os.path.abspath(os.getcwd())
                if not os.path.isabs(file_path):
                    file_path = os.path.join(workspace_root, file_path)
                file_path = os.path.abspath(file_path)

                # Check for path traversal attack
                if (
                    not file_path.startswith(workspace_root + os.sep)
                    and file_path != workspace_root
                ):
                    return Observation(
                        tool_name=tool_call.name,
                        success=False,
                        output="",
                        error="Path traversal detected: path escapes workspace",
                        latency_ms=int((time.time() - start_time) * 1000),
                    )

                # Create parent directories if needed
                parent = Path(file_path).parent
                parent.mkdir(parents=True, exist_ok=True)

                # Write content
                with open(file_path, "w", encoding="utf-8") as f:
                    bytes_written = f.write(content)

                latency_ms = int((time.time() - start_time) * 1000)
                output = json.dumps(
                    {
                        "path": file_path,
                        "bytes_written": bytes_written,
                        "approved_by_user": True,
                    },
                    indent=2,
                )

            elif tool_call.name == "fs:delete":
                # Delete file or directory directly in Python
                file_path = tool_call.arguments.get("path", "")

                # Security: Resolve and validate path stays within workspace
                workspace_root = os.path.abspath(os.getcwd())
                if not os.path.isabs(file_path):
                    file_path = os.path.join(workspace_root, file_path)
                file_path = os.path.abspath(file_path)

                # Check for path traversal attack
                if (
                    not file_path.startswith(workspace_root + os.sep)
                    and file_path != workspace_root
                ):
                    return Observation(
                        tool_name=tool_call.name,
                        success=False,
                        output="",
                        error="Path traversal detected: path escapes workspace",
                        latency_ms=int((time.time() - start_time) * 1000),
                    )

                path_obj = Path(file_path)
                if path_obj.is_dir():
                    # Use os.rmdir() instead of shutil.rmtree() to match Rust behavior
                    # (only deletes empty directories, safer)
                    os.rmdir(file_path)
                    deleted_type = "directory (empty)"
                elif path_obj.exists():
                    os.remove(file_path)
                    deleted_type = "file"
                else:
                    return Observation(
                        tool_name=tool_call.name,
                        success=False,
                        output="",
                        error=f"Path does not exist: {file_path}",
                        latency_ms=int((time.time() - start_time) * 1000),
                    )

                latency_ms = int((time.time() - start_time) * 1000)
                output = json.dumps(
                    {
                        "path": file_path,
                        "deleted": deleted_type,
                        "approved_by_user": True,
                    },
                    indent=2,
                )

            else:
                # For other tools, we can't bypass the Rust sandbox
                return Observation(
                    tool_name=tool_call.name,
                    success=False,
                    output="",
                    error="Cannot approve this tool type dynamically",
                    latency_ms=int((time.time() - start_time) * 1000),
                )

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
            - processed_output: Output to show in Observation (may be preview)
            - reduced_step: Context-reduced Step for history
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
            result=output_for_observation,  # Use preview for reduction
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
    "CognitiveAgent",
]
