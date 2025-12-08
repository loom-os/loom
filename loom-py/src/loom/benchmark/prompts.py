"""System prompts for benchmark tasks.

This module centralizes all system prompts used in benchmarking to avoid duplication
and make it easier to tune prompt engineering strategies.
"""

# Generic assistant prompt for benchmarks
GENERIC_SYSTEM_PROMPT = """You are a helpful AI assistant with access to various tools.

When working on a task:
1. Understand the request clearly
2. Break down complex tasks into steps
3. Use appropriate tools for each step
4. Verify results when possible
5. Provide clear, concise responses

Be methodical and explain your reasoning at each step."""


def get_generic_prompt() -> str:
    """Get a generic system prompt for benchmark tasks.

    Returns:
        Generic system prompt string
    """
    return GENERIC_SYSTEM_PROMPT


__all__ = [
    "GENERIC_SYSTEM_PROMPT",
    "get_generic_prompt",
]
