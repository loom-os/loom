"""Tests for configuration management."""

from pathlib import Path

from loom.config import (
    BridgeConfig,
    DashboardConfig,
    LLMProviderConfig,
    MCPServerConfig,
    ProjectConfig,
)


def test_bridge_config_defaults():
    """Test BridgeConfig defaults."""
    config = BridgeConfig()
    assert config.address == "127.0.0.1:50051"
    assert config.mode == "full"
    assert config.version == "latest"


def test_llm_provider_config():
    """Test LLMProviderConfig."""
    config = LLMProviderConfig(
        name="deepseek",
        type="http",
        api_key="test-key",
        api_base="https://api.deepseek.com",
        model="deepseek-chat",
    )
    assert config.name == "deepseek"
    assert config.api_key == "test-key"
    assert config.model == "deepseek-chat"


def test_mcp_server_config():
    """Test MCPServerConfig."""
    config = MCPServerConfig(
        name="web-search",
        command="mcp-server-web",
        args=["--port", "8080"],
        env={"API_KEY": "secret"},
    )
    assert config.name == "web-search"
    assert config.command == "mcp-server-web"
    assert len(config.args) == 2
    assert config.env["API_KEY"] == "secret"


def test_dashboard_config():
    """Test DashboardConfig."""
    config = DashboardConfig(enabled=True, port=3030)
    assert config.enabled
    assert config.port == 3030


def test_project_config_defaults():
    """Test ProjectConfig defaults."""
    config = ProjectConfig()
    assert config.version == "0.1.0"
    assert config.bridge.address == "127.0.0.1:50051"
    assert config.dashboard.port == 3030
    assert len(config.llm_providers) == 0
    assert len(config.mcp_servers) == 0


def test_project_config_load_simple(tmp_path):
    """Test loading simple configuration."""
    config_file = tmp_path / "loom.toml"
    config_file.write_text(
        """
name = "test-project"
version = "1.0.0"
description = "Test project"

[bridge]
address = "127.0.0.1:9999"
mode = "bridge-only"

[dashboard]
enabled = false
port = 8080
"""
    )

    config = ProjectConfig.load(config_file)
    assert config.name == "test-project"
    assert config.version == "1.0.0"
    assert config.bridge.address == "127.0.0.1:9999"
    assert config.bridge.mode == "bridge-only"
    assert not config.dashboard.enabled
    assert config.dashboard.port == 8080


def test_project_config_load_with_llm(tmp_path):
    """Test loading configuration with LLM providers."""
    config_file = tmp_path / "loom.toml"
    config_file.write_text(
        """
name = "ai-project"

[llm.deepseek]
type = "http"
api_key = "sk-test-key"
api_base = "https://api.deepseek.com"
model = "deepseek-chat"
max_tokens = 4096
temperature = 0.8

[llm.local]
type = "http"
api_base = "http://localhost:8000"
model = "qwen2.5"
"""
    )

    config = ProjectConfig.load(config_file)
    assert "deepseek" in config.llm_providers
    assert "local" in config.llm_providers

    deepseek = config.llm_providers["deepseek"]
    assert deepseek.api_key == "sk-test-key"
    assert deepseek.model == "deepseek-chat"
    assert deepseek.max_tokens == 4096

    local = config.llm_providers["local"]
    assert local.api_base == "http://localhost:8000"
    assert local.model == "qwen2.5"


def test_project_config_load_with_mcp(tmp_path):
    """Test loading configuration with MCP servers."""
    config_file = tmp_path / "loom.toml"
    config_file.write_text(
        """
[mcp.web-search]
command = "mcp-server-web"
args = ["--verbose"]

[mcp.file-system]
command = "mcp-server-fs"
args = ["--root", "/tmp"]

[mcp.file-system.env]
LOG_LEVEL = "debug"
"""
    )

    config = ProjectConfig.load(config_file)
    assert "web-search" in config.mcp_servers
    assert "file-system" in config.mcp_servers

    web = config.mcp_servers["web-search"]
    assert web.command == "mcp-server-web"
    assert "--verbose" in web.args

    fs = config.mcp_servers["file-system"]
    assert fs.command == "mcp-server-fs"
    assert "--root" in fs.args
    assert fs.env.get("LOG_LEVEL") == "debug"


def test_project_config_to_env_vars():
    """Test converting config to environment variables."""
    config = ProjectConfig()
    config.bridge.address = "127.0.0.1:9999"
    config.dashboard.enabled = True
    config.dashboard.port = 8080

    config.llm_providers["deepseek"] = LLMProviderConfig(
        name="deepseek",
        type="http",
        api_key="sk-test",
        api_base="https://api.deepseek.com",
        model="deepseek-chat",
    )

    env = config.to_env_vars()
    assert env["LOOM_BRIDGE_ADDR"] == "127.0.0.1:9999"
    assert env["LOOM_DASHBOARD"] == "true"
    assert env["LOOM_DASHBOARD_PORT"] == "8080"
    assert env["LOOM_LLM_API_KEY"] == "sk-test"
    assert env["LOOM_LLM_MODEL"] == "deepseek-chat"


def test_project_config_nonexistent_file():
    """Test loading non-existent config returns defaults."""
    config = ProjectConfig.load(Path("nonexistent.toml"))
    assert config.version == "0.1.0"
    assert config.bridge.address == "127.0.0.1:50051"
