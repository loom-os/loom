"""Context Engineering module.

This is the Python SDK's context engineering system, corresponding to
core/src/context/ in Rust Core. It provides:

**Module Organization:**

- **engineering/**: Context optimization (reducer, compactor, offloader, step)
- **prompting/**: Prompt construction (builder, few_shot, tool_descriptor)
- **memory/**: Working memory and persistent storage
- **ranking.py**: Context ranking and prioritization
- **window.py**: Token budget management

Per the Brain/Hand separation:
- Python handles context assembly, ranking, and memory strategies
- Rust Core handles persistent storage (RocksDB) via Bridge
"""

# Core utilities
# Engineering submodule
from .engineering import (
    CompactedHistory,
    CompactionConfig,
    CompactStep,
    DataOffloader,
    DefaultReducer,
    FileEditReducer,
    FileReadReducer,
    FileWriteReducer,
    OffloadConfig,
    OffloadResult,
    SearchReducer,
    ShellReducer,
    Step,
    StepCompactor,
    StepReducer,
    ToolReducer,
    WebFetchReducer,
    compute_content_hash,
    generate_step_id,
)

# Memory submodule
from .memory import InMemoryStore, MemoryItem, MemoryTier, WorkingMemory, _memory

# Prompting submodule
from .prompting import (
    ContextBuilder,
    ContextWindow,
    FewShotExample,
    FewShotLibrary,
    ToolDescriptor,
    ToolParameter,
    ToolRegistry,
    create_default_registry,
    get_default_library,
)
from .ranking import ContextRanker, ScoredItem
from .window import TokenBudget, TokenWindowManager

__all__ = [
    # Ranking & Window (main level)
    "ContextRanker",
    "ScoredItem",
    "TokenWindowManager",
    "TokenBudget",
    # Engineering
    "Step",
    "CompactStep",
    "generate_step_id",
    "compute_content_hash",
    "StepReducer",
    "ToolReducer",
    "FileReadReducer",
    "FileWriteReducer",
    "FileEditReducer",
    "ShellReducer",
    "SearchReducer",
    "WebFetchReducer",
    "DefaultReducer",
    "StepCompactor",
    "CompactionConfig",
    "CompactedHistory",
    "DataOffloader",
    "OffloadConfig",
    "OffloadResult",
    # Memory
    "WorkingMemory",
    "InMemoryStore",
    "_memory",
    "MemoryItem",
    "MemoryTier",
    # Prompting
    "ContextBuilder",
    "ContextWindow",
    "FewShotExample",
    "FewShotLibrary",
    "get_default_library",
    "ToolDescriptor",
    "ToolParameter",
    "ToolRegistry",
    "create_default_registry",
]
