"""Configuration management for Loom projects.

Parses loom.toml files with support for:
- Project metadata
- Bridge connection settings
- LLM provider configuration
- MCP server configuration
- Dashboard settings
"""

from __future__ import annotations

import sys
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Optional

if sys.version_info >= (3, 11):
    import tomllib as toml  # type: ignore
else:
    try:
        import tomli as toml  # type: ignore
    except ImportError:
        toml = None  # type: ignore


@dataclass
class BridgeConfig:
    """Bridge connection configuration."""

    address: str = "127.0.0.1:50051"
    mode: str = "full"  # "bridge-only" or "full"
    version: str = "latest"
    timeout_sec: int = 30


@dataclass
class LLMProviderConfig:
    """LLM provider configuration."""

    name: str  # e.g., "deepseek", "openai", "local"
    type: str  # "http", "grpc", etc.
    api_key: Optional[str] = None
    api_base: Optional[str] = None
    model: Optional[str] = None
    max_tokens: int = 2048
    temperature: float = 0.7
    timeout_sec: int = 30
    extra: dict[str, Any] = field(default_factory=dict)


@dataclass
class MCPServerConfig:
    """MCP server configuration."""

    name: str
    command: str
    args: list[str] = field(default_factory=list)
    env: dict[str, str] = field(default_factory=dict)


@dataclass
class DashboardConfig:
    """Dashboard configuration."""

    enabled: bool = True
    port: int = 3030
    host: str = "127.0.0.1"


@dataclass
class ProjectConfig:
    """Complete Loom project configuration."""

    # Project metadata
    name: Optional[str] = None
    version: str = "0.1.0"
    description: Optional[str] = None

    # Runtime
    bridge: BridgeConfig = field(default_factory=BridgeConfig)
    dashboard: DashboardConfig = field(default_factory=DashboardConfig)

    # LLM providers (key = provider name, value = config)
    llm_providers: dict[str, LLMProviderConfig] = field(default_factory=dict)

    # MCP servers
    mcp_servers: dict[str, MCPServerConfig] = field(default_factory=dict)

    # Agent-specific settings
    agents: dict[str, Any] = field(default_factory=dict)

    @classmethod
    def load(cls, path: Path = Path("loom.toml")) -> ProjectConfig:
        """Load configuration from loom.toml file."""
        if not path.exists():
            return cls()

        if toml is None:
            raise ImportError(
                "tomli is required for Python < 3.11. Install with: pip install tomli"
            )

        try:
            data = toml.loads(path.read_text(encoding="utf-8"))
        except Exception as e:
            raise RuntimeError(f"Failed to parse {path}: {e}") from e

        config = cls()

        # Project metadata
        config.name = data.get("name")
        config.version = data.get("version", "0.1.0")
        config.description = data.get("description")

        # Bridge config
        if "bridge" in data:
            bridge_data = data["bridge"]
            config.bridge = BridgeConfig(
                address=bridge_data.get("address", "127.0.0.1:50051"),
                mode=bridge_data.get("mode", "full"),
                version=bridge_data.get("version", "latest"),
                timeout_sec=bridge_data.get("timeout_sec", 30),
            )

        # Dashboard config
        if "dashboard" in data:
            dash_data = data["dashboard"]
            config.dashboard = DashboardConfig(
                enabled=dash_data.get("enabled", True),
                port=dash_data.get("port", 3030),
                host=dash_data.get("host", "127.0.0.1"),
            )

        # LLM providers
        if "llm" in data:
            for provider_name, provider_data in data["llm"].items():
                config.llm_providers[provider_name] = LLMProviderConfig(
                    name=provider_name,
                    type=provider_data.get("type", "http"),
                    api_key=provider_data.get("api_key"),
                    api_base=provider_data.get("api_base"),
                    model=provider_data.get("model"),
                    max_tokens=provider_data.get("max_tokens", 2048),
                    temperature=provider_data.get("temperature", 0.7),
                    timeout_sec=provider_data.get("timeout_sec", 30),
                    extra=provider_data.get("extra", {}),
                )

        # MCP servers
        if "mcp" in data:
            for server_name, server_data in data["mcp"].items():
                config.mcp_servers[server_name] = MCPServerConfig(
                    name=server_name,
                    command=server_data.get("command", ""),
                    args=server_data.get("args", []),
                    env=server_data.get("env", {}),
                )

        # Agent configs
        config.agents = data.get("agents", {})

        return config

    def to_env_vars(self) -> dict[str, str]:
        """Convert configuration to environment variables for runtime."""
        env = {}

        # Bridge
        env["LOOM_BRIDGE_ADDR"] = self.bridge.address

        # Dashboard
        if self.dashboard.enabled:
            env["LOOM_DASHBOARD"] = "true"
            env["LOOM_DASHBOARD_HOST"] = self.dashboard.host
            env["LOOM_DASHBOARD_PORT"] = str(self.dashboard.port)

        # LLM providers (simplified - in practice, would need more structure)
        if self.llm_providers:
            default_provider = list(self.llm_providers.values())[0]
            if default_provider.api_key:
                env["LOOM_LLM_API_KEY"] = default_provider.api_key
            if default_provider.api_base:
                env["LOOM_LLM_API_BASE"] = default_provider.api_base
            if default_provider.model:
                env["LOOM_LLM_MODEL"] = default_provider.model

        return env


def load_project_config(start_dir: Path = Path(".")) -> ProjectConfig:
    """Load project configuration, searching up from start_dir."""
    current = start_dir.resolve()
    while current != current.parent:
        config_path = current / "loom.toml"
        if config_path.exists():
            return ProjectConfig.load(config_path)
        current = current.parent

    # No config found, return defaults
    return ProjectConfig()


__all__ = [
    "BridgeConfig",
    "LLMProviderConfig",
    "MCPServerConfig",
    "DashboardConfig",
    "ProjectConfig",
    "load_project_config",
]
