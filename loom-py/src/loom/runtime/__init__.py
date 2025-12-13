"""Runtime module - Loom Runtime management.

This module provides tools for managing the Loom runtime:
- embedded: Start Bridge/Core as embedded subprocess
- orchestrator: Manage full project lifecycle
- config: Project configuration (loom.toml)
- backend: Reusable backend agent infrastructure
"""

from .backend import BackendAgent
from .config import (
    BridgeConfig,
    DashboardConfig,
    LLMProviderConfig,
    MCPServerConfig,
    ProjectConfig,
    load_project_config,
)
from .embedded import (
    binary_path,
    cache_dir,
    get_binary,
    platform_tag,
    start_bridge,
    start_core,
)
from .orchestrator import Orchestrator, OrchestratorConfig, ProcessInfo, run_orchestrator

__all__ = [
    # Backend
    "BackendAgent",
    # Config
    "ProjectConfig",
    "BridgeConfig",
    "LLMProviderConfig",
    "MCPServerConfig",
    "DashboardConfig",
    "load_project_config",
    # Embedded
    "start_bridge",
    "start_core",
    "get_binary",
    "binary_path",
    "cache_dir",
    "platform_tag",
    # Orchestrator
    "Orchestrator",
    "OrchestratorConfig",
    "ProcessInfo",
    "run_orchestrator",
]
