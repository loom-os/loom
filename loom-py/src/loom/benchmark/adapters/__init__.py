"""Benchmark adapters for different datasets.

Adapters provide dataset-specific loading, preparation, and evaluation logic.
New adapters should implement:
- load_tasks(max_tasks: int) -> list[dict]
- prepare_task(task: dict) -> Path
- evaluate_result(task: dict, result: CognitiveResult) -> bool
"""

__all__ = []
