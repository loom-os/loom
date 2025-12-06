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

import time
from pathlib import Path
from typing import TYPE_CHECKING, AsyncIterator, Optional, Union

from opentelemetry import trace

from ..context import (
    DataOffloader,
    OffloadConfig,
    StepCompactor,
    StepReducer,
    ToolRegistry,
    create_default_registry,
)
from ..context.memory import WorkingMemory
from .config import CognitiveConfig, ThinkingStrategy
from .executor import ToolExecutor
from .strategies import StrategyExecutor
from .types import CognitiveResult, ThoughtStep

if TYPE_CHECKING:
    from ..agent import EventContext
    from ..llm import LLMProvider

# Get tracer for cognitive spans
tracer = trace.get_tracer(__name__)


class CognitiveAgent:
    """Autonomous agent with perceive-think-act cognitive loop.

    This class wraps an Agent's EventContext and LLMProvider to implement
    the cognitive loop pattern for autonomous task execution.
    """

    def __init__(
        self,
        ctx: "EventContext",
        llm: "LLMProvider",
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

        # Initialize executors
        self.tool_executor = ToolExecutor(
            ctx=ctx,
            step_reducer=self.step_reducer,
            data_offloader=self.data_offloader,
            permission_callback=permission_callback,
        )

        self.strategy_executor = StrategyExecutor(
            llm=llm,
            config=self.config,
            memory=self.memory,
            tool_executor=self.tool_executor,
            step_compactor=self.step_compactor,
            available_tools=self.available_tools,
            tool_registry=self.tool_registry,
        )

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
        # Update strategy executor's tool list
        self.strategy_executor.available_tools = tools

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

            # Add goal to memory
            self.memory.add("user", goal)

            # Add context if provided
            if context:
                for ctx_item in context:
                    self.memory.add("system", f"Context: {ctx_item}")

            try:
                if self.config.thinking_strategy == ThinkingStrategy.SINGLE_SHOT:
                    result = await self.strategy_executor.run_single_shot(goal)
                elif self.config.thinking_strategy == ThinkingStrategy.REACT:
                    result = await self.strategy_executor.run_react(goal)
                else:  # ChainOfThought
                    result = await self.strategy_executor.run_cot(goal)

                span.set_attribute("success", result.success)
                span.set_attribute("iterations", result.iterations)
                span.set_attribute("answer.length", len(result.answer))

            except Exception as e:
                result = CognitiveResult(answer="", iterations=0)
                result.success = False
                result.error = str(e)
                span.record_exception(e)
                span.set_status(trace.Status(trace.StatusCode.ERROR, str(e)))

            result.total_latency_ms = int((time.time() - start_time) * 1000)
            return result

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
            async for item in self.strategy_executor.run_react_stream(goal):
                yield item


__all__ = [
    "CognitiveAgent",
]
