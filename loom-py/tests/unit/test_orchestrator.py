"""Tests for orchestrator."""

from pathlib import Path

import pytest

from loom.runtime.orchestrator import Orchestrator, OrchestratorConfig


def test_orchestrator_config_defaults():
    """Test OrchestratorConfig defaults."""
    config = OrchestratorConfig(project_dir=Path.cwd())
    assert config.project_dir == Path.cwd()
    assert config.runtime_mode == "full"
    assert config.runtime_version == "latest"
    assert config.dashboard_port == 3030
    assert config.startup_wait_sec == 2.0
    assert len(config.agent_scripts) == 0


def test_orchestrator_config_custom():
    """Test OrchestratorConfig with custom values."""
    project_dir = Path("/tmp/test-project")
    logs_dir = Path("/tmp/logs")
    agent_scripts = [Path("agent1.py"), Path("agent2.py")]

    config = OrchestratorConfig(
        project_dir=project_dir,
        logs_dir=logs_dir,
        runtime_mode="bridge-only",
        runtime_version="0.2.0",
        bridge_port=9999,
        dashboard_port=8080,
        startup_wait_sec=1.0,
        agent_scripts=agent_scripts,
    )

    assert config.project_dir == project_dir
    assert config.logs_dir == logs_dir
    assert config.runtime_mode == "bridge-only"
    assert config.runtime_version == "0.2.0"
    assert config.bridge_port == 9999
    assert config.dashboard_port == 8080
    assert config.startup_wait_sec == 1.0
    assert len(config.agent_scripts) == 2


def test_orchestrator_init():
    """Test Orchestrator initialization."""
    config = OrchestratorConfig(project_dir=Path.cwd())
    orch = Orchestrator(config)

    assert orch.config == config
    # project_config is loaded from loom.toml or defaults to a new ProjectConfig
    assert orch.project_config is not None
    assert orch.runtime_proc is None
    assert len(orch.agent_procs) == 0
    assert not orch._shutdown_requested


@pytest.mark.skip(reason="Requires actual runtime binaries and network")
async def test_orchestrator_start_stop():
    """Test orchestrator start/stop (integration test)."""
    config = OrchestratorConfig(
        runtime_mode="bridge-only",
        startup_wait_sec=0.5,
    )
    orch = Orchestrator(config)

    try:
        await orch.start()
        assert len(orch.processes) > 0

        # Check bridge process is running
        bridge_proc = next((p for p in orch.processes if p.name == "bridge"), None)
        assert bridge_proc is not None
        assert bridge_proc.proc.poll() is None  # Still running

    finally:
        await orch.shutdown()
        assert all(p.proc.poll() is not None for p in orch.processes)
