"""Cognitive loop - Core reasoning logic.

This module contains the pure logic for cognitive loops:
- ReAct prompt building and parsing
- Chain-of-thought prompting
- Response parsing and tool call extraction
"""

from __future__ import annotations

import json
import re
from typing import TYPE_CHECKING, Any, Optional

if TYPE_CHECKING:
    from .types import ThoughtStep


def build_react_system_prompt(base_prompt: Optional[str], available_tools: list[str]) -> str:
    """Build system prompt for ReAct pattern.

    Args:
        base_prompt: Custom base system prompt (or None for default)
        available_tools: List of available tool names

    Returns:
        Complete system prompt for ReAct
    """
    base = base_prompt or "You are a helpful AI assistant."

    tools_desc = ""
    if available_tools:
        tools_list = ", ".join(available_tools)
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


def build_react_prompt(goal: str, steps: list[ThoughtStep]) -> str:
    """Build prompt for current ReAct iteration.

    Args:
        goal: The original goal/task
        steps: Previous reasoning steps

    Returns:
        Prompt string for the LLM
    """
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


def build_cot_prompt(goal: str) -> str:
    """Build prompt for chain-of-thought reasoning.

    Args:
        goal: The task to reason about

    Returns:
        CoT prompt string
    """
    return f"""Task: {goal}

Let's think through this step by step:
1. First, I'll identify what we need to do
2. Then, I'll work through the logic
3. Finally, I'll provide the answer

Begin:"""


def parse_react_response(response: str) -> dict[str, Any]:
    """Parse LLM response to extract thought, action, or final answer.

    Args:
        response: Raw LLM response text

    Returns:
        Dict with 'type' key and relevant content:
        - {"type": "final_answer", "content": "..."}
        - {"type": "tool_call", "thought": "...", "tool": "...", "args": {...}}
        - {"type": "reasoning", "content": "..."}
    """
    response = response.strip()

    # Check for final answer
    final_match = re.search(r"FINAL ANSWER:\s*(.+)", response, re.IGNORECASE | re.DOTALL)
    if final_match:
        return {"type": "final_answer", "content": final_match.group(1).strip()}

    # Try to extract tool call JSON
    tool_call = extract_tool_call(response)
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


def extract_tool_call(text: str) -> Optional[dict]:
    """Extract tool call JSON from text.

    Supports multiple JSON formats:
    - {"tool": "name", "args": {...}}
    - {"action": "name", "arguments": {...}}
    - {"name": "...", "input": {...}}

    Args:
        text: Text that may contain a JSON tool call

    Returns:
        Dict with 'tool' and 'args' keys, or None if not found
    """
    # Find JSON object - handle nested braces
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


def synthesize_answer(steps: list[ThoughtStep]) -> str:
    """Synthesize answer from steps if no explicit final answer.

    Args:
        steps: List of reasoning steps

    Returns:
        Synthesized answer string
    """
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
    "build_react_system_prompt",
    "build_react_prompt",
    "build_cot_prompt",
    "parse_react_response",
    "extract_tool_call",
    "synthesize_answer",
]
