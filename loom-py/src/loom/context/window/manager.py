"""Token Window Manager - Manage context window token limits.

This module handles token budget management:
- Estimate token counts
- Truncate/fit content to window
- Track token usage

Corresponds to core/src/context/window/ in Rust Core.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Optional


@dataclass
class TokenBudget:
    """Token budget allocation.

    Attributes:
        total: Total tokens available
        system: Tokens reserved for system prompt
        history: Tokens reserved for conversation history
        context: Tokens reserved for retrieved context
        response: Tokens reserved for response
    """

    total: int = 4096
    system: int = 500
    history: int = 1500
    context: int = 1500
    response: int = 596  # total - system - history - context


class TokenWindowManager:
    """Manages token budgets and content fitting.

    Example:
        manager = TokenWindowManager(max_tokens=4096)
        manager.set_system_budget(500)
        manager.set_history_budget(1500)

        truncated = manager.fit_content(long_text, budget=1000)
    """

    def __init__(self, max_tokens: int = 4096, chars_per_token: int = 4):
        """Initialize window manager.

        Args:
            max_tokens: Maximum context window tokens
            chars_per_token: Estimated characters per token (varies by model)
        """
        self.max_tokens = max_tokens
        self.chars_per_token = chars_per_token
        self._budget = TokenBudget(total=max_tokens)

    @property
    def budget(self) -> TokenBudget:
        """Get current budget allocation."""
        return self._budget

    def estimate_tokens(self, text: str) -> int:
        """Estimate token count for text.

        Args:
            text: Input text

        Returns:
            Estimated token count
        """
        return len(text) // self.chars_per_token

    def fit_content(self, text: str, budget: int, truncate_end: bool = True) -> str:
        """Fit content to token budget.

        Args:
            text: Content to fit
            budget: Token budget
            truncate_end: If True, truncate from end; else from start

        Returns:
            Truncated content that fits in budget
        """
        current_tokens = self.estimate_tokens(text)

        if current_tokens <= budget:
            return text

        # Calculate max characters
        max_chars = budget * self.chars_per_token

        if truncate_end:
            return text[:max_chars] + "..."
        else:
            return "..." + text[-max_chars:]

    def fit_messages(self, messages: list[dict[str, str]], budget: int) -> list[dict[str, str]]:
        """Fit message list to token budget (keeps most recent).

        Args:
            messages: List of message dicts
            budget: Token budget

        Returns:
            Truncated message list
        """
        result = []
        current_tokens = 0

        # Process from most recent to oldest
        for msg in reversed(messages):
            msg_tokens = self.estimate_tokens(msg.get("content", ""))
            if current_tokens + msg_tokens <= budget:
                result.insert(0, msg)
                current_tokens += msg_tokens
            else:
                break

        return result

    def allocate(
        self,
        system: Optional[int] = None,
        history: Optional[int] = None,
        context: Optional[int] = None,
        response: Optional[int] = None,
    ) -> TokenBudget:
        """Allocate token budget to different purposes.

        Args:
            system: Tokens for system prompt
            history: Tokens for conversation history
            context: Tokens for retrieved context
            response: Tokens for response

        Returns:
            Updated TokenBudget
        """
        if system is not None:
            self._budget.system = system
        if history is not None:
            self._budget.history = history
        if context is not None:
            self._budget.context = context
        if response is not None:
            self._budget.response = response

        return self._budget


__all__ = [
    "TokenWindowManager",
    "TokenBudget",
]
