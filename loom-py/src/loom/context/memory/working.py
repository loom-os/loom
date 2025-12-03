"""Working Memory - Short-term memory for cognitive loops.

Stores conversation history and intermediate results during agent reasoning.
"""

from __future__ import annotations

from typing import Any, Optional


class WorkingMemory:
    """Working memory for the cognitive loop.

    Stores conversation history and intermediate results.
    This is the "scratchpad" during a single cognitive run.
    """

    def __init__(self, max_items: int = 50):
        """Initialize working memory.

        Args:
            max_items: Maximum number of items to keep (oldest are dropped)
        """
        self.max_items = max_items
        self._items: list[dict[str, Any]] = []

    def add(self, role: str, content: str, metadata: Optional[dict] = None) -> None:
        """Add an item to working memory.

        Args:
            role: Message role ("user", "assistant", "system")
            content: Message content
            metadata: Optional additional metadata
        """
        item = {"role": role, "content": content}
        if metadata:
            item["metadata"] = metadata
        self._items.append(item)

        # Trim if over limit
        if len(self._items) > self.max_items:
            self._items = self._items[-self.max_items :]

    def get_context(self, max_items: Optional[int] = None) -> list[dict[str, Any]]:
        """Get recent items from memory.

        Args:
            max_items: Maximum items to return (default: all)

        Returns:
            List of memory items (role, content, metadata)
        """
        n = max_items or len(self._items)
        return self._items[-n:]

    def to_messages(self) -> list[dict[str, str]]:
        """Convert memory to chat messages format.

        Returns:
            List of {"role": ..., "content": ...} dicts
        """
        return [{"role": item["role"], "content": item["content"]} for item in self._items]

    def clear(self) -> None:
        """Clear all items."""
        self._items.clear()

    def __len__(self) -> int:
        return len(self._items)


__all__ = ["WorkingMemory"]
