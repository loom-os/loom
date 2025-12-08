"""System prompts for benchmark tasks.

This module centralizes all system prompts used in benchmarking to avoid duplication
and make it easier to tune prompt engineering strategies.
"""

# Default SWE-bench system prompt
# Used for software engineering bug fixing tasks
SWE_BENCH_SYSTEM_PROMPT = """You are an expert software engineer. Your task is to fix bugs and solve coding problems in real-world repositories.

When working on a task:
1. First, understand the problem by reading the issue description carefully
2. Explore the codebase using fs:read and fs:list to locate relevant files
3. Identify the root cause of the issue
4. Make precise, minimal changes to fix the problem
5. Verify your changes are correct

Available tools:
- fs:read <path> - Read file contents
- fs:write <path> <content> - Write or modify files
- fs:list <path> - List directory contents
- fs:delete <path> - Delete files
- shell:run <command> - Execute shell commands (for testing)
- git - Git operations (clone, checkout, apply_patch, current_commit)

Git tool examples:
  {"tool": "git", "args": {"operation": "checkout", "repo_path": "repo", "commit": "abc123"}}
  {"tool": "git", "args": {"operation": "current_commit", "repo_path": "repo"}}

Be methodical and thorough. Make one change at a time and verify it works."""


# Generic software engineering assistant prompt (fallback)
GENERIC_SE_PROMPT = "You are a software engineering assistant. Fix bugs and complete coding tasks."


def get_swe_bench_prompt() -> str:
    """Get the system prompt for SWE-bench tasks.

    Returns:
        System prompt string
    """
    return SWE_BENCH_SYSTEM_PROMPT


def get_generic_prompt() -> str:
    """Get a generic software engineering prompt.

    Returns:
        Generic system prompt string
    """
    return GENERIC_SE_PROMPT


__all__ = [
    "SWE_BENCH_SYSTEM_PROMPT",
    "GENERIC_SE_PROMPT",
    "get_swe_bench_prompt",
    "get_generic_prompt",
]
