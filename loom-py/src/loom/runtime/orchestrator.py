"""
Loom Project Orchestrator

Manages the lifecycle of Loom runtime and agent processes for a project.
Provides live log streaming with Rich UI.
"""

from __future__ import annotations

import asyncio
import os
import signal
import subprocess
import sys
import tempfile
from collections import deque
from dataclasses import dataclass, field
from pathlib import Path
from typing import Optional

from rich.console import Console, Group
from rich.live import Live
from rich.panel import Panel
from rich.table import Table
from rich.text import Text

from . import embedded
from .config import load_project_config

# Console for output
console = Console()


@dataclass
class ProcessInfo:
    """Metadata for a managed process."""

    name: str
    proc: subprocess.Popen
    pid: int
    is_critical: bool = True  # If True, restart on failure
    log_lines: deque = field(default_factory=lambda: deque(maxlen=100))
    status: str = "running"  # running, stopped, error


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
    prefer_release: bool = True  # Prefer release over debug builds
    force_download: bool = False  # Force download from GitHub
    show_logs: bool = True  # Show live logs in terminal


class Orchestrator:
    """Orchestrates Loom runtime and agent processes."""

    def __init__(self, config: OrchestratorConfig):
        self.config = config
        self.runtime_proc: Optional[ProcessInfo] = None
        self.agent_procs: list[ProcessInfo] = []
        self._shutdown_requested = False
        self._bridge_addr: str = ""
        self._dashboard_url: str = ""

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

    def _create_status_panel(self) -> Panel:
        """Create status panel showing all processes."""
        table = Table(show_header=True, header_style="bold cyan", box=None, padding=(0, 1))
        table.add_column("Process", style="bold")
        table.add_column("PID", style="dim")
        table.add_column("Status")

        # Runtime row
        if self.runtime_proc:
            status_style = "green" if self.runtime_proc.status == "running" else "red"
            status_icon = "●" if self.runtime_proc.status == "running" else "○"
            table.add_row(
                "🔧 Runtime",
                str(self.runtime_proc.pid),
                Text(f"{status_icon} {self.runtime_proc.status}", style=status_style),
            )

        # Agent rows
        for agent in self.agent_procs:
            status_style = "green" if agent.status == "running" else "red"
            status_icon = "●" if agent.status == "running" else "○"
            table.add_row(
                f"🤖 {agent.name}",
                str(agent.pid),
                Text(f"{status_icon} {agent.status}", style=status_style),
            )

        # Info section
        info_lines = []
        if self._dashboard_url:
            info_lines.append(f"[cyan]Dashboard:[/cyan] {self._dashboard_url}")
        if self._bridge_addr:
            info_lines.append(f"[cyan]Bridge:[/cyan] {self._bridge_addr}")
        info_lines.append("[dim]Press Ctrl+C to stop[/dim]")

        return Panel(
            Group(table, Text("\n".join(info_lines), justify="left")),
            title="[bold white]🧠 Loom Orchestrator[/bold white]",
            border_style="cyan",
        )

    def _create_logs_panel(self) -> Panel:
        """Create panel showing recent logs from all processes."""
        # Collect recent logs from all processes
        all_logs = []

        if self.runtime_proc:
            for line in list(self.runtime_proc.log_lines)[-20:]:
                all_logs.append(("[cyan]runtime[/cyan]", line))

        for agent in self.agent_procs:
            for line in list(agent.log_lines)[-10:]:
                all_logs.append((f"[green]{agent.name}[/green]", line))

        # Sort by time (most recent last) - we just show in order added
        log_text = Text()
        for source, line in all_logs[-30:]:  # Show last 30 lines total
            log_text.append_text(Text.from_markup(f"{source}: "))
            # Truncate long lines
            display_line = line[:120] + "..." if len(line) > 120 else line
            log_text.append(display_line + "\n", style="dim")

        if not all_logs:
            log_text.append("Waiting for logs...", style="dim italic")

        return Panel(
            log_text,
            title="[bold white]📜 Live Logs[/bold white]",
            border_style="dim",
        )

    def _create_display(self) -> Group:
        """Create the full display."""
        return Group(
            self._create_status_panel(),
            self._create_logs_panel(),
        )

    async def _read_process_output(self, proc_info: ProcessInfo, stream, is_stderr: bool = False):
        """Read output from a process stream asynchronously."""
        try:
            while not self._shutdown_requested:
                line = await asyncio.get_event_loop().run_in_executor(None, stream.readline)
                if not line:
                    break
                decoded = line.decode("utf-8", errors="replace").rstrip()
                if decoded:
                    prefix = "[ERR] " if is_stderr else ""
                    proc_info.log_lines.append(f"{prefix}{decoded}")
        except Exception:
            pass

    async def start_runtime(self) -> ProcessInfo:
        """Start Loom Core or Bridge."""
        console.print("\n[bold cyan]Starting Loom Runtime...[/bold cyan]")

        # Determine bridge address
        bridge_port = self.config.bridge_port or 50051
        bridge_addr = f"127.0.0.1:{bridge_port}"
        self._bridge_addr = bridge_addr

        # Apply default telemetry envs to current process to reduce user friction
        os.environ.setdefault("OTEL_EXPORTER_OTLP_ENDPOINT", "http://localhost:4317")
        os.environ.setdefault("OTEL_TRACE_SAMPLER", "always_on")
        os.environ.setdefault("OTEL_SERVICE_NAME", "loom-runtime")

        # Set environment variables from project config
        env_vars = self.project_config.to_env_vars()
        env_vars["LOOM_BRIDGE_ADDR"] = bridge_addr

        if self.config.runtime_mode == "bridge-only":
            proc = embedded.start_bridge(
                bridge_addr,
                version=self.config.runtime_version,
                prefer_release=self.config.prefer_release,
                force_download=self.config.force_download,
                capture_output=self.config.show_logs,
            )
            console.print(f"[green]✓[/green] Bridge started (PID {proc.pid})")
            console.print(f"  [dim]Address: {bridge_addr}[/dim]")
        else:  # full mode
            dashboard_port = self.config.dashboard_port
            env_vars["LOOM_DASHBOARD"] = "true"
            env_vars["LOOM_DASHBOARD_PORT"] = str(dashboard_port)
            self._dashboard_url = f"http://localhost:{dashboard_port}"

            # Prepare MCP servers config from project config
            mcp_servers = None
            if self.project_config.mcp_servers:
                mcp_servers = {}
                for name, mcp_cfg in self.project_config.mcp_servers.items():
                    mcp_servers[name] = {
                        "command": mcp_cfg.command,
                        "args": mcp_cfg.args,
                        "env": mcp_cfg.env,
                    }
                console.print(f"  [dim]MCP Servers: {list(mcp_servers.keys())}[/dim]")

            proc = embedded.start_core(
                bridge_addr=bridge_addr,
                dashboard_port=dashboard_port,
                version=self.config.runtime_version,
                prefer_release=self.config.prefer_release,
                force_download=self.config.force_download,
                mcp_servers=mcp_servers,
                capture_output=self.config.show_logs,
            )
            console.print(f"[green]✓[/green] Core started (PID {proc.pid})")
            console.print(f"  [cyan]Dashboard:[/cyan] {self._dashboard_url}")
            console.print(f"  [cyan]Bridge:[/cyan] {bridge_addr}")

        runtime_info = ProcessInfo(
            name="loom-runtime",
            proc=proc,
            pid=proc.pid,
            is_critical=True,
            log_lines=deque(maxlen=100),
        )
        self.runtime_proc = runtime_info
        return runtime_info

    async def start_agents(self) -> list[ProcessInfo]:
        """Start all agent processes."""
        if not self.config.agent_scripts:
            console.print("[dim]No agent scripts to start[/dim]")
            return []

        console.print("\n[bold cyan]Starting Python Agents...[/bold cyan]")

        env_vars = os.environ.copy()
        env_vars.update(self.project_config.to_env_vars())
        # Ensure telemetry defaults for agents if not provided
        env_vars.setdefault("OTEL_EXPORTER_OTLP_ENDPOINT", "http://localhost:4317")
        env_vars.setdefault("OTEL_TRACE_SAMPLER", "always_on")

        agent_procs = []
        for script_path in self.config.agent_scripts:
            agent_name = script_path.stem

            # Per-agent env: MUST override service name for each agent
            env_agent = env_vars.copy()
            env_agent["OTEL_SERVICE_NAME"] = f"agent-{agent_name}"

            # Start with pipes for live log capture
            proc = subprocess.Popen(
                [sys.executable, str(script_path)],
                env=env_agent,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                cwd=self.config.project_dir,
            )

            console.print(
                f"[green]✓[/green] Agent '[bold]{agent_name}[/bold]' started (PID {proc.pid})"
            )

            proc_info = ProcessInfo(
                name=agent_name,
                proc=proc,
                pid=proc.pid,
                is_critical=False,
                log_lines=deque(maxlen=100),
            )
            agent_procs.append(proc_info)

        self.agent_procs = agent_procs
        return agent_procs

    async def _monitor_process_output(self):
        """Monitor and collect output from all processes."""
        tasks = []

        # Monitor runtime output
        if self.runtime_proc and self.runtime_proc.proc.stdout:
            tasks.append(
                asyncio.create_task(
                    self._read_process_output(self.runtime_proc, self.runtime_proc.proc.stdout)
                )
            )
        if self.runtime_proc and self.runtime_proc.proc.stderr:
            tasks.append(
                asyncio.create_task(
                    self._read_process_output(
                        self.runtime_proc, self.runtime_proc.proc.stderr, is_stderr=True
                    )
                )
            )

        # Monitor agent output
        for agent in self.agent_procs:
            if agent.proc.stdout:
                tasks.append(
                    asyncio.create_task(self._read_process_output(agent, agent.proc.stdout))
                )
            if agent.proc.stderr:
                tasks.append(
                    asyncio.create_task(
                        self._read_process_output(agent, agent.proc.stderr, is_stderr=True)
                    )
                )

        # Wait for all tasks (they will exit when processes exit or shutdown requested)
        if tasks:
            await asyncio.gather(*tasks, return_exceptions=True)

    async def monitor(self):
        """Monitor processes and update display."""
        # Start output monitoring in background
        output_task = asyncio.create_task(self._monitor_process_output())

        try:
            with Live(self._create_display(), console=console, refresh_per_second=4) as live:
                while not self._shutdown_requested:
                    # Check runtime
                    if self.runtime_proc:
                        if self.runtime_proc.proc.poll() is not None:
                            self.runtime_proc.status = "stopped"
                            console.print("[red]ERROR: Runtime process exited unexpectedly[/red]")
                            self._shutdown_requested = True
                            break

                    # Check agents
                    for agent in self.agent_procs:
                        if agent.proc.poll() is not None:
                            agent.status = "stopped"

                    # Update display
                    live.update(self._create_display())

                    await asyncio.sleep(0.25)
        finally:
            output_task.cancel()
            try:
                await output_task
            except asyncio.CancelledError:
                pass

    async def monitor_simple(self):
        """Simple monitoring without Rich Live display (fallback mode)."""
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

            await asyncio.sleep(0.5)

    async def shutdown(self):
        """Gracefully shutdown all processes."""
        console.print("\n[yellow]Shutting down...[/yellow]")

        # Terminate agents first
        for agent in self.agent_procs:
            if agent.proc.poll() is None:
                console.print(f"[dim]Stopping agent '{agent.name}'...[/dim]")
                agent.proc.terminate()

        # Wait for agents to exit
        await asyncio.sleep(1.0)

        # Force kill agents if needed
        for agent in self.agent_procs:
            if agent.proc.poll() is None:
                console.print(f"[yellow]Force killing agent '{agent.name}'...[/yellow]")
                agent.proc.kill()

        # Terminate runtime
        if self.runtime_proc and self.runtime_proc.proc.poll() is None:
            console.print("[dim]Stopping runtime...[/dim]")
            self.runtime_proc.proc.terminate()

        # Wait for runtime to exit
        await asyncio.sleep(2.0)

        # Force kill runtime if needed
        if self.runtime_proc and self.runtime_proc.proc.poll() is None:
            console.print("[yellow]Force killing runtime...[/yellow]")
            self.runtime_proc.proc.kill()

        console.print("[green]✓ Shutdown complete[/green]")

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
            console.print(
                f"\n[dim]Waiting {self.config.startup_wait_sec}s for runtime readiness...[/dim]"
            )
            await asyncio.sleep(self.config.startup_wait_sec)

            # Start agents
            await self.start_agents()

            console.print()

            # Monitor with live display
            if self.config.show_logs:
                await self.monitor()
            else:
                # Print status and wait
                total_procs = 1 + len(self.agent_procs)
                console.print(f"[green]✓ {total_procs} processes running[/green]")
                console.print("[dim]Press Ctrl+C to stop all processes[/dim]")
                await self.monitor_simple()

        finally:
            # Cleanup
            await self.shutdown()


async def run_orchestrator(config: OrchestratorConfig):
    """Run the orchestrator (convenience function)."""
    orchestrator = Orchestrator(config)
    await orchestrator.run()
