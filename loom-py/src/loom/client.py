from __future__ import annotations
import asyncio
import os
from typing import AsyncIterator, Optional

import grpc

from .proto import bridge_pb2 as pb_bridge
from .proto import bridge_pb2_grpc as pb_bridge_grpc
from .proto import action_pb2 as pb_action
from .proto import event_pb2 as pb_event

DEFAULT_ADDR = "127.0.0.1:50051"  # resolved at construction time

class BridgeClient:
    def __init__(self, address: Optional[str] = None):
        # Resolve default lazily to avoid import-time env read
        self.address = address or os.environ.get("LOOM_BRIDGE_ADDR", DEFAULT_ADDR)
        self._channel: Optional[grpc.aio.Channel] = None
        self._stub: Optional[pb_bridge_grpc.BridgeStub] = None

    async def connect(self):
        if self._channel is None:
            self._channel = grpc.aio.insecure_channel(self.address)
            self._stub = pb_bridge_grpc.BridgeStub(self._channel)

    async def close(self):
        if self._channel:
            await self._channel.close()
            self._channel = None
            self._stub = None

    async def register_agent(self, agent_id: str, topics: list[str], capabilities: list[pb_action.CapabilityDescriptor], metadata: Optional[dict[str, str]] = None) -> bool:
        assert self._stub is not None
        req = pb_bridge.AgentRegisterRequest(
            agent_id=agent_id,
            subscribed_topics=topics,
            capabilities=capabilities,
            metadata=metadata or {},
        )
        resp = await self._stub.RegisterAgent(req)
        if not resp.success:
            raise RuntimeError(f"RegisterAgent failed: {resp.error_message}")
        return True

    async def event_stream(self, agent_id: str, outbound: AsyncIterator[pb_bridge.ClientEvent]):
        assert self._stub is not None
        # Handshake requires first message Ack containing agent_id
        async def _with_handshake():
            # This generator yields Ack first, then forwards from outbound
            yield pb_bridge.ClientEvent(ack=pb_bridge.Ack(message_id=agent_id))
            async for item in outbound:
                yield item
        return self._stub.EventStream(_with_handshake())

    async def forward_action(self, call: pb_action.ActionCall) -> pb_action.ActionResult:
        assert self._stub is not None
        return await self._stub.ForwardAction(call)

    async def heartbeat(self) -> pb_bridge.HeartbeatResponse:
        assert self._stub is not None
        return await self._stub.Heartbeat(pb_bridge.HeartbeatRequest())

__all__ = ["BridgeClient", "pb_bridge", "pb_event", "pb_action"]
