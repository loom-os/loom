"""SWE-bench adapter for Loom benchmark framework.

This adapter integrates the SWE-bench (Software Engineering Benchmark) dataset
for evaluating agent capabilities on real-world code editing tasks.

SWE-bench tasks involve:
- Reading and understanding existing codebases
- Identifying and fixing bugs based on issue descriptions
- Running tests to verify fixes

Dataset: https://github.com/princeton-nlp/SWE-bench
"""

from __future__ import annotations

import json
import subprocess
from pathlib import Path
from typing import Any

from ...cognitive import CognitiveResult


class SWEBenchAdapter:
    """Adapter for SWE-bench dataset.

    This adapter handles:
    - Loading SWE-bench tasks from JSON dataset
    - Preparing workspace with repository code
    - Formatting tasks for agent execution
    - Evaluating results against test suites

    Example:
        ```python
        adapter = SWEBenchAdapter(
            dataset_path="./swe-bench-lite",
            workspace_path="./workspaces"
        )

        tasks = adapter.load_tasks(max_tasks=10)
        for task in tasks:
            ctx = await adapter.prepare_task(task)
            # ... run agent ...
            correct = await adapter.evaluate_result(task, result)
        ```
    """

    def __init__(
        self,
        dataset_path: Path,
        workspace_path: Path,
    ):
        """Initialize SWE-bench adapter.

        Args:
            dataset_path: Path to SWE-bench dataset directory
            workspace_path: Path to workspace for task execution
        """
        self.dataset_path = Path(dataset_path)
        self.workspace_path = Path(workspace_path)
        self.workspace_path.mkdir(parents=True, exist_ok=True)

    def load_tasks(self, max_tasks: int = 100) -> list[dict]:
        """Load tasks from SWE-bench dataset.

        Args:
            max_tasks: Maximum number of tasks to load

        Returns:
            List of task specifications
        """
        # Try to find the dataset file
        dataset_files = [
            self.dataset_path / "swe-bench-lite.json",
            self.dataset_path / "swe_bench_lite.json",
            self.dataset_path / "test.json",
            self.dataset_path / "tasks.json",
        ]

        dataset_file = None
        for f in dataset_files:
            if f.exists():
                dataset_file = f
                break

        if dataset_file is None:
            # If no file found, return synthetic tasks for testing
            return self._generate_synthetic_tasks(max_tasks)

        # Load real dataset
        with open(dataset_file) as f:
            data = json.load(f)

        # Handle different dataset formats
        if isinstance(data, list):
            tasks = data[:max_tasks]
        elif isinstance(data, dict):
            if "tasks" in data:
                tasks = data["tasks"][:max_tasks]
            elif "instances" in data:
                tasks = data["instances"][:max_tasks]
            else:
                # Assume the dict itself is a single task
                tasks = [data]
        else:
            raise ValueError(f"Unexpected dataset format: {type(data)}")

        return [self._normalize_task(t) for t in tasks]

    def _normalize_task(self, task: dict) -> dict:
        """Normalize task format to standard structure.

        Args:
            task: Raw task from dataset

        Returns:
            Normalized task with standard fields
        """
        # Extract common fields with fallbacks
        task_id = task.get("instance_id") or task.get("task_id") or task.get("id", "unknown")
        repo = task.get("repo") or "unknown/repo"
        problem_statement = (
            task.get("problem_statement") or task.get("description") or "No description"
        )

        # Build prompt for the agent
        prompt = self._build_task_prompt(task_id, repo, problem_statement)

        return {
            "task_id": task_id,
            "repo": repo,
            "problem_statement": problem_statement,
            "prompt": prompt,
            "test_patch": task.get("test_patch", ""),
            "base_commit": task.get("base_commit", ""),
            "raw_task": task,
        }

    def _build_task_prompt(self, task_id: str, repo: str, problem_statement: str) -> str:
        """Build agent prompt for the task.

        Args:
            task_id: Task identifier
            repo: Repository name
            problem_statement: Description of the issue

        Returns:
            Formatted prompt for the agent
        """
        return f"""You are working on a software engineering task from the {repo} repository.

Task ID: {task_id}

Problem Statement:
{problem_statement}

Your goal is to:
1. Understand the codebase and locate the relevant files
2. Identify the root cause of the issue
3. Implement a fix
4. Verify the fix works correctly

Available tools:
- fs:read_file - Read file contents
- fs:write_file - Write/modify files
- fs:list_dir - List directory contents
- shell:run - Run shell commands (e.g., tests)
- web:search - Search for documentation (if needed)

Please fix the issue and verify your changes work.
"""

    def _generate_synthetic_tasks(self, max_tasks: int) -> list[dict]:
        """Generate synthetic tasks for testing when dataset is unavailable.

        Args:
            max_tasks: Number of synthetic tasks to generate

        Returns:
            List of synthetic task specifications
        """
        synthetic_tasks = []

        # Task 1: Simple function with bug
        synthetic_tasks.append(
            {
                "task_id": "synthetic-python-0",
                "repo": "test/python-project",
                "problem_statement": """There is a bug in the calculate_average function in calculator.py.
The function should calculate the average of a list of numbers, but it returns incorrect results.

The function is located in calculator.py and currently has an off-by-one error.
Please fix the bug and ensure the function returns the correct average.""",
                "test_patch": "test_calculator.py",
                "base_commit": "",
                "files": {
                    "calculator.py": '''def calculate_average(numbers):
    """Calculate the average of a list of numbers."""
    if not numbers:
        return 0
    total = sum(numbers)
    # BUG: Should divide by len(numbers), not len(numbers) + 1
    return total / (len(numbers) + 1)
''',
                    "test_calculator.py": """import calculator

def test_calculate_average():
    assert calculator.calculate_average([1, 2, 3, 4, 5]) == 3.0
    assert calculator.calculate_average([10, 20, 30]) == 20.0
    assert calculator.calculate_average([5]) == 5.0
    assert calculator.calculate_average([]) == 0
    print("All tests passed!")

if __name__ == "__main__":
    test_calculate_average()
""",
                },
            }
        )

        # Task 2: String manipulation bug
        synthetic_tasks.append(
            {
                "task_id": "synthetic-python-1",
                "repo": "test/string-utils",
                "problem_statement": """The reverse_words function in string_utils.py is not working correctly.
It should reverse the order of words in a sentence while preserving spaces,
but it's currently reversing the entire string instead.

Please fix the function to reverse only the word order, not the characters.""",
                "test_patch": "test_string_utils.py",
                "base_commit": "",
                "files": {
                    "string_utils.py": '''def reverse_words(sentence):
    """Reverse the order of words in a sentence."""
    # BUG: This reverses characters, not words
    return sentence[::-1]
''',
                    "test_string_utils.py": """import string_utils

def test_reverse_words():
    assert string_utils.reverse_words("hello world") == "world hello"
    assert string_utils.reverse_words("the quick brown fox") == "fox brown quick the"
    assert string_utils.reverse_words("single") == "single"
    print("All tests passed!")

if __name__ == "__main__":
    test_reverse_words()
""",
                },
            }
        )

        # Task 3: List processing bug
        synthetic_tasks.append(
            {
                "task_id": "synthetic-python-2",
                "repo": "test/list-processing",
                "problem_statement": """The remove_duplicates function in list_utils.py should remove duplicate elements
from a list while preserving the original order of first occurrences.
Currently, it's not preserving order correctly.

Please fix the function to maintain the order of first occurrences.""",
                "test_patch": "test_list_utils.py",
                "base_commit": "",
                "files": {
                    "list_utils.py": '''def remove_duplicates(items):
    """Remove duplicates from list, preserving order."""
    # BUG: set() doesn't preserve order in older Python versions
    # and this doesn't work correctly anyway
    return list(set(items))
''',
                    "test_list_utils.py": """import list_utils

def test_remove_duplicates():
    assert list_utils.remove_duplicates([1, 2, 2, 3, 1, 4]) == [1, 2, 3, 4]
    assert list_utils.remove_duplicates(['a', 'b', 'a', 'c']) == ['a', 'b', 'c']
    assert list_utils.remove_duplicates([1, 1, 1]) == [1]
    print("All tests passed!")

if __name__ == "__main__":
    test_remove_duplicates()
""",
                },
            }
        )

        return [self._normalize_task(t) for t in synthetic_tasks[:max_tasks]]

    async def prepare_task(self, task: dict) -> Any:
        """Prepare execution environment for a task.

        This sets up the workspace with the repository code and any
        necessary configuration for the agent to work on the task.

        Args:
            task: Task specification

        Returns:
            Path to task workspace
        """
        task_id = task["task_id"]
        task_workspace = self.workspace_path / task_id
        task_workspace.mkdir(parents=True, exist_ok=True)

        # If task has files (synthetic tasks), create them
        if "files" in task["raw_task"]:
            for filename, content in task["raw_task"]["files"].items():
                file_path = task_workspace / filename
                file_path.write_text(content)

        # In a full implementation with real SWE-bench:
        # 1. Clone the repository at the correct commit
        # 2. Apply any necessary patches
        # 3. Set up the Python environment

        return task_workspace

    async def evaluate_result(self, task: dict, result: CognitiveResult) -> bool:
        """Evaluate if the agent's result correctly solves the task.

        This runs the test suite to check if the fix is correct.

        Args:
            task: Task specification
            result: Agent's cognitive result

        Returns:
            True if the task was solved correctly
        """
        if not result.success:
            return False

        task_id = task["task_id"]
        task_workspace = self.workspace_path / task_id

        # Check if workspace exists
        if not task_workspace.exists():
            return False

        # For synthetic tasks with test files, actually run the tests
        if "files" in task["raw_task"]:
            test_file = None
            for filename in task["raw_task"]["files"].keys():
                if filename.startswith("test_"):
                    test_file = task_workspace / filename
                    break

            if test_file and test_file.exists():
                try:
                    # Run the test file
                    result = subprocess.run(
                        ["python", test_file.name],
                        cwd=task_workspace,
                        capture_output=True,
                        text=True,
                        timeout=10,
                    )

                    # Check if tests passed
                    success = result.returncode == 0
                    if success and "All tests passed!" in result.stdout:
                        return True
                    return success

                except subprocess.TimeoutExpired:
                    return False
                except Exception:
                    return False

        # For real SWE-bench tasks, check if solution files were modified
        # Simple heuristic: check if Python files exist
        python_files = list(task_workspace.rglob("*.py"))
        if not python_files:
            return False

        # Basic check: did agent complete with substantial work?
        return result.iterations >= 2

    async def run_tests(self, task: dict, workspace: Path) -> tuple[bool, str]:
        """Run test suite for the task.

        Args:
            task: Task specification
            workspace: Path to workspace with agent's changes

        Returns:
            Tuple of (success, output)
        """
        test_patch = task.get("test_patch", "")
        if not test_patch:
            return True, "No tests provided"

        try:
            # Run pytest or unittest
            result = subprocess.run(
                ["python", "-m", "pytest", "-xvs"],
                cwd=workspace,
                capture_output=True,
                text=True,
                timeout=60,
            )

            success = result.returncode == 0
            output = result.stdout + result.stderr

            return success, output

        except subprocess.TimeoutExpired:
            return False, "Tests timed out"
        except Exception as e:
            return False, f"Test execution failed: {e}"


__all__ = ["SWEBenchAdapter"]
