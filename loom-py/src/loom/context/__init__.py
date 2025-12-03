"""Context Engineering module.

This is the Python SDK's context engineering system, corresponding to
core/src/context/ in Rust Core. It provides:

- **builder**: Assemble prompts from memory and context
- **memory**: Working memory, short-term, and persistent storage
- **ranking**: Rank and prioritize context items
- **window**: Token budget management

Per the Brain/Hand separation:
- Python handles context assembly, ranking, and memory strategies
- Rust Core handles persistent storage (RocksDB) via Bridge
"""

from .builder import ContextBuilder, ContextWindow
from .memory import InMemoryStore, MemoryItem, MemoryTier, WorkingMemory, _memory
from .ranking import ContextRanker, ScoredItem
from .window import TokenBudget, TokenWindowManager

__all__ = [
    # Builder
    "ContextBuilder",
    "ContextWindow",
    # Memory
    "WorkingMemory",
    "InMemoryStore",
    "_memory",
    "MemoryItem",
    "MemoryTier",
    # Ranking
    "ContextRanker",
    "ScoredItem",
    # Window
    "TokenWindowManager",
    "TokenBudget",
]
