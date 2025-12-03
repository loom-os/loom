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
from typing import TYPE_CHECKING, AsyncIterator, Optional, Union

from opentelemetry import trace

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
    ):
        """Initialize cognitive agent.

        Args:
            ctx: Agent context for tool calls
            llm: LLM provider for reasoning
            config: Cognitive configuration
            available_tools: List of available tool names (e.g., ["weather:get", "web:search"])
        """
        self.ctx = ctx
        self.llm = llm
        self.config = config or CognitiveConfig()
        self.available_tools = available_tools or []
        self.memory = WorkingMemory()

    def set_available_tools(self, tools: list[str]) -> None:
        """Update the list of available tools."""
        self.available_tools = tools

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

        system = build_react_system_prompt(self.config.system_prompt, self.available_tools)

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

        system = build_react_system_prompt(self.config.system_prompt, self.available_tools)

        for iteration in range(self.config.max_iterations):
            result.iterations = iteration + 1

            with tracer.start_as_current_span(
                "cognitive.react_stream_iteration",
                attributes={"iteration": iteration + 1},
            ):
                # Build prompt with history
                prompt = build_react_prompt(goal, result.steps)

                # Stream the LLM response
                full_response = ""
                async for chunk in self.llm.generate_stream(
                    prompt=prompt,
                    system=system,
                    temperature=self.config.temperature,
                ):
                    full_response += chunk
                    yield chunk  # Stream each chunk to caller

                # Parse the complete response
                parsed = parse_react_response(full_response)

                if parsed["type"] == "final_answer":
                    result.answer = parsed["content"]
                    result.success = True
                    self.memory.add("assistant", result.answer)
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
        """Execute a tool call via context."""
        with tracer.start_as_current_span(
            "cognitive.tool_call",
            attributes={
                "tool.name": tool_call.name,
                "tool.arguments": json.dumps(tool_call.arguments)[:500],
            },
        ) as span:
            start = time.time()

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
                    output = json.dumps(parsed, indent=2)
                except (json.JSONDecodeError, AttributeError):
                    output = str(result)

                span.set_attribute("tool.success", True)
                span.set_attribute("tool.latency_ms", latency_ms)
                span.set_attribute("tool.output.size", len(output))

                return Observation(
                    tool_name=tool_call.name,
                    success=True,
                    output=output[:2000],  # Truncate long outputs
                    latency_ms=latency_ms,
                )

            except Exception as e:
                latency_ms = int((time.time() - start) * 1000)
                span.set_attribute("tool.success", False)
                span.set_attribute("tool.latency_ms", latency_ms)
                span.set_attribute("tool.error", str(e))
                span.record_exception(e)

                return Observation(
                    tool_name=tool_call.name,
                    success=False,
                    output="",
                    error=str(e),
                    latency_ms=latency_ms,
                )


__all__ = [
    "CognitiveAgent",
]
