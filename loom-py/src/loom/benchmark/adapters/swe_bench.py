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

        Raises:
            FileNotFoundError: If dataset file not found
            ValueError: If dataset format is invalid
        """
        # Try to find the dataset file
        dataset_files = [
            self.dataset_path / "tasks.json",
            self.dataset_path / "swe-bench-lite.json",
            self.dataset_path / "swe_bench_lite.json",
            self.dataset_path / "test.json",
        ]

        dataset_file = None
        for f in dataset_files:
            if f.exists():
                dataset_file = f
                break

        if dataset_file is None:
            raise FileNotFoundError(
                f"No SWE-bench dataset found in {self.dataset_path}. "
                "Expected one of: tasks.json, swe-bench-lite.json"
            )

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
                raise ValueError("Dataset must contain 'tasks' or 'instances' key")
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
        return f"""Fix the bug in the {repo} repository.

**Task ID**: {task_id}

**Problem Statement**:
{problem_statement}

**Instructions**:
1. The code is in the `repo/` directory
2. Explore using `fs:list` and `fs:read` to understand the codebase
3. Locate the relevant files and identify the root cause
4. Make minimal, precise changes using `fs:write`
5. You can run tests with `shell:run` if needed

Be systematic and thorough. Read the issue carefully before making changes.
"""

    async def prepare_task(self, task: dict) -> Any:
        """Prepare execution environment for a task.

        This clones the repository and checks out the specific commit
        where the bug exists.

        Args:
            task: Task specification

        Returns:
            Path to task workspace
        """
        task_id = task["task_id"]
        task_workspace = self.workspace_path / task_id
        task_workspace.mkdir(parents=True, exist_ok=True)

        # Clone the repository
        repo_name = task["repo"]
        base_commit = task.get("base_commit", "")

        if not base_commit or not repo_name:
            raise ValueError(f"Task {task_id} missing repo or base_commit")

        repo_url = f"https://github.com/{repo_name}.git"
        repo_path = task_workspace / "repo"

        if not repo_path.exists():
            # TODO: Use native git tool instead of subprocess
            # For now, use subprocess as a temporary solution
            # The git tool needs to be integrated into the Python context
            import subprocess

            try:
                # Clone repository with all branches
                print(f"  Cloning {repo_name}...")
                subprocess.run(
                    [
                        "git",
                        "clone",
                        "--depth",
                        "1",
                        "--no-single-branch",
                        repo_url,
                        str(repo_path),
                    ],
                    check=True,
                    capture_output=True,
                    timeout=60,
                )

                # Fetch and checkout specific commit
                print(f"  Checking out {base_commit[:8]}...")
                subprocess.run(
                    ["git", "fetch", "origin", base_commit],
                    cwd=repo_path,
                    check=True,
                    capture_output=True,
                    timeout=30,
                )
                subprocess.run(
                    ["git", "checkout", base_commit],
                    cwd=repo_path,
                    check=True,
                    capture_output=True,
                    timeout=10,
                )

                print(f"  ✓ Repository ready at {repo_path}")
            except subprocess.TimeoutExpired:
                raise RuntimeError(f"Timeout cloning repository {repo_name}") from None
            except subprocess.CalledProcessError as e:
                error_msg = e.stderr.decode() if e.stderr else str(e)
                raise RuntimeError(f"Git error: {error_msg}") from e

        return task_workspace

    async def evaluate_result(self, task: dict, result: CognitiveResult) -> bool:
        """Evaluate if the agent's result correctly solves the task.

        For real evaluation, this should apply the test patch and run tests.
        Currently uses a simple heuristic: check if agent completed work.

        Args:
            task: Task specification
            result: Agent's cognitive result

        Returns:
            True if the task was solved (heuristic-based)
        """
        if not result.success:
            return False

        task_id = task["task_id"]
        task_workspace = self.workspace_path / task_id
        repo_path = task_workspace / "repo"

        # Check if workspace and repo exist
        if not repo_path.exists():
            return False

        # Simple heuristic: did agent complete with substantial work?
        # A proper implementation would:
        # 1. Apply test_patch to create test suite
        # 2. Run tests and check if they pass
        # 3. Compare with gold patch
        return result.iterations >= 2


__all__ = ["SWEBenchAdapter"]
