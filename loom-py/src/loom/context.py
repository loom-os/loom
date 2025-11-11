from __future__ import annotations
import asyncio
import json
import uuid
from typing import Any, AsyncIterator, Awaitable, Callable, Dict, Optional

from .client import BridgeClient, pb_bridge, pb_action, pb_event
from .envelope import Envelope
from .memory import _memory

EventHandler = Callable[["Context", str, Envelope], Awaitable[None]]

class Context:
    def __init__(self, agent_id: str, client: BridgeClient):
        self.agent_id = agent_id
        self.client = client
        self._pending: Dict[str, asyncio.Future[Envelope]] = {}

    # Event API
    async def emit(self, topic: str, *, type: str, payload: bytes = b"", envelope: Optional[Envelope] = None) -> None:
        env = envelope or Envelope.new(type=type, payload=payload, sender=self.agent_id)
        ev = env.to_proto(pb_event.Event)
        msg = pb_bridge.ClientEvent(publish=pb_bridge.Publish(topic=topic, event=ev))
        # Send via stream producer (in Agent)
        await self._send(msg)

    async def request(self, topic: str, *, type: str, payload: bytes = b"", timeout_ms: int = 5000) -> Envelope:
        # Create correlation id and wait for a matching reply
        env = Envelope.new(type=type, payload=payload, sender=self.agent_id)
        env.correlation_id = env.id
        env.reply_to = f"agent.{self.agent_id}.replies"
        fut: asyncio.Future[Envelope] = asyncio.get_event_loop().create_future()
        self._pending[env.correlation_id] = fut
        await self.emit(topic, type=type, payload=payload, envelope=env)
        try:
            return await asyncio.wait_for(fut, timeout=timeout_ms / 1000)
        finally:
            self._pending.pop(env.correlation_id, None)

    async def reply(self, original: Envelope, *, type: str, payload: bytes = b"") -> None:
        thread_topic = original.reply_to or f"agent.{original.sender}.replies"
        env = Envelope.new(
            type=type,
            payload=payload,
            sender=self.agent_id,
            correlation_id=original.correlation_id or original.id,
            thread_id=original.thread_id,
        )
        await self.emit(thread_topic, type=env.type, payload=env.payload, envelope=env)

    async def tool(self, name: str, *, version: str = "1.0", payload: Any = None, timeout_ms: int = 5000) -> bytes:
        data = payload
        if payload is not None and not isinstance(payload, (bytes, bytearray)):
            data = json.dumps(payload).encode("utf-8")
        call_id = str(uuid.uuid4())
        correlation_id = call_id
        call = pb_action.ActionCall(
            id=call_id,
            capability=name,
            version=version,
            payload=data or b"",
            headers={
                "x-correlation-id": correlation_id,
                "x-agent-id": self.agent_id,
            },
            timeout_ms=timeout_ms,
            correlation_id=correlation_id,
            qos=0,
        )
        res = await self.client.forward_action(call)
        if res.status == pb_action.ActionStatus.ACTION_OK:
            return bytes(res.output)
        raise RuntimeError(f"Tool call failed: {res.error.message if res.error else 'unknown'}")

    async def join_thread(self, thread_id: str) -> None:
        # MVP: use topic naming convention (doc): thread.{thread_id}.events
        # Requires subscription at registration time.
        return None

    # Internal wiring
    async def _send(self, client_event: pb_bridge.ClientEvent) -> None:
        # Agent sets this handler to push into the active stream
        if not hasattr(self, "_outbound_queue"):
            raise RuntimeError("Context not bound to Agent stream")
        await self._outbound_queue.put(client_event)

    def _bind(self, outbound_queue: "asyncio.Queue[pb_bridge.ClientEvent]") -> None:
        self._outbound_queue = outbound_queue

    def _on_delivery(self, delivery: pb_bridge.Delivery) -> None:
        if delivery.event is None:
            return
        env = Envelope.from_proto(delivery.event)
        cid = env.correlation_id
        if cid and cid in self._pending:
            fut = self._pending[cid]
            if not fut.done():
                fut.set_result(env)

__all__ = ["Context"]
