"""Test fixtures and configuration for loom-py tests.

Test Structure:
    tests/
    ├── conftest.py          # Shared fixtures
    ├── unit/                # Unit tests (no external deps, use mocks)
    │   ├── test_cognitive.py
    │   ├── test_config.py
    │   ├── test_context.py
    │   ├── test_envelope.py
    │   ├── test_llm_provider.py
    │   ├── test_memory.py
    │   ├── test_orchestrator.py
    │   └── test_tool.py
    ├── integration/         # Integration tests (require Bridge server)
    │   ├── bridge_server.py # Server process helper
    │   ├── test_integration.py
    │   ├── test_mcp_env_parsing.py
    │   ├── test_mcp_loading.py
    │   └── test_tool_call_chain.py
    └── e2e/                 # End-to-end tests
        ├── test_embedded.py
        └── test_native_tools.py

Running tests:
    pytest tests/unit -v                    # Unit tests only
    pytest tests/integration -v -m integration  # Integration tests
    pytest tests/e2e -v                     # E2E tests
    pytest -v -m "not integration"          # All except integration
"""

import asyncio
import sys
from pathlib import Path
from typing import AsyncGenerator, Generator

import pytest

# Add tests directory to path for imports
tests_dir = Path(__file__).parent
if str(tests_dir) not in sys.path:
    sys.path.insert(0, str(tests_dir))

from integration.bridge_server import BridgeServerProcess  # noqa: E402


@pytest.fixture(scope="session")
def event_loop_policy() -> asyncio.AbstractEventLoopPolicy:
    """Use the default event loop policy for all tests."""
    return asyncio.get_event_loop_policy()


@pytest.fixture(scope="function")
async def event_loop(
    event_loop_policy: asyncio.AbstractEventLoopPolicy,
) -> AsyncGenerator[asyncio.AbstractEventLoop, None]:
    """Create a new event loop for each test function."""
    loop = event_loop_policy.new_event_loop()
    yield loop
    loop.close()


@pytest.fixture
def mock_bridge_addr() -> str:
    """Mock bridge address for testing."""
    return "127.0.0.1:50051"


@pytest.fixture(scope="module")
def bridge_server() -> Generator[str, None, None]:
    """
    Start a Bridge server for integration tests.
    Returns the server address.
    """
    server = BridgeServerProcess()
    try:
        address = server.start()
        yield address
    finally:
        server.stop()
