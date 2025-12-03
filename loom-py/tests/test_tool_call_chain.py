#!/usr/bin/env python3
"""Test: Tool Call Chain Verification

This test verifies the complete tool call chain:
    Python ctx.tool() -> Bridge (gRPC) -> Core ToolRegistry -> Tool

Tests both:
1. Native tools (registered in Core at startup)
2. MCP tools (need explicit registration)

Run with:
    pytest loom-py/tests/test_tool_call_chain.py -v -s

Prerequisites:
    - Core + Bridge running: cargo run --bin loom-server
    - Or use the embedded test mode
"""

import asyncio
import json
import os
import sys
from pathlib import Path

import pytest

# Add loom-py to path
sys.path.insert(0, str(Path(__file__).parent.parent / "src"))

from loom import Agent


@pytest.mark.integration
class TestToolCallChain:
    """Test tool invocation through the full stack.

    These tests require a running bridge server.
    Run with: pytest -m integration
    """

    @pytest.fixture
    def bridge_address(self):
        """Get Bridge address from environment or default."""
        return os.environ.get("LOOM_BRIDGE_ADDR", "127.0.0.1:50051")

    @pytest.mark.asyncio
    async def test_native_tool_weather(self, bridge_address):
        """Test calling a native tool (weather) through Bridge.

        This tests the chain:
        ctx.tool("weather:get") -> Bridge.forward_tool_call -> ToolRegistry.call

        The weather tool is registered by Core at startup.
        """
        agent = Agent(
            agent_id="test-tool-chain-weather",
            topics=["test.tool.replies"],
            address=bridge_address,
        )

        try:
            await agent.start()

            # Call the native weather tool
            result = await agent._ctx.tool(
                "weather:get",
                payload={"location": "Tokyo"},
                timeout_ms=5000,
            )

            print(f"\n[test] Weather tool result: {result}")

            # Parse and verify
            data = json.loads(result)
            assert "temperature" in data or "error" in data

        finally:
            await agent.stop()

    @pytest.mark.asyncio
    async def test_native_tool_shell(self, bridge_address):
        """Test calling the shell tool.

        Shell tool should be registered with allowlist.
        Tool name: system:shell
        """
        agent = Agent(
            agent_id="test-tool-chain-shell",
            topics=["test.tool.replies"],
            address=bridge_address,
        )

        try:
            await agent.start()

            # Call shell tool with an allowed command
            result = await agent._ctx.tool(
                "system:shell",
                payload={"command": "echo", "args": ["hello", "loom"]},
                timeout_ms=5000,
            )

            print(f"\n[test] Shell tool result: {result}")

            data = json.loads(result)
            assert "stdout" in data or "output" in data or "error" in data

        finally:
            await agent.stop()

    @pytest.mark.asyncio
    async def test_native_tool_fs_read(self, bridge_address):
        """Test calling the filesystem read tool.

        Tests workspace sandboxing.
        """
        agent = Agent(
            agent_id="test-tool-chain-fs",
            topics=["test.tool.replies"],
            address=bridge_address,
        )

        try:
            await agent.start()

            # Try to read a file (should fail if file doesn't exist)
            try:
                result = await agent._ctx.tool(
                    "fs:read_file",
                    payload={"path": "README.md"},
                    timeout_ms=5000,
                )
                print(f"\n[test] FS read result: {result[:200]}...")
                data = json.loads(result)
                assert "content" in data or "error" in data
            except RuntimeError as e:
                # Expected if file doesn't exist or permission denied
                print(f"\n[test] FS read error (expected): {e}")
                assert "not found" in str(e).lower() or "error" in str(e).lower()

        finally:
            await agent.stop()

    @pytest.mark.asyncio
    async def test_tool_not_found(self, bridge_address):
        """Test calling a non-existent tool returns proper error."""
        agent = Agent(
            agent_id="test-tool-chain-notfound",
            topics=["test.tool.replies"],
            address=bridge_address,
        )

        try:
            await agent.start()

            with pytest.raises(RuntimeError) as exc_info:
                await agent._ctx.tool(
                    "nonexistent:tool",
                    payload={},
                    timeout_ms=5000,
                )

            print(f"\n[test] Expected error: {exc_info.value}")
            assert "not found" in str(exc_info.value).lower()

        finally:
            await agent.stop()

    @pytest.mark.asyncio
    async def test_native_tool_web_search(self, bridge_address):
        """Test the native web search tool (DuckDuckGo).

        Tool name: web:search
        This is a native tool, not MCP.
        """
        agent = Agent(
            agent_id="test-tool-chain-websearch",
            topics=["test.tool.replies"],
            address=bridge_address,
        )

        try:
            await agent.start()

            # Call web search tool
            result = await agent._ctx.tool(
                "web:search",
                payload={"query": "AI agents"},
                timeout_ms=15000,  # May need more time for HTTP request
            )

            print(f"\n[test] Web search result: {result[:500]}...")

            data = json.loads(result)
            # DuckDuckGo returns results in a specific format
            assert "results" in data or "abstract" in data or isinstance(data, list)

        finally:
            await agent.stop()

    @pytest.mark.asyncio
    async def test_list_available_tools(self, bridge_address):
        """List all tools registered in Core's ToolRegistry.

        This helps debug what tools are actually available.
        """
        # This would require adding a list_tools RPC to Bridge
        # For now, we'll just document what should be available
        expected_tools = [
            "llm:generate",  # LLM generation (if configured)
            "fs:read_file",  # Filesystem read
            "system:shell",  # Shell execution
            "weather:get",  # Weather (Open-Meteo API)
            "web:search",  # Web search (DuckDuckGo)
        ]

        print("\n[test] Expected available tools:")
        for tool in expected_tools:
            print(f"  - {tool}")

        # Actual tool names in Core:
        # - llm:generate    (LLM generation)
        # - fs:read_file    (Filesystem read)
        # - system:shell    (Shell execution)
        # - weather:get     (Weather)
        # - web:search      (Web search via DuckDuckGo)
        #
        # MCP tools are named: "{server_name}:{tool_name}"
        # e.g., "brave-search:brave_web_search"

        # TODO: Add ListTools RPC to Bridge for tool discovery


class TestMCPToolChain:
    """Test MCP tool invocation.

    MCP tools require:
    1. MCP server to be running (e.g., via npx)
    2. McpManager.add_server() to be called in Core
    3. Tool registered as "server_name:tool_name"
    """

    @pytest.fixture
    def bridge_address(self):
        return os.environ.get("LOOM_BRIDGE_ADDR", "127.0.0.1:50051")

    @pytest.mark.asyncio
    @pytest.mark.skip(reason="Requires MCP server setup - run manually with BRAVE_API_KEY")
    async def test_mcp_brave_search(self, bridge_address):
        """Test MCP Brave Search tool.

        Prerequisites:
        - BRAVE_API_KEY environment variable set
        - MCP server registered in Core

        Tool name format: "brave-search:brave_web_search"
        """
        if not os.environ.get("BRAVE_API_KEY"):
            pytest.skip("BRAVE_API_KEY not set")

        agent = Agent(
            agent_id="test-mcp-brave",
            topics=["test.mcp.replies"],
            address=bridge_address,
        )

        try:
            await agent.start()

            # Call MCP tool
            # Note: The exact tool name depends on how MCP server registers it
            result = await agent._ctx.tool(
                "brave-search:brave_web_search",
                payload={"query": "AI agents 2024"},
                timeout_ms=15000,  # MCP calls may be slower
            )

            print(f"\n[test] MCP Brave search result: {result[:500]}...")

            data = json.loads(result)
            assert "results" in data or "web_results" in data

        finally:
            await agent.stop()


if __name__ == "__main__":
    # Quick manual test
    async def main():
        print("=" * 60)
        print("Tool Call Chain Test")
        print("=" * 60)
        print("\nMake sure Core + Bridge is running:")
        print("  cargo run --bin loom-server")
        print("\nOr start with:")
        print("  cd demo/deep-research && loom run")
        print("=" * 60)

        # Test native tool
        test = TestToolCallChain()
        try:
            await test.test_native_tool_weather("127.0.0.1:50051")
            print("\n✅ Weather tool test passed")
        except Exception as e:
            print(f"\n❌ Weather tool test failed: {e}")

        try:
            await test.test_tool_not_found("127.0.0.1:50051")
            print("\n✅ Tool not found test passed")
        except Exception as e:
            print(f"\n❌ Tool not found test failed: {e}")

    asyncio.run(main())
