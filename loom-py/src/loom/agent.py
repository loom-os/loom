from __future__ import annotations

import asyncio
import json
import logging
import os
import signal
from collections.abc import Awaitable, Iterable
from typing import Any, Callable, Optional

from opentelemetry import trace
from opentelemetry.trace import set_span_in_context

from .client import BridgeClient, pb_action, pb_bridge
from .context import Context
from .envelope import Envelope
from .tool import Tool
from .tracing import init_telemetry

EventHandler = Callable[[Context, str, Envelope], Awaitable[None]]

# Get tracer for agent spans
tracer = trace.get_tracer(__name__)


class Agent:
    def __init__(
        self,
        agent_id: str,
        topics: Iterable[str],
        tools: Optional[Iterable[Callable[..., Any]]] = None,
        address: Optional[str] = None,
        on_event: Optional[EventHandler] = None,
        # Deprecated parameter - use 'tools' instead
        capabilities: Optional[Iterable[Callable[..., Any]]] = None,
    ):
        # Auto-initialize telemetry unless explicitly disabled
        if os.getenv("LOOM_TELEMETRY_AUTO", "1") != "0":
            # Derive a sensible default service name per agent process
            svc = os.getenv("OTEL_SERVICE_NAME") or f"agent-{agent_id}"
            try:
                init_telemetry(service_name=svc)
            except Exception as e:
                logging.warning(
                    "Failed to initialize telemetry for agent %s: %s. Continuing without tracing.",
                    agent_id,
                    e,
                )

        self.agent_id = agent_id
        self.topics = list(topics)
        self._tool_decls: list[Tool] = []

        # Support both 'tools' and deprecated 'capabilities' parameter
        tool_funcs = tools or capabilities
        if tool_funcs:
            for fn in tool_funcs:
                # Check for new @tool decorator first, then legacy @capability
                t = getattr(fn, "__loom_tool__", None) or getattr(fn, "__loom_capability__", None)
                if not t:
                    raise ValueError(f"Function {fn.__name__} is not decorated with @tool")
                self._tool_decls.append(t)

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
        # Convert tools to ToolDescriptor
        tool_descriptors: list[pb_action.ToolDescriptor] = []
        for t in self._tool_decls:
            tool_descriptors.append(
                pb_action.ToolDescriptor(
                    name=t.name,
                    description=t.description,
                    parameters=t.parameters_schema,
                )
            )
        # Ensure reply topic is always subscribed
        topics = list(self.topics)
        reply_topic = f"agent.{self.agent_id}.replies"
        if reply_topic not in topics:
            topics.append(reply_topic)
        await self.client.register_agent(self.agent_id, topics, tool_descriptors)

        # Start stream
        async def outbound_iter():
            while True:
                msg = await self._outbound_queue.get()
                yield msg

        self._stream = await self.client.event_stream(self.agent_id, outbound_iter())
        self._stream_task = asyncio.create_task(self._run_stream())
        # Start heartbeat monitor
        self._heartbeat_task = asyncio.create_task(self._heartbeat_loop())

    async def _run_stream(self):
        try:
            async for server_msg in self._stream:
                which = server_msg.WhichOneof("msg")
                if which == "delivery":
                    delivery = server_msg.delivery
                    self._ctx._on_delivery(delivery)
                    # Convert proto Event -> Envelope before calling user handler for type safety
                    if self._on_event and delivery.event is not None:
                        env = Envelope.from_proto(delivery.event)

                        # Extract trace context and create child span for event handling
                        parent_ctx = env.extract_trace_context()
                        if parent_ctx:
                            ctx = set_span_in_context(trace.NonRecordingSpan(parent_ctx))
                        else:
                            ctx = None

                        # Create span for event handling with proper parent
                        with tracer.start_as_current_span(
                            "agent.on_event",
                            context=ctx,
                            attributes={
                                "agent.id": self.agent_id,
                                "event.id": env.id,
                                "event.type": env.type,
                                "topic": delivery.topic,
                                "thread_id": env.thread_id or "",
                                "correlation_id": env.correlation_id or "",
                            },
                        ):
                            await self._on_event(self._ctx, delivery.topic, env)
                elif which == "tool_call":
                    await self._handle_tool_call(server_msg.tool_call)
                elif which == "pong":
                    # ignore
                    pass
                elif which == "err":
                    # log server-side error surfaced on the stream
                    err = server_msg.err
                    logging.error(
                        "[loom] Server error on stream: %s - %s",
                        getattr(err, "code", "UNKNOWN"),
                        getattr(err, "message", ""),
                    )
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
                    tool_descriptors: list[pb_action.ToolDescriptor] = []
                    for t in self._tool_decls:
                        tool_descriptors.append(
                            pb_action.ToolDescriptor(
                                name=t.name,
                                description=t.description,
                                parameters=t.parameters_schema,
                            )
                        )
                    topics = list(self.topics)
                    reply_topic = f"agent.{self.agent_id}.replies"
                    if reply_topic not in topics:
                        topics.append(reply_topic)
                    await self.client.register_agent(self.agent_id, topics, tool_descriptors)

                    # Restart stream
                    async def outbound_iter():
                        while True:
                            msg = await self._outbound_queue.get()
                            yield msg

                    self._stream = await self.client.event_stream(self.agent_id, outbound_iter())
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

    async def _handle_tool_call(self, call: pb_action.ToolCall):
        # Route to matching tool by name
        for t in self._tool_decls:
            if t.name == call.name:
                # Parse arguments from JSON string
                args = {}
                if call.arguments:
                    try:
                        args = json.loads(call.arguments)
                    except json.JSONDecodeError as e:
                        err_res = pb_action.ToolResult(
                            call_id=call.id,
                            status=pb_action.ToolStatus.TOOL_ERROR,
                            error=pb_action.ToolError(
                                code="INVALID_ARGUMENTS",
                                message=f"Failed to parse arguments JSON: {e}",
                            ),
                        )
                        await self._outbound_queue.put(pb_bridge.ClientEvent(tool_result=err_res))
                        return

                # Validate with input model if available
                if t.input_model:
                    try:
                        args = t.input_model(**args).model_dump()
                    except Exception as e:
                        err_res = pb_action.ToolResult(
                            call_id=call.id,
                            status=pb_action.ToolStatus.TOOL_ERROR,
                            error=pb_action.ToolError(
                                code="INVALID_INPUT",
                                message=f"Failed to validate input for tool '{t.name}': {e}",
                            ),
                        )
                        await self._outbound_queue.put(pb_bridge.ClientEvent(tool_result=err_res))
                        return

                try:
                    result = t.func(**args)
                    if asyncio.iscoroutine(result):
                        result = await result
                    # Serialize output to JSON string
                    output = json.dumps(result) if result is not None else ""
                    res = pb_action.ToolResult(
                        call_id=call.id,
                        status=pb_action.ToolStatus.TOOL_OK,
                        output=output,
                    )
                except Exception as e:
                    res = pb_action.ToolResult(
                        call_id=call.id,
                        status=pb_action.ToolStatus.TOOL_ERROR,
                        error=pb_action.ToolError(code="TOOL_ERROR", message=str(e)),
                    )
                # Send back on stream by enqueuing as tool_result
                await self._outbound_queue.put(pb_bridge.ClientEvent(tool_result=res))
                return

        # No tool matched
        res = pb_action.ToolResult(
            call_id=call.id,
            status=pb_action.ToolStatus.TOOL_ERROR,
            error=pb_action.ToolError(code="NOT_FOUND", message=f"Tool '{call.name}' not found"),
        )
        await self._outbound_queue.put(pb_bridge.ClientEvent(tool_result=res))

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
