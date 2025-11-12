"""
Helper module to start and manage a Bridge server for integration tests.
"""

import os
import signal
import socket
import subprocess
import time
from pathlib import Path
from typing import Optional


def find_available_port() -> int:
    """Find an available port on localhost."""
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
        s.bind(("127.0.0.1", 0))
        s.listen(1)
        port = s.getsockname()[1]
    return port


class BridgeServerProcess:
    """Manages a Bridge server subprocess for testing."""

    def __init__(self, bridge_bin: Optional[Path] = None):
        """
        Initialize Bridge server manager.

        Args:
            bridge_bin: Path to loom-bridge-server binary. If None, will search in target/debug
        """
        if bridge_bin is None:
            # Try to find the bridge server binary
            repo_root = Path(__file__).parent.parent.parent.parent
            bridge_bin = repo_root / "target" / "debug" / "loom-bridge-server"
            if not bridge_bin.exists():
                # Try release build
                bridge_bin = repo_root / "target" / "release" / "loom-bridge-server"

        if not bridge_bin.exists():
            raise FileNotFoundError(
                f"Bridge server binary not found at {bridge_bin}. "
                "Please build it first with: cargo build -p loom-bridge --bin loom-bridge-server"
            )

        self.bridge_bin = bridge_bin
        self.process: Optional[subprocess.Popen] = None
        self.port: Optional[int] = None
        self.address: Optional[str] = None

    def start(self, port: Optional[int] = None) -> str:
        """
        Start the Bridge server.

        Args:
            port: Port to bind to. If None, finds an available port automatically.

        Returns:
            The address of the running server (e.g., "127.0.0.1:50051")
        """
        if self.process is not None:
            raise RuntimeError("Bridge server is already running")

        if port is None:
            port = find_available_port()

        self.port = port
        self.address = f"127.0.0.1:{port}"

        # Start the bridge server with specified address
        env = os.environ.copy()
        env["LOOM_BRIDGE_ADDR"] = self.address
        env["RUST_LOG"] = env.get("RUST_LOG", "loom_bridge=info,loom_core=info")

        self.process = subprocess.Popen(
            [str(self.bridge_bin)],
            env=env,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
        )

        # Wait for server to be ready
        max_wait = 5.0
        start = time.time()
        while time.time() - start < max_wait:
            if self._is_server_ready():
                return self.address
            if self.process.poll() is not None:
                # Process exited
                stdout, stderr = self.process.communicate()
                raise RuntimeError(
                    f"Bridge server failed to start:\nSTDOUT:\n{stdout}\n\nSTDERR:\n{stderr}"
                )
            time.sleep(0.1)

        # Timeout
        self.stop()
        raise TimeoutError(f"Bridge server did not start within {max_wait}s")

    def _is_server_ready(self) -> bool:
        """Check if the server is ready to accept connections."""
        if self.port is None:
            return False
        try:
            with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
                s.settimeout(0.5)
                s.connect(("127.0.0.1", self.port))
                return True
        except (socket.timeout, ConnectionRefusedError, OSError):
            return False

    def stop(self) -> None:
        """Stop the Bridge server."""
        if self.process is None:
            return

        try:
            # Try graceful shutdown first
            self.process.send_signal(signal.SIGTERM)
            try:
                self.process.wait(timeout=2.0)
            except subprocess.TimeoutExpired:
                # Force kill if graceful shutdown fails
                self.process.kill()
                self.process.wait(timeout=1.0)
        except Exception:
            # Best effort cleanup
            try:
                self.process.kill()
            except Exception:
                pass
        finally:
            self.process = None
            self.port = None
            self.address = None

    def __enter__(self):
        """Context manager entry."""
        self.start()
        return self

    def __exit__(self, exc_type, exc_val, exc_tb):
        """Context manager exit."""
        self.stop()

    async def __aenter__(self):
        """Async context manager entry."""
        self.start()
        return self

    async def __aexit__(self, exc_type, exc_val, exc_tb):
        """Async context manager exit."""
        self.stop()
