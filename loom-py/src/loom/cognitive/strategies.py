"""Thinking strategies for cognitive agents.

This module implements different reasoning strategies:
- Single-shot: One LLM call, no iteration
- ReAct: Iterative Reasoning + Acting with tool use
- Chain-of-Thought: Step-by-step reasoning without tools
"""

from __future__ import annotations

from typing import TYPE_CHECKING, AsyncIterator, Optional, Union

from opentelemetry import trace

from .loop import (
    build_cot_prompt,
    build_react_prompt,
    build_react_system_prompt,
    parse_react_response,
    synthesize_answer,
)
from .types import CognitiveResult, ThoughtStep, ToolCall

if TYPE_CHECKING:
    from ..context import StepCompactor, ToolRegistry
    from ..context.memory import WorkingMemory
    from ..llm import LLMProvider
    from .config import CognitiveConfig
    from .executor import ToolExecutor

# Get tracer for strategy spans
tracer = trace.get_tracer(__name__)


class StrategyExecutor:
    """Executes different thinking strategies for cognitive agents."""

    def __init__(
        self,
        llm: LLMProvider,
        config: CognitiveConfig,
        memory: WorkingMemory,
        tool_executor: ToolExecutor,
        step_compactor: StepCompactor,
        available_tools: list[str],
        tool_registry: Optional[ToolRegistry] = None,
    ):
        """Initialize strategy executor.

        Args:
            llm: LLM provider for reasoning
            config: Cognitive configuration
            memory: Working memory for conversation history
            tool_executor: Tool executor for action execution
            step_compactor: Step compactor for context engineering
            available_tools: List of available tool names
            tool_registry: Optional tool registry for enhanced descriptions
        """
        self.llm = llm
        self.config = config
        self.memory = memory
        self.tool_executor = tool_executor
        self.step_compactor = step_compactor
        self.available_tools = available_tools
        self.tool_registry = tool_registry

    async def run_single_shot(self, goal: str) -> CognitiveResult:
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

    async def run_react(self, goal: str) -> CognitiveResult:
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
                prompt = build_react_prompt(
                    goal,
                    result.steps,
                    compactor=self.step_compactor,
                    use_compaction=True,
                )

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
                    observation = await self.tool_executor.execute_tool(step.tool_call)
                    step.observation = observation

                    # Attach reduced step if available
                    if observation.reduced_step:
                        step.reduced_step = observation.reduced_step

                    result.steps.append(step)
                    self.memory.add(
                        "assistant",
                        f"Thought: {step.reasoning}\nAction: {step.tool_call.name}",
                    )

                    # Add observation with offload reference if available
                    if observation.reduced_step and observation.reduced_step.outcome_ref:
                        obs_text = (
                            f"Observation: (Data saved to {observation.reduced_step.outcome_ref})"
                        )
                    else:
                        obs_text = f"Observation: {observation.output if observation.success else observation.error}"
                    self.memory.add("system", obs_text)

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
            result.answer = synthesize_answer(result.steps)
            result.success = bool(result.answer)

        return result

    async def run_react_stream(
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
                prompt = build_react_prompt(
                    goal,
                    result.steps,
                    compactor=self.step_compactor,
                    use_compaction=True,
                )

                # Stream the LLM response
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
                    observation = await self.tool_executor.execute_tool(step.tool_call)
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

    async def run_cot(self, goal: str) -> CognitiveResult:
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


__all__ = [
    "StrategyExecutor",
]
