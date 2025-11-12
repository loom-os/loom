"""Test fixtures and configuration for loom-py tests."""

import asyncio
from typing import AsyncGenerator

import pytest


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
