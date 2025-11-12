"""
Integration tests for Loom Python SDK with Bridge server.

These tests require a running Bridge server.
Run with: pytest -v -m integration
"""

import asyncio
import json

import pytest

from loom.agent import Agent
from loom.capability import capability
from loom.client import BridgeClient
from loom.envelope import Envelope
from loom.proto import action_pb2, bridge_pb2, event_pb2


@pytest.mark.integration
@pytest.mark.asyncio
async def test_bridge_connection(bridge_server: str) -> None:
    """Test that we can connect to the Bridge server."""
    client = BridgeClient(address=bridge_server)
    await client.connect()

    # Test heartbeat
    response = await client.heartbeat()
    assert response is not None
    assert response.status == "ok"

    await client.close()


@pytest.mark.integration
@pytest.mark.asyncio
async def test_agent_registration(bridge_server: str) -> None:
    """Test that an agent can register with the Bridge."""
    client = BridgeClient(address=bridge_server)
    await client.connect()

    # Register agent
    success = await client.register_agent(
        agent_id="test_agent_1",
        topics=["test.topic"],
        capabilities=[],
        metadata={"test": "true"},
    )
    assert success is True

    await client.close()


@pytest.mark.integration
@pytest.mark.asyncio
async def test_event_publish_and_receive(bridge_server: str) -> None:
    """Test that events can be published and received through the Bridge."""
    client = BridgeClient(address=bridge_server)
    await client.connect()

    # Register agent
    agent_id = "test_agent_2"
    test_topic = "test.roundtrip"

    await client.register_agent(
        agent_id=agent_id,
        topics=[test_topic],
        capabilities=[],
    )

    # Create outbound queue for publishing
    outbound_queue: asyncio.Queue[bridge_pb2.ClientEvent] = asyncio.Queue()

    # Start event stream
    stream = await client.event_stream(agent_id, _queue_generator(outbound_queue))

    # Publish an event
    test_event = event_pb2.Event(
        id="test_event_1",
        type="test.message",
        timestamp_ms=0,
        source=agent_id,
        payload=b"hello from python",
        confidence=1.0,
        priority=50,
    )

    await outbound_queue.put(
        bridge_pb2.ClientEvent(
            publish=bridge_pb2.Publish(
                topic=test_topic,
                event=test_event,
            )
        )
    )

    # Receive the event back (echo)
    received = False
    timeout_task = asyncio.create_task(asyncio.sleep(3.0))
    receive_task = asyncio.create_task(_receive_delivery(stream))

    done, pending = await asyncio.wait(
        [timeout_task, receive_task],
        return_when=asyncio.FIRST_COMPLETED,
    )

    for task in pending:
        task.cancel()

    if receive_task in done:
        delivery = receive_task.result()
        if delivery and delivery.event:
            assert delivery.topic == test_topic
            assert delivery.event.type == "test.message"
            assert delivery.event.payload == b"hello from python"
            received = True

    assert received, "Expected to receive the published event"

    await client.close()


@pytest.mark.integration
@pytest.mark.asyncio
async def test_agent_capability_invocation(bridge_server: str) -> None:
    """Test that agent capabilities can be invoked through the Bridge."""

    # Define a test capability
    @capability(name="test.add", version="1.0.0")
    def add_numbers(a: int, b: int) -> int:
        """Add two numbers."""
        return a + b

    # Create agent with the capability
    agent = Agent(
        agent_id="test_agent_3",
        topics=["test.capability"],
        capabilities=[add_numbers],
        address=bridge_server,
    )

    await agent.start()

    try:
        # Give agent a moment to fully register
        await asyncio.sleep(0.5)

        # Create a separate client to invoke the capability
        client = BridgeClient(address=bridge_server)
        await client.connect()

        # Forward an action call
        action_call = action_pb2.ActionCall(
            id="call_1",
            capability="test.add",
            payload=json.dumps({"a": 5, "b": 3}).encode("utf-8"),
        )

        # This should route through the ActionBroker in Core
        # For now, we're testing the basic flow
        result = await client.forward_action(action_call)

        assert result is not None
        assert result.id == "call_1"

        await client.close()
    finally:
        await agent.stop()


@pytest.mark.integration
@pytest.mark.asyncio
async def test_heartbeat(bridge_server: str) -> None:
    """Test that heartbeat mechanism works correctly."""
    client = BridgeClient(address=bridge_server)
    await client.connect()

    # Send multiple heartbeats
    for _ in range(3):
        response = await client.heartbeat()
        assert response.status == "ok"
        assert response.timestamp_ms >= 0  # Server may return 0
        await asyncio.sleep(0.1)

    await client.close()


@pytest.mark.integration
@pytest.mark.asyncio
async def test_multiple_agents_communication(bridge_server: str) -> None:
    """Test that multiple agents can communicate through the Bridge."""
    topic = "multi.agent.test"

    # Create two agents
    agent1_received = asyncio.Event()
    agent2_received = asyncio.Event()
    agent1_data = []
    agent2_data = []

    async def agent1_handler(ctx, topic_name, event):
        if event.source != "agent1":  # Ignore own events
            agent1_data.append(event)
            agent1_received.set()

    async def agent2_handler(ctx, topic_name, event):
        if event.source != "agent2":  # Ignore own events
            agent2_data.append(event)
            agent2_received.set()

    agent1 = Agent(
        agent_id="agent1",
        topics=[topic],
        address=bridge_server,
        on_event=agent1_handler,
    )

    agent2 = Agent(
        agent_id="agent2",
        topics=[topic],
        address=bridge_server,
        on_event=agent2_handler,
    )

    await agent1.start()
    await agent2.start()

    try:
        # Give agents time to register
        await asyncio.sleep(0.5)

        # Agent 1 publishes
        envelope1 = Envelope.new(
            type="test.msg",
            source="agent1",
            payload=b"from agent 1",
        )
        await agent1._ctx.emit(topic, type="test.msg", envelope=envelope1)

        # Agent 2 publishes
        envelope2 = Envelope.new(
            type="test.msg",
            source="agent2",
            payload=b"from agent 2",
        )
        await agent2._ctx.emit(topic, type="test.msg", envelope=envelope2)

        # Wait for both agents to receive messages
        try:
            await asyncio.wait_for(agent1_received.wait(), timeout=3.0)
            await asyncio.wait_for(agent2_received.wait(), timeout=3.0)
        except asyncio.TimeoutError:
            pytest.fail("Agents did not receive messages in time")

        # Verify received messages
        assert len(agent1_data) > 0, "Agent 1 should have received messages"
        assert len(agent2_data) > 0, "Agent 2 should have received messages"

    finally:
        await agent1.stop()
        await agent2.stop()


# Helper functions


async def _queue_generator(queue: asyncio.Queue):
    """Generate items from an asyncio queue."""
    while True:
        item = await queue.get()
        yield item


async def _receive_delivery(stream):
    """Receive the first delivery from a stream."""
    async for msg in stream:
        if msg.WhichOneof("msg") == "delivery":
            return msg.delivery
    return None
