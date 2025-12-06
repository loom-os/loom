"""Context Builder - Assemble prompts from memory and context.

This module provides tools for building LLM prompts from various sources:
- Memory items (working, short-term, long-term)
- Retrieved documents
- System context

Corresponds to core/src/context/builder.rs in Rust Core.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import TYPE_CHECKING, Any, Optional

if TYPE_CHECKING:
    from .memory.types import MemoryItem


@dataclass
class ContextWindow:
    """Represents a context window for LLM input.

    Attributes:
        system_prompt: System message content
        messages: Conversation history
        max_tokens: Maximum tokens allowed
        current_tokens: Estimated current token count
    """

    system_prompt: Optional[str] = None
    messages: list[dict[str, str]] = field(default_factory=list)
    max_tokens: int = 4096
    current_tokens: int = 0


class ContextBuilder:
    """Builder for assembling LLM context from various sources.

    Example:
        builder = ContextBuilder(max_tokens=4096)
        builder.set_system("You are a helpful assistant.")
        builder.add_memory_items(working_memory.get_context())
        builder.add_retrieved_docs(docs)
        context = builder.build()
    """

    def __init__(self, max_tokens: int = 4096):
        """Initialize context builder.

        Args:
            max_tokens: Maximum tokens for the context window
        """
        self.max_tokens = max_tokens
        self._system_prompt: Optional[str] = None
        self._messages: list[dict[str, str]] = []
        self._metadata: dict[str, Any] = {}

    def set_system(self, prompt: str) -> "ContextBuilder":
        """Set the system prompt.

        Args:
            prompt: System message content

        Returns:
            Self for chaining
        """
        self._system_prompt = prompt
        return self

    def add_message(self, role: str, content: str) -> "ContextBuilder":
        """Add a single message.

        Args:
            role: Message role (user, assistant, system)
            content: Message content

        Returns:
            Self for chaining
        """
        self._messages.append({"role": role, "content": content})
        return self

    def add_memory_items(self, items: list[MemoryItem | dict]) -> "ContextBuilder":
        """Add memory items to context.

        Args:
            items: List of MemoryItem objects or dicts with role/content

        Returns:
            Self for chaining
        """
        for item in items:
            if hasattr(item, "to_message"):
                self._messages.append(item.to_message())
            else:
                self._messages.append({"role": item["role"], "content": item["content"]})
        return self

    def add_retrieved_docs(
        self, docs: list[str], prefix: str = "Context document"
    ) -> "ContextBuilder":
        """Add retrieved documents as system messages.

        Args:
            docs: List of document contents
            prefix: Prefix for document messages

        Returns:
            Self for chaining
        """
        for i, doc in enumerate(docs, 1):
            self._messages.append({"role": "system", "content": f"{prefix} {i}:\n{doc}"})
        return self

    def with_metadata(self, key: str, value: Any) -> "ContextBuilder":
        """Add metadata to the context.

        Args:
            key: Metadata key
            value: Metadata value

        Returns:
            Self for chaining
        """
        self._metadata[key] = value
        return self

    def build(self) -> ContextWindow:
        """Build the final context window.

        Returns:
            ContextWindow with assembled context
        """
        # Simple token estimation (4 chars ~= 1 token)
        token_count = 0
        if self._system_prompt:
            token_count += len(self._system_prompt) // 4

        for msg in self._messages:
            token_count += len(msg["content"]) // 4

        return ContextWindow(
            system_prompt=self._system_prompt,
            messages=self._messages.copy(),
            max_tokens=self.max_tokens,
            current_tokens=token_count,
        )

    def to_messages(self) -> list[dict[str, str]]:
        """Get messages in chat format.

        Returns:
            List of message dicts for LLM API
        """
        messages = []
        if self._system_prompt:
            messages.append({"role": "system", "content": self._system_prompt})
        messages.extend(self._messages)
        return messages


__all__ = [
    "ContextBuilder",
    "ContextWindow",
]
