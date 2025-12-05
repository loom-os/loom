"""Few-shot examples library for cognitive agent prompts.

This module provides curated examples of successful tool usage patterns
to help LLMs understand how to effectively use tools in the ReAct loop.
"""

from dataclasses import dataclass, field
from typing import Optional


@dataclass
class FewShotExample:
    """A single few-shot example demonstrating tool usage."""

    goal: str  # The task/goal being solved
    thought: str  # The reasoning step
    action: str  # The tool call in JSON format
    observation: str  # The result
    category: Optional[str] = None  # e.g., "filesystem", "research", "analysis"

    def format_for_prompt(self) -> str:
        """Format as ReAct-style example."""
        return f"""Thought: {self.thought}
Action: {self.action}
Observation: {self.observation}"""


@dataclass
class FewShotLibrary:
    """Collection of few-shot examples organized by category."""

    examples: list[FewShotExample] = field(default_factory=list)

    def add(self, example: FewShotExample) -> None:
        """Add an example to the library."""
        self.examples.append(example)

    def get_by_category(self, category: str) -> list[FewShotExample]:
        """Get all examples in a category."""
        return [ex for ex in self.examples if ex.category == category]

    def get_relevant(self, goal: str, max_examples: int = 3) -> list[FewShotExample]:
        """Get relevant examples for a goal.

        Simple keyword matching for now. Could be enhanced with embeddings.
        """
        keywords = goal.lower().split()
        scored = []

        for example in self.examples:
            # Score by keyword overlap
            example_text = (example.goal + " " + example.thought).lower()
            score = sum(1 for kw in keywords if kw in example_text)
            if score > 0:
                scored.append((score, example))

        # Sort by score and return top N
        scored.sort(reverse=True, key=lambda x: x[0])
        return [ex for _, ex in scored[:max_examples]]

    def format_examples(self, examples: list[FewShotExample]) -> str:
        """Format multiple examples for inclusion in prompt."""
        if not examples:
            return ""

        parts = ["Here are some examples of how to use tools effectively:\n"]
        for i, example in enumerate(examples, 1):
            parts.append(f"Example {i} (Goal: {example.goal}):")
            parts.append(example.format_for_prompt())
            parts.append("")  # Empty line between examples

        return "\n".join(parts)


def create_default_library() -> FewShotLibrary:
    """Create library with default examples."""
    library = FewShotLibrary()

    # Filesystem examples
    library.add(
        FewShotExample(
            goal="Find all TODO comments in Python files",
            thought="I need to search for the text 'TODO' in Python files. I'll use fs:search with a file pattern.",
            action='{"tool": "fs:search", "args": {"query": "TODO", "path": ".", "include": "**/*.py"}}',
            observation="Found 23 matches across 8 files: src/main.py (5 matches), src/utils.py (3 matches), ...",
            category="filesystem",
        )
    )

    library.add(
        FewShotExample(
            goal="Read the first 50 lines of a configuration file",
            thought="I should read just the beginning of the config file to understand its structure without loading too much data.",
            action='{"tool": "fs:read_file", "args": {"path": "/etc/app.conf", "start_line": 1, "end_line": 50}}',
            observation="Read app.conf (50 lines, 1.2KB)\n[config]\napi_url = https://api.example.com\n...",
            category="filesystem",
        )
    )

    # Shell examples
    library.add(
        FewShotExample(
            goal="Check if a Python project has any uncommitted changes",
            thought="I'll use git status to check for uncommitted changes in the repository.",
            action='{"tool": "shell:run", "args": {"command": "git status --short"}}',
            observation="M src/main.py\n?? temp.txt\n(2 lines of output)",
            category="shell",
        )
    )

    library.add(
        FewShotExample(
            goal="Install project dependencies",
            thought="I need to install the Python packages listed in requirements.txt. I'll use pip install.",
            action='{"tool": "shell:run", "args": {"command": "pip install -r requirements.txt", "timeout": 120}}',
            observation="Ran `pip install -r requirements.txt` → 45 lines output\nSuccessfully installed 15 packages",
            category="shell",
        )
    )

    # Web/Research examples
    library.add(
        FewShotExample(
            goal="Find documentation about Python asyncio",
            thought="I'll search the web for Python asyncio documentation to find authoritative sources.",
            action='{"tool": "web:search", "args": {"query": "Python asyncio official documentation", "max_results": 3}}',
            observation="Found 3 results:\n1. Python asyncio — Asynchronous I/O (docs.python.org)\n2. asyncio Tutorial (realpython.com)\n...",
            category="research",
        )
    )

    library.add(
        FewShotExample(
            goal="Get the latest GitHub API documentation",
            thought="I need to fetch the GitHub API documentation page to understand the endpoints.",
            action='{"tool": "web:fetch", "args": {"url": "https://docs.github.com/en/rest"}}',
            observation="Fetched docs.github.com (45KB)\nGitHub REST API Documentation\nThe REST API reference...",
            category="research",
        )
    )

    # Analysis examples
    library.add(
        FewShotExample(
            goal="Analyze test coverage of a Python project",
            thought="I'll run pytest with coverage to see which parts of the code are tested.",
            action='{"tool": "shell:run", "args": {"command": "pytest --cov=src tests/", "cwd": "/project"}}',
            observation="Ran `pytest --cov=src tests/` → 124 lines output\n====== test session starts ======\ncollected 45 items\n...\nCoverage: 87%",
            category="analysis",
        )
    )

    # Multi-step task example
    library.add(
        FewShotExample(
            goal="Find and fix Python import errors",
            thought="First, I need to identify files with import errors by searching for 'ImportError' or 'ModuleNotFoundError' in recent logs or code.",
            action='{"tool": "fs:search", "args": {"query": "ImportError|ModuleNotFoundError", "path": ".", "include": "**/*.py"}}',
            observation="Found 3 matches in 2 files: src/main.py (2 matches), tests/test_api.py (1 match)",
            category="debugging",
        )
    )

    # Error handling example
    library.add(
        FewShotExample(
            goal="Debug a failing unit test",
            thought="I should run the specific test with verbose output to see the exact failure.",
            action='{"tool": "shell:run", "args": {"command": "pytest tests/test_utils.py::test_parse_config -v"}}',
            observation="FAILED tests/test_utils.py::test_parse_config - AssertionError: Expected {...} but got None\nThe test shows that parse_config returns None for invalid input.",
            category="debugging",
        )
    )

    return library


# Singleton instance
_default_library: Optional[FewShotLibrary] = None


def get_default_library() -> FewShotLibrary:
    """Get the default few-shot library (singleton)."""
    global _default_library
    if _default_library is None:
        _default_library = create_default_library()
    return _default_library
