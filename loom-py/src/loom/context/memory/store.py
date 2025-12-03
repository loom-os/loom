"""Memory Store - Thread-scoped key-value storage.

Simple in-memory store for agent state, scoped by thread/session ID.
"""

from __future__ import annotations

from collections import defaultdict
from typing import Any, Dict


class InMemoryStore:
    """Simple thread-scoped key-value memory (MVP).

    Keys are (thread_id, key) pairs. Each thread has isolated storage.

    Example:
        store = InMemoryStore()
        store.put("thread-1", "counter", 0)
        store.put("thread-1", "counter", store.get("thread-1", "counter", 0) + 1)
    """

    def __init__(self):
        self._data: Dict[str, Dict[str, Any]] = defaultdict(dict)

    def put(self, thread_id: str, key: str, value: Any) -> None:
        """Store a value for a thread.

        Args:
            thread_id: Thread/session identifier
            key: Storage key
            value: Value to store
        """
        self._data[thread_id][key] = value

    def get(self, thread_id: str, key: str, default: Any = None) -> Any:
        """Get a value for a thread.

        Args:
            thread_id: Thread/session identifier
            key: Storage key
            default: Default value if not found

        Returns:
            Stored value or default
        """
        return self._data[thread_id].get(key, default)

    def thread(self, thread_id: str) -> Dict[str, Any]:
        """Get all data for a thread.

        Args:
            thread_id: Thread/session identifier

        Returns:
            Dict of all key-value pairs for this thread
        """
        return self._data[thread_id]

    def clear_thread(self, thread_id: str) -> None:
        """Clear all data for a thread.

        Args:
            thread_id: Thread/session identifier
        """
        self._data[thread_id].clear()

    def clear_all(self) -> None:
        """Clear all data across all threads."""
        self._data.clear()


# Global singleton for simple use cases
_memory = InMemoryStore()

__all__ = ["InMemoryStore", "_memory"]
