"""Context Engineering - Token optimization and data management.

This submodule provides context engineering techniques to manage token budgets:
- **reducer**: Summarize tool execution steps
- **compactor**: Compress historical context
- **offloader**: Save large data to files
- **step**: Step representation and tracking
"""

from .compactor import CompactedHistory, CompactionConfig, StepCompactor
from .offloader import DataOffloader, OffloadConfig, OffloadResult
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

__all__ = [
    # Step
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
]
