"""
Loom Project Orchestrator

Manages the lifecycle of Loom runtime and agent processes for a project.
"""

from __future__ import annotations

import asyncio
import os
import signal
import subprocess
import sys
import tempfile
from dataclasses import dataclass, field
from pathlib import Path
from typing import Optional

from . import embedded
from .config import load_project_config


@dataclass
class ProcessInfo:
    """Metadata for a managed process."""

    name: str
    proc: subprocess.Popen
    pid: int
    is_critical: bool = True  # If True, restart on failure


@dataclass
class OrchestratorConfig:
    """Configuration for the orchestrator."""

    project_dir: Path
    logs_dir: Optional[Path] = None
    runtime_mode: str = "full"  # "full" or "bridge-only"
    runtime_version: str = "latest"
    bridge_port: Optional[int] = None
    dashboard_port: int = 3030
    startup_wait_sec: float = 2.0
    agent_scripts: list[Path] = field(default_factory=list)


class Orchestrator:
    """Orchestrates Loom runtime and agent processes."""

    def __init__(self, config: OrchestratorConfig):
        self.config = config
        self.runtime_proc: Optional[ProcessInfo] = None
        self.agent_procs: list[ProcessInfo] = []
        self._shutdown_requested = False

        # Load project configuration
        self.project_config = load_project_config(self.config.project_dir)

        # Setup logs directory
        if self.config.logs_dir:
            self.config.logs_dir.mkdir(parents=True, exist_ok=True)

    def _get_log_file(self, name: str, stderr: bool = False) -> Path:
        """Get log file path for a process."""
        if self.config.logs_dir:
            suffix = ".err" if stderr else ".log"
            return self.config.logs_dir / f"{name}{suffix}"
        else:
            # Use system temp directory (cross-platform)
            suffix = ".err" if stderr else ".log"
            temp_dir = Path(tempfile.gettempdir())
            return temp_dir / f"loom-{name}{suffix}"

    async def start_runtime(self) -> ProcessInfo:
        """Start Loom Core or Bridge."""
        print("\n" + "=" * 58)
        print("Starting Loom Runtime")
        print("=" * 58)

        # Determine bridge address
        bridge_port = self.config.bridge_port or 50051
        bridge_addr = f"127.0.0.1:{bridge_port}"

        # Set environment variables from project config
        env_vars = self.project_config.to_env_vars()
        env_vars["LOOM_BRIDGE_ADDR"] = bridge_addr

        if self.config.runtime_mode == "bridge-only":
            proc = embedded.start_bridge(bridge_addr, version=self.config.runtime_version)
            print(f"[loom] ✓ Bridge started (PID {proc.pid})")
            print(f"[loom]   Address: {bridge_addr}")
        else:  # full mode
            dashboard_port = self.config.dashboard_port
            env_vars["LOOM_DASHBOARD"] = "true"
            env_vars["LOOM_DASHBOARD_PORT"] = str(dashboard_port)

            proc = embedded.start_core(
                bridge_addr=bridge_addr,
                dashboard_port=dashboard_port,
                version=self.config.runtime_version,
            )
            print(f"[loom] ✓ Core started (PID {proc.pid})")
            print(f"[loom]   Dashboard: http://localhost:{dashboard_port}")
            print(f"[loom]   Bridge: {bridge_addr}")

        runtime_info = ProcessInfo(name="loom-runtime", proc=proc, pid=proc.pid, is_critical=True)
        self.runtime_proc = runtime_info
        return runtime_info

    async def start_agents(self) -> list[ProcessInfo]:
        """Start all agent processes."""
        if not self.config.agent_scripts:
            print("[loom] No agent scripts to start")
            return []

        print("\n" + "=" * 58)
        print("Starting Python Agents")
        print("=" * 58)

        env_vars = os.environ.copy()
        env_vars.update(self.project_config.to_env_vars())

        agent_procs = []
        for script_path in self.config.agent_scripts:
            agent_name = script_path.stem

            # Setup log files
            stdout_log = self._get_log_file(agent_name, stderr=False)
            stderr_log = self._get_log_file(agent_name, stderr=True)

            with open(stdout_log, "w") as out_f, open(stderr_log, "w") as err_f:
                proc = subprocess.Popen(
                    [sys.executable, str(script_path)],
                    env=env_vars,
                    stdout=out_f,
                    stderr=err_f,
                    cwd=self.config.project_dir,
                )

            print(f"[loom] ✓ Agent '{agent_name}' started (PID {proc.pid})")

            proc_info = ProcessInfo(name=agent_name, proc=proc, pid=proc.pid, is_critical=False)
            agent_procs.append(proc_info)

        self.agent_procs = agent_procs
        return agent_procs

    async def monitor(self):
        """Monitor processes and handle failures."""
        while not self._shutdown_requested:
            # Check runtime
            if self.runtime_proc and self.runtime_proc.proc.poll() is not None:
                print("[loom] ERROR: Runtime process exited unexpectedly")
                self._shutdown_requested = True
                break

            # Check agents
            for agent in self.agent_procs:
                if agent.proc.poll() is not None:
                    print(f"[loom] WARNING: Agent '{agent.name}' exited (PID {agent.pid})")
                    # TODO: Implement restart logic if agent.is_critical

            await asyncio.sleep(0.5)

    async def shutdown(self):
        """Gracefully shutdown all processes."""
        print("\n[loom] Shutting down...")

        # Terminate agents first
        for agent in self.agent_procs:
            if agent.proc.poll() is None:
                print(f"[loom] Stopping agent '{agent.name}'...")
                agent.proc.terminate()

        # Wait for agents to exit
        await asyncio.sleep(1.0)

        # Force kill agents if needed
        for agent in self.agent_procs:
            if agent.proc.poll() is None:
                print(f"[loom] Force killing agent '{agent.name}'...")
                agent.proc.kill()

        # Terminate runtime
        if self.runtime_proc and self.runtime_proc.proc.poll() is None:
            print("[loom] Stopping runtime...")
            self.runtime_proc.proc.terminate()

        # Wait for runtime to exit
        await asyncio.sleep(2.0)

        # Force kill runtime if needed
        if self.runtime_proc and self.runtime_proc.proc.poll() is None:
            print("[loom] Force killing runtime...")
            self.runtime_proc.proc.kill()

        print("[loom] Shutdown complete")

    async def run(self):
        """Run the orchestrator (main entry point)."""

        # Setup signal handlers
        def signal_handler(signum, frame):
            self._shutdown_requested = True

        signal.signal(signal.SIGINT, signal_handler)
        signal.signal(signal.SIGTERM, signal_handler)

        try:
            # Start runtime
            await self.start_runtime()

            # Wait for runtime to be ready
            print(f"\n[loom] Waiting {self.config.startup_wait_sec}s for runtime readiness...")
            await asyncio.sleep(self.config.startup_wait_sec)

            # Start agents
            await self.start_agents()

            # Print status
            total_procs = 1 + len(self.agent_procs)
            print("\n" + "=" * 58)
            print("Loom Orchestrator Running")
            print("=" * 58)
            print(f"[loom] {total_procs} processes running")
            print("[loom] Press Ctrl+C to stop all processes")
            print("=" * 58 + "\n")

            # Monitor until shutdown requested
            await self.monitor()

        finally:
            # Cleanup
            await self.shutdown()


async def run_orchestrator(config: OrchestratorConfig):
    """Run the orchestrator (convenience function)."""
    orchestrator = Orchestrator(config)
    await orchestrator.run()
