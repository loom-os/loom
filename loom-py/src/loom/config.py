"""Configuration management for Loom projects.

Parses loom.toml files with support for:
- Project metadata
- Bridge connection settings
- LLM provider configuration
- MCP server configuration
- Dashboard settings

Example loom.toml structure:

    [llm.openai]
    type = "http"
    api_key = "sk-..."
    model = "gpt-4"

    [mcp.filesystem]
    command = "npx"
    args = ["-y", "@modelcontextprotocol/server-filesystem"]

    [agents.my_agent]
    role = "analyst"
    max_iterations = 10

Note: Use proper TOML tables (not string-encoded Python dictionaries).
"""

from __future__ import annotations

import ast
import os
import re
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


def _load_env_file(env_path: Path) -> None:
    """Load environment variables from .env file."""
    if not env_path.exists():
        return

    try:
        with open(env_path, "r", encoding="utf-8") as f:
            for line in f:
                line = line.strip()
                # Skip empty lines and comments
                if not line or line.startswith("#"):
                    continue

                # Parse KEY=VALUE or KEY='VALUE' or KEY="VALUE"
                if "=" in line:
                    key, _, value = line.partition("=")
                    key = key.strip()
                    value = value.strip()

                    # Remove quotes if present
                    if value and value[0] in ('"', "'") and value[-1] == value[0]:
                        value = value[1:-1]

                    # Only set if not already in environment
                    if key and key not in os.environ:
                        os.environ[key] = value
    except Exception as e:
        print(f"[loom.config] Warning: Failed to load .env file: {e}")


def _expand_env_vars(value: Any) -> Any:
    """Recursively expand ${VAR} and $VAR environment variable references."""
    if isinstance(value, str):
        # Replace ${VAR} and $VAR patterns
        def replace_var(match):
            var_name = match.group(1) or match.group(2)
            return os.environ.get(var_name, match.group(0))

        # Match ${VAR} or $VAR (but not $$)
        pattern = r"\$\{([A-Za-z_][A-Za-z0-9_]*)\}|\$([A-Za-z_][A-Za-z0-9_]*)"
        return re.sub(pattern, replace_var, value)
    elif isinstance(value, dict):
        return {k: _expand_env_vars(v) for k, v in value.items()}
    elif isinstance(value, list):
        return [_expand_env_vars(item) for item in value]
    else:
        return value


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
        """Load configuration from loom.toml file.

        Automatically searches for and loads .env files from:
        1. Same directory as loom.toml
        2. Parent directories up to project root
        3. Current working directory

        Expands ${VAR} environment variable references in the config.
        """
        if not path.exists():
            return cls()

        if toml is None:
            raise ImportError(
                "tomli is required for Python < 3.11. Install with: pip install tomli"
            )

        # Load .env files - search up the directory tree
        env_search_paths = [
            path.parent / ".env",  # Same dir as loom.toml
            Path.cwd() / ".env",  # Current working directory
        ]

        # Also search parent directories
        current = path.parent
        while current != current.parent:
            env_search_paths.append(current / ".env")
            current = current.parent

        # Load first .env file found
        for env_path in env_search_paths:
            if env_path.exists():
                _load_env_file(env_path)
                break

        try:
            raw_data = toml.loads(path.read_text(encoding="utf-8"))
            # Expand environment variables in the entire config
            data = _expand_env_vars(raw_data)
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

        # LLM providers - expect proper TOML tables
        if "llm" in data:
            for provider_name, provider_data in data["llm"].items():
                if not isinstance(provider_data, dict):
                    # Skip invalid entries - TOML should provide dictionaries
                    continue

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

        # MCP servers - expect proper TOML tables
        if "mcp" in data:
            for server_name, server_data in data["mcp"].items():
                if not isinstance(server_data, dict):
                    # Skip invalid entries - TOML should provide dictionaries
                    continue

                config.mcp_servers[server_name] = MCPServerConfig(
                    name=server_name,
                    command=server_data.get("command", ""),
                    args=server_data.get("args", []),
                    env=server_data.get("env", {}),
                )

        # Agent configs - expect proper TOML tables. Backward-compat: coerce stringified dicts.
        raw_agents = data.get("agents", {})
        agents: dict[str, Any] = {}
        for name, val in raw_agents.items():
            if isinstance(val, str):
                # Attempt to parse legacy stringified Python dicts safely
                try:
                    parsed = ast.literal_eval(val)
                    if isinstance(parsed, dict):
                        agents[name] = parsed
                    else:
                        agents[name] = {"value": parsed}
                except Exception:
                    # Keep as-is if parsing fails
                    agents[name] = {"_raw": val}
            else:
                agents[name] = val

        config.agents = agents

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
