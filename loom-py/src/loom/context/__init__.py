"""Context Engineering module.

This is the Python SDK's context engineering system, corresponding to
core/src/context/ in Rust Core. It provides:

- **builder**: Assemble prompts from memory and context
- **memory**: Working memory, short-term, and persistent storage
- **ranking**: Rank and prioritize context items
- **window**: Token budget management
- **step**: Reduced step representations for context efficiency
- **reducer**: Per-tool reduction rules
- **compactor**: Step history compaction
- **offloader**: Large data offloading to files

Per the Brain/Hand separation:
- Python handles context assembly, ranking, and memory strategies
- Rust Core handles persistent storage (RocksDB) via Bridge
"""

from .builder import ContextBuilder, ContextWindow
from .compactor import CompactedHistory, CompactionConfig, StepCompactor
from .memory import InMemoryStore, MemoryItem, MemoryTier, WorkingMemory, _memory
from .offloader import DataOffloader, OffloadConfig, OffloadResult
from .ranking import ContextRanker, ScoredItem
from .reducer import (
    DefaultReducer,
    FileEditReducer,
    FileReadReducer,
    FileWriteReducer,
    SearchReducer,
    ShellReducer,
    StepReducer,
    ToolReducer,
    WebFetchReducer,
)
from .step import CompactStep, Step, compute_content_hash, generate_step_id
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
    # Step (Context Reduction)
    "Step",
    "CompactStep",
    "generate_step_id",
    "compute_content_hash",
    # Reducer
    "StepReducer",
    "ToolReducer",
    "FileReadReducer",
    "FileWriteReducer",
    "FileEditReducer",
    "ShellReducer",
    "SearchReducer",
    "WebFetchReducer",
    "DefaultReducer",
    # Compactor
    "StepCompactor",
    "CompactionConfig",
    "CompactedHistory",
    # Offloader
    "DataOffloader",
    "OffloadConfig",
    "OffloadResult",
    # Window
    "TokenWindowManager",
    "TokenBudget",
]
