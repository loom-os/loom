#!/usr/bin/env python3
"""Test: MCP Configuration Parsing via Environment Variable

This test starts the server with a mock MCP config to verify:
1. LOOM_MCP_SERVERS environment variable is parsed
2. Server starts and begins MCP loading
3. Server logs indicate MCP loading attempt

Usage:
    python tests/test_mcp_env_parsing.py
"""

from __future__ import annotations

import asyncio
import json
import os
import subprocess
import sys
import time
from pathlib import Path
from typing import List, Tuple, Union

import pytest

sys.path.insert(0, str(Path(__file__).parent.parent / "src"))

from loom import Agent


def check_logs_for_mcp_parsing(logs: List[str], server_names: List[str]) -> Tuple[bool, List[str]]:
    """Check if MCP parsing messages appear in logs.

    Note: Since MCP loading is serial and may block on failed connections,
    we only check for the first server name.
    """
    issues = []

    # Check for MCP loading start
    found_loading = any("Loading MCP servers from LOOM_MCP_SERVERS" in log for log in logs)
    if not found_loading:
        issues.append("No 'Loading MCP servers' message found")

    # Check for at least the first server being processed
    first_server = server_names[0] if server_names else None
    if first_server:
        found_first = any(first_server in log for log in logs)
        if not found_first:
            issues.append(f"First server '{first_server}' not found in logs")

    return len(issues) == 0, issues


def start_server_with_mcp(bridge_addr: str, mcp_config: Union[dict, list]) -> subprocess.Popen:
    """Start loom-bridge-server with MCP configuration."""
    binary = Path(__file__).parent.parent.parent / "target" / "release" / "loom-bridge-server"

    if not binary.exists():
        print(f"[!] Binary not found: {binary}")
        sys.exit(1)

    env = os.environ.copy()
    env["LOOM_BRIDGE_ADDR"] = bridge_addr
    env["LOOM_MCP_SERVERS"] = json.dumps(mcp_config)
    env["RUST_LOG"] = "info,mcp_manager=debug"

    print(f"[*] Starting server on {bridge_addr}")

    proc = subprocess.Popen(
        [str(binary)],
        env=env,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
    )

    return proc


def collect_logs(proc: subprocess.Popen, timeout: float = 3.0) -> list[str]:
    """Collect logs from server for a short period."""
    logs = []
    start_time = time.time()

    import fcntl

    # Make stdout non-blocking
    fd = proc.stdout.fileno()
    flags = fcntl.fcntl(fd, fcntl.F_GETFL)
    fcntl.fcntl(fd, fcntl.F_SETFL, flags | os.O_NONBLOCK)

    while time.time() - start_time < timeout:
        if proc.poll() is not None:
            break
        try:
            line = proc.stdout.readline()
            if line:
                logs.append(line.strip())
                print(f"   [server] {line.strip()}")
        except (IOError, OSError):
            time.sleep(0.1)

    return logs


async def verify_server_responds(bridge_addr: str) -> bool:
    """Verify the server is responsive with a native tool call."""
    agent = Agent(
        agent_id="test-mcp-parsing",
        topics=["test.replies"],
        address=bridge_addr,
    )

    try:
        await agent.start()
        result = await agent._ctx.tool(
            "weather:get", payload={"location": "Paris"}, timeout_ms=10000
        )
        print(f"[✓] Server responds: weather = {result[:100]}...")
        return True
    except Exception as e:
        print(f"[✗] Server error: {e}")
        return False
    finally:
        await agent.stop()


@pytest.mark.integration
@pytest.mark.asyncio
async def test_object_format():
    """Test MCP config as object format (name is key).

    Requires compiled bridge server binary.
    """
    print("\n" + "=" * 60)
    print("Test 1: Object format (name as key) - Parsing only")
    print("=" * 60)

    bridge_addr = "127.0.0.1:50054"

    # This config won't actually connect, but should parse without error
    mcp_config = {
        "test-server-1": {"command": "echo", "args": ["hello"]},
        "test-server-2": {"command": "cat", "args": ["/dev/null"], "env": {"TEST_VAR": "value"}},
    }

    proc = start_server_with_mcp(bridge_addr, mcp_config)

    try:
        # Collect logs for a few seconds
        logs = collect_logs(proc, timeout=5.0)

        # Check for MCP parsing messages
        success, issues = check_logs_for_mcp_parsing(logs, ["test-server-1", "test-server-2"])

        if success:
            print("[✓] Object format config parsed correctly")
            return True
        else:
            for issue in issues:
                print(f"[!] {issue}")
            return False

    finally:
        proc.terminate()
        try:
            proc.wait(timeout=3)
        except subprocess.TimeoutExpired:
            proc.kill()


@pytest.mark.integration
@pytest.mark.asyncio
async def test_array_format():
    """Test MCP config as array format (name in each object).

    Requires compiled bridge server binary.
    """
    print("\n" + "=" * 60)
    print("Test 2: Array format (name in each object) - Parsing only")
    print("=" * 60)

    bridge_addr = "127.0.0.1:50055"

    mcp_config = [
        {"name": "test-server-a", "command": "echo", "args": ["test"]},
        {"name": "test-server-b", "command": "cat", "args": ["-"]},
    ]

    proc = start_server_with_mcp(bridge_addr, mcp_config)

    try:
        logs = collect_logs(proc, timeout=5.0)

        success, issues = check_logs_for_mcp_parsing(logs, ["test-server-a", "test-server-b"])

        if success:
            print("[✓] Array format config parsed correctly")
            return True
        else:
            for issue in issues:
                print(f"[!] {issue}")
            return False

    finally:
        proc.terminate()
        try:
            proc.wait(timeout=3)
        except subprocess.TimeoutExpired:
            proc.kill()


@pytest.mark.integration
@pytest.mark.asyncio
async def test_no_mcp_config():
    """Test server starts normally without MCP config.

    Requires compiled bridge server binary.
    """
    print("\n" + "=" * 60)
    print("Test 3: No MCP config (server starts normally)")
    print("=" * 60)

    bridge_addr = "127.0.0.1:50056"

    binary = Path(__file__).parent.parent.parent / "target" / "release" / "loom-bridge-server"

    env = os.environ.copy()
    env["LOOM_BRIDGE_ADDR"] = bridge_addr
    env["RUST_LOG"] = "info"
    # No LOOM_MCP_SERVERS set
    if "LOOM_MCP_SERVERS" in env:
        del env["LOOM_MCP_SERVERS"]

    proc = subprocess.Popen(
        [str(binary)],
        env=env,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
    )

    try:
        # Give server time to start
        await asyncio.sleep(2)

        if proc.poll() is not None:
            print("[✗] Server exited unexpectedly")
            return False

        server_ok = await verify_server_responds(bridge_addr)

        if server_ok:
            print("[✓] Server starts without MCP config")
            return True
        else:
            print("[✗] Server failed to respond")
            return False

    finally:
        proc.terminate()
        try:
            proc.wait(timeout=3)
        except subprocess.TimeoutExpired:
            proc.kill()


async def main():
    print("=" * 60)
    print("MCP Environment Variable Parsing Tests")
    print("=" * 60)

    results = []

    results.append(("Object format", await test_object_format()))
    results.append(("Array format", await test_array_format()))
    results.append(("No MCP config", await test_no_mcp_config()))

    print("\n" + "=" * 60)
    print("Summary")
    print("=" * 60)

    passed = 0
    for name, ok in results:
        status = "✓" if ok else "✗"
        print(f"  [{status}] {name}")
        if ok:
            passed += 1

    print(f"\n{passed}/{len(results)} tests passed")

    return 0 if passed == len(results) else 1


if __name__ == "__main__":
    exit_code = asyncio.run(main())
    sys.exit(exit_code)
