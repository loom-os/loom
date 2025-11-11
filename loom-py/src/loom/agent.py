from __future__ import annotations
import asyncio
import json
import signal
from typing import Any, Awaitable, Callable, Iterable, Optional
import logging

from .client import BridgeClient, pb_bridge, pb_action
from .context import Context
from .capability import Capability

EventHandler = Callable[[Context, str, Any], Awaitable[None]]

class Agent:
    def __init__(
        self,
        agent_id: str,
        topics: Iterable[str],
        capabilities: Optional[Iterable[Callable[..., Any]]] = None,
        address: str = None,
        on_event: Optional[EventHandler] = None,
    ):
        self.agent_id = agent_id
        self.topics = list(topics)
        self._cap_decls: list[Capability] = []
        if capabilities:
            for fn in capabilities:
                cap = getattr(fn, "__loom_capability__", None)
                if not cap:
                    raise ValueError(f"Function {fn.__name__} is not decorated with @capability")
                self._cap_decls.append(cap)
        self._on_event = on_event
        self.client = BridgeClient(address=address) if address else BridgeClient()
        self._ctx = Context(agent_id=self.agent_id, client=self.client)
        self._outbound_queue: asyncio.Queue[pb_bridge.ClientEvent] = asyncio.Queue(maxsize=1024)
        self._ctx._bind(self._outbound_queue)
        self._stream_task: Optional[asyncio.Task] = None
        self._stopped = asyncio.Event()
        self._heartbeat_task = None
        self._reconnect_lock = asyncio.Lock()

    async def start(self):
        await self.client.connect()
        # Convert capabilities
        caps: list[pb_action.CapabilityDescriptor] = []
        for c in self._cap_decls:
            caps.append(pb_action.CapabilityDescriptor(
                name=c.name,
                version=c.version,
                provider=pb_action.ProviderKind.PROVIDER_GRPC,
                metadata=c.metadata,
            ))
        # Ensure reply topic is always subscribed
        topics = list(self.topics)
        reply_topic = f"agent.{self.agent_id}.replies"
        if reply_topic not in topics:
            topics.append(reply_topic)
        await self.client.register_agent(self.agent_id, topics, caps)

        # Start stream (do not await: returns async iterator)
        async def outbound_iter():
            while True:
                msg = await self._outbound_queue.get()
                yield msg
        self._stream = self.client.event_stream(self.agent_id, outbound_iter())
        self._stream_task = asyncio.create_task(self._run_stream())
        # Start heartbeat monitor
        self._heartbeat_task = asyncio.create_task(self._heartbeat_loop())

    async def _run_stream(self):
        try:
            async for server_msg in self._stream:
                which = server_msg.WhichOneof('msg')
                if which == 'delivery':
                    delivery = server_msg.delivery
                    self._ctx._on_delivery(delivery)
                    if self._on_event and delivery.event is not None:
                        await self._on_event(self._ctx, delivery.topic, delivery.event)
                elif which == 'action_call':
                    await self._handle_action_call(server_msg.action_call)
                elif which == 'pong':
                    # ignore
                    pass
                elif which == 'err':
                    # log server-side error surfaced on the stream
                    err = server_msg.err
                    logging.error("[loom] Server error on stream: %s - %s", getattr(err, 'code', 'UNKNOWN'), getattr(err, 'message', ''))
        except Exception as e:
            logging.warning("[loom] Stream error: %s", e)
            await self._reconnect()

    async def _reconnect(self):
        if self._stopped.is_set():
            return
        async with self._reconnect_lock:
            backoff = 0.5
            while not self._stopped.is_set():
                try:
                    await self.client.close()
                    await self.client.connect()
                    # Re-register (ensure reply topic stays present)
                    caps: list[pb_action.CapabilityDescriptor] = []
                    for c in self._cap_decls:
                        caps.append(pb_action.CapabilityDescriptor(
                            name=c.name,
                            version=c.version,
                            provider=pb_action.ProviderKind.PROVIDER_GRPC,
                            metadata=c.metadata,
                        ))
                    topics = list(self.topics)
                    reply_topic = f"agent.{self.agent_id}.replies"
                    if reply_topic not in topics:
                        topics.append(reply_topic)
                    await self.client.register_agent(self.agent_id, topics, caps)
                    # Restart stream
                    async def outbound_iter():
                        while True:
                            msg = await self._outbound_queue.get()
                            yield msg
                    self._stream = self.client.event_stream(self.agent_id, outbound_iter())
                    self._stream_task = asyncio.create_task(self._run_stream())
                    logging.info("[loom] Reconnected agent %s", self.agent_id)
                    return
                except Exception as e:
                    logging.warning("[loom] Reconnect failed: %s; retrying in %.1fs", e, backoff)
                    await asyncio.sleep(backoff)
                    backoff = min(backoff * 2, 10.0)

    async def _heartbeat_loop(self):
        try:
            while not self._stopped.is_set():
                await asyncio.sleep(15)
                try:
                    await asyncio.wait_for(self.client.heartbeat(), timeout=5)
                except Exception as e:
                    logging.warning("[loom] Heartbeat failed: %s", e)
                    await self._reconnect()
        except asyncio.CancelledError:
            return

    async def _handle_action_call(self, call: pb_action.ActionCall):
        # Route to matching capability by name
        for cap in self._cap_decls:
            if cap.name == call.capability:
                payload = bytes(call.payload) if call.payload is not None else b""
                args = {}
                if cap.input_model:
                    try:
                        data = json.loads(payload.decode('utf-8')) if payload else {}
                        args = cap.input_model(**data).model_dump()
                    except Exception as e:
                        # Failed to deserialize or validate input; send an error result and abort
                        err_res = pb_action.ActionResult(
                            id=call.id,
                            status=pb_action.ActionStatus.ACTION_ERROR,
                            error=pb_action.ActionError(
                                code="INVALID_INPUT",
                                message=f"Failed to parse input for capability '{cap.name}': {e}"
                            ),
                        )
                        await self._outbound_queue.put(pb_bridge.ClientEvent(action_result=err_res))
                        return
                try:
                    result = cap.func(**args)
                    if asyncio.iscoroutine(result):
                        result = await result
                    output = (
                        json.dumps(result).encode('utf-8')
                        if result is not None and not isinstance(result, (bytes, bytearray))
                        else (result or b"")
                    )
                    res = pb_action.ActionResult(
                        id=call.id,
                        status=pb_action.ActionStatus.ACTION_OK,
                        output=output,
                    )
                except Exception as e:
                    res = pb_action.ActionResult(
                        id=call.id,
                        status=pb_action.ActionStatus.ACTION_ERROR,
                        error=pb_action.ActionError(code="CAPABILITY_ERROR", message=str(e)),
                    )
                # Send back on stream by enqueuing as action_result
                await self._outbound_queue.put(pb_bridge.ClientEvent(action_result=res))
                return
        # No capability matched
        res = pb_action.ActionResult(
            id=call.id,
            status=pb_action.ActionStatus.ACTION_ERROR,
            error=pb_action.ActionError(code="NOT_FOUND", message=f"Capability {call.capability} not found"),
        )
        await self._outbound_queue.put(pb_bridge.ClientEvent(action_result=res))

    async def stop(self):
        self._stopped.set()
        if self._stream_task:
            self._stream_task.cancel()
            try:
                await self._stream_task
            except asyncio.CancelledError:
                pass
        if self._heartbeat_task:
            self._heartbeat_task.cancel()
            try:
                await self._heartbeat_task
            except asyncio.CancelledError:
                pass
        await self.client.close()

    def run(self):
        async def _main():
            await self.start()
            # Wait for Ctrl+C
            loop = asyncio.get_running_loop()
            stop = asyncio.Event()
            for sig in (signal.SIGINT, signal.SIGTERM):
                loop.add_signal_handler(sig, stop.set)
            await stop.wait()
            await self.stop()
        asyncio.run(_main())

__all__ = ["Agent"]
