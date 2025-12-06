"""Context Ranker - Rank and prioritize context items.

This module provides context ranking functionality:
- Score context items by relevance
- Prioritize what fits in the token window

Corresponds to core/src/context/ranking/ in Rust Core.
Future: Add semantic similarity scoring with embeddings.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any, Callable, Optional


@dataclass
class ScoredItem:
    """A context item with a relevance score.

    Attributes:
        content: The item content
        score: Relevance score (0.0 - 1.0)
        metadata: Additional item metadata
    """

    content: str
    score: float
    metadata: dict[str, Any] | None = None


class ContextRanker:
    """Ranks context items by relevance to a query.

    Example:
        ranker = ContextRanker()
        ranked = ranker.rank(items, query="weather in Tokyo")
        top_items = ranker.select_top(ranked, max_tokens=2000)
    """

    def __init__(self, scorer: Optional[Callable[[str, str], float]] = None):
        """Initialize ranker.

        Args:
            scorer: Custom scoring function (item, query) -> score
                   Default uses simple keyword overlap
        """
        self._scorer = scorer or self._default_scorer

    @staticmethod
    def _default_scorer(item: str, query: str) -> float:
        """Simple keyword overlap scorer.

        Args:
            item: Context item content
            query: Search query

        Returns:
            Score between 0.0 and 1.0
        """
        item_words = set(item.lower().split())
        query_words = set(query.lower().split())

        if not query_words:
            return 0.5  # Neutral score if no query

        overlap = len(item_words & query_words)
        return min(1.0, overlap / len(query_words))

    def rank(self, items: list[str], query: str) -> list[ScoredItem]:
        """Rank items by relevance to query.

        Args:
            items: List of context items
            query: Search/relevance query

        Returns:
            List of ScoredItem sorted by score (descending)
        """
        scored = [ScoredItem(content=item, score=self._scorer(item, query)) for item in items]
        return sorted(scored, key=lambda x: x.score, reverse=True)

    def select_top(
        self, scored_items: list[ScoredItem], max_tokens: int, chars_per_token: int = 4
    ) -> list[str]:
        """Select top items that fit in token budget.

        Args:
            scored_items: Ranked items (highest score first)
            max_tokens: Maximum tokens allowed
            chars_per_token: Estimated characters per token

        Returns:
            List of item contents that fit in budget
        """
        selected = []
        current_tokens = 0

        for item in scored_items:
            item_tokens = len(item.content) // chars_per_token
            if current_tokens + item_tokens <= max_tokens:
                selected.append(item.content)
                current_tokens += item_tokens
            else:
                break

        return selected


__all__ = [
    "ContextRanker",
    "ScoredItem",
]
