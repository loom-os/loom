"""Memory module - Memory management for agents.

Provides different memory abstractions:
- WorkingMemory: Short-term scratchpad during cognitive loops
- InMemoryStore: Thread-scoped key-value storage
- MemoryItem/MemoryTier: Types for memory classification

Future additions (per ROADMAP):
- Short-term memory (session, ~1 hour)
- Long-term memory (persistent, RocksDB via Bridge)
- Semantic retrieval with embeddings
"""

from .store import InMemoryStore, _memory
from .types import MemoryItem, MemoryTier
from .working import WorkingMemory

__all__ = [
    # Working memory
    "WorkingMemory",
    # Key-value store
    "InMemoryStore",
    "_memory",
    # Types
    "MemoryItem",
    "MemoryTier",
]
