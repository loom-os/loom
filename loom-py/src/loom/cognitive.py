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
import re
from dataclasses import dataclass, field
from enum import Enum
from typing import Any, Optional

from opentelemetry import trace

from .context import Context
from .llm import LLMProvider

# Get tracer for cognitive spans
tracer = trace.get_tracer(__name__)


class ThinkingStrategy(Enum):
    """Strategy for cognitive processing."""

    SINGLE_SHOT = "single_shot"  # One LLM call, no tools
    REACT = "react"  # ReAct pattern: Thought -> Action -> Observation
    CHAIN_OF_THOUGHT = "cot"  # Chain of thought reasoning


@dataclass
class CognitiveConfig:
    """Configuration for cognitive loop."""

    system_prompt: Optional[str] = None
    thinking_strategy: ThinkingStrategy = ThinkingStrategy.REACT
    max_iterations: int = 10
    max_tools_per_step: int = 3
    temperature: float = 0.7
    stop_on_final_answer: bool = True


@dataclass
class ToolCall:
    """A tool call to be executed."""

    name: str
    arguments: dict[str, Any]

    def to_dict(self) -> dict:
        return {"tool": self.name, "args": self.arguments}


@dataclass
class Observation:
    """Result of a tool execution."""

    tool_name: str
    success: bool
    output: str
    error: Optional[str] = None
    latency_ms: int = 0


@dataclass
class ThoughtStep:
    """A single step in the reasoning process."""

    step: int
    reasoning: str
    tool_call: Optional[ToolCall] = None
    observation: Optional[Observation] = None


@dataclass
class CognitiveResult:
    """Result of a cognitive loop execution."""

    answer: str
    steps: list[ThoughtStep] = field(default_factory=list)
    iterations: int = 0
    success: bool = True
    error: Optional[str] = None
    total_latency_ms: int = 0


class WorkingMemory:
    """Working memory for the cognitive loop.

    Stores conversation history and intermediate results.
    """

    def __init__(self, max_items: int = 50):
        self.max_items = max_items
        self._items: list[dict[str, Any]] = []

    def add(self, role: str, content: str, metadata: Optional[dict] = None) -> None:
        """Add an item to working memory."""
        item = {"role": role, "content": content}
        if metadata:
            item["metadata"] = metadata
        self._items.append(item)

        # Trim if over limit
        if len(self._items) > self.max_items:
            self._items = self._items[-self.max_items :]

    def get_context(self, max_items: Optional[int] = None) -> list[dict[str, Any]]:
        """Get recent items from memory."""
        n = max_items or len(self._items)
        return self._items[-n:]

    def to_messages(self) -> list[dict[str, str]]:
        """Convert memory to chat messages format."""
        return [{"role": item["role"], "content": item["content"]} for item in self._items]

    def clear(self) -> None:
        """Clear all items."""
        self._items.clear()


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
            import time

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

        system = self._build_react_system_prompt()

        for iteration in range(self.config.max_iterations):
            result.iterations = iteration + 1

            with tracer.start_as_current_span(
                "cognitive.react_iteration",
                attributes={"iteration": iteration + 1},
            ):
                # Build prompt with history
                prompt = self._build_react_prompt(goal, result.steps)

                # Think
                response = await self.llm.generate(
                    prompt=prompt,
                    system=system,
                    temperature=self.config.temperature,
                )

                # Parse response
                parsed = self._parse_react_response(response)

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
            result.answer = self._synthesize_answer(result.steps)
            result.success = bool(result.answer)

        return result

    async def _run_cot(self, goal: str) -> CognitiveResult:
        """Chain of thought: step by step reasoning without tools."""
        system = self.config.system_prompt or (
            "You are a helpful AI assistant. Think through problems step by step. "
            "Show your reasoning process clearly."
        )

        prompt = f"""Task: {goal}

Let's think through this step by step:
1. First, I'll identify what we need to do
2. Then, I'll work through the logic
3. Finally, I'll provide the answer

Begin:"""

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

    def _build_react_system_prompt(self) -> str:
        """Build system prompt for ReAct."""
        base = self.config.system_prompt or "You are a helpful AI assistant."

        tools_desc = ""
        if self.available_tools:
            tools_list = ", ".join(self.available_tools)
            tools_desc = f"\n\nAvailable tools: {tools_list}"

        return f"""{base}

You follow the ReAct (Reasoning + Acting) pattern:
1. Thought: Analyze the situation and decide what to do
2. Action: If needed, call a tool using JSON format: {{"tool": "tool_name", "args": {{"key": "value"}}}}
3. Observation: You'll receive the tool result
4. Repeat until you have enough information

When you have the final answer, respond with:
FINAL ANSWER: <your complete answer here>
{tools_desc}"""

    def _build_react_prompt(self, goal: str, steps: list[ThoughtStep]) -> str:
        """Build prompt for current ReAct iteration."""
        parts = [f"Goal: {goal}"]

        if steps:
            parts.append("\nPrevious steps:")
            for step in steps:
                parts.append(f"\nThought {step.step}: {step.reasoning}")
                if step.tool_call:
                    parts.append(f"Action: {step.tool_call.name}({step.tool_call.arguments})")
                if step.observation:
                    if step.observation.success:
                        parts.append(f"Observation: {step.observation.output}")
                    else:
                        parts.append(f"Observation: Error - {step.observation.error}")

        parts.append("\nWhat is your next thought or final answer?")

        return "\n".join(parts)

    def _parse_react_response(self, response: str) -> dict[str, Any]:
        """Parse LLM response to extract thought, action, or final answer."""
        response = response.strip()

        # Check for final answer
        final_match = re.search(r"FINAL ANSWER:\s*(.+)", response, re.IGNORECASE | re.DOTALL)
        if final_match:
            return {"type": "final_answer", "content": final_match.group(1).strip()}

        # Try to extract tool call JSON
        tool_call = self._extract_tool_call(response)
        if tool_call:
            # Extract thought before tool call
            thought = response.split("{")[0].strip()
            # Remove "Thought:" prefix if present
            thought = re.sub(r"^Thought\s*\d*:\s*", "", thought, flags=re.IGNORECASE)
            return {
                "type": "tool_call",
                "thought": thought,
                "tool": tool_call["tool"],
                "args": tool_call.get("args", {}),
            }

        # Just reasoning
        content = re.sub(r"^Thought\s*\d*:\s*", "", response, flags=re.IGNORECASE)
        return {"type": "reasoning", "content": content}

    def _extract_tool_call(self, text: str) -> Optional[dict]:
        """Extract tool call JSON from text."""
        # Find JSON object - handle nested braces
        # Look for balanced braces
        start_idx = text.find("{")
        if start_idx == -1:
            return None

        # Find matching closing brace
        depth = 0
        end_idx = start_idx
        for i, char in enumerate(text[start_idx:], start_idx):
            if char == "{":
                depth += 1
            elif char == "}":
                depth -= 1
                if depth == 0:
                    end_idx = i
                    break

        if depth != 0:
            return None

        json_str = text[start_idx : end_idx + 1]

        try:
            obj = json.loads(json_str)

            # Support multiple formats
            tool_name = obj.get("tool") or obj.get("action") or obj.get("name")
            if not tool_name:
                return None

            args = obj.get("args") or obj.get("arguments") or obj.get("input") or {}

            return {"tool": tool_name, "args": args}
        except json.JSONDecodeError:
            return None

    async def _execute_tool(self, tool_call: ToolCall) -> Observation:
        """Execute a tool call via context."""
        import time

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

            return Observation(
                tool_name=tool_call.name,
                success=True,
                output=output[:2000],  # Truncate long outputs
                latency_ms=latency_ms,
            )

        except Exception as e:
            latency_ms = int((time.time() - start) * 1000)
            return Observation(
                tool_name=tool_call.name,
                success=False,
                output="",
                error=str(e),
                latency_ms=latency_ms,
            )

    def _synthesize_answer(self, steps: list[ThoughtStep]) -> str:
        """Synthesize answer from steps if no explicit final answer."""
        if not steps:
            return ""

        # Collect successful observations
        observations = []
        for step in steps:
            if step.observation and step.observation.success:
                observations.append(f"- {step.observation.output[:500]}")

        if observations:
            return "Based on the gathered information:\n" + "\n".join(observations)

        # Fall back to last reasoning
        return steps[-1].reasoning


__all__ = [
    "CognitiveAgent",
    "CognitiveConfig",
    "CognitiveResult",
    "ThinkingStrategy",
    "ToolCall",
    "Observation",
    "ThoughtStep",
    "WorkingMemory",
]
