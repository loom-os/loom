from __future__ import annotations
from typing import Any, Dict
from collections import defaultdict

class InMemoryStore:
    """Simple thread-scoped key-value memory (MVP).

    Keys are (thread_id, key)."""

    def __init__(self):
        self._data: Dict[str, Dict[str, Any]] = defaultdict(dict)

    def put(self, thread_id: str, key: str, value: Any) -> None:
        self._data[thread_id][key] = value

    def get(self, thread_id: str, key: str, default: Any = None) -> Any:
        return self._data[thread_id].get(key, default)

    def thread(self, thread_id: str) -> Dict[str, Any]:
        return self._data[thread_id]

_memory = InMemoryStore()

__all__ = ["InMemoryStore", "_memory"]
