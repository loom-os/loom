"""Event Context - Agent's interface to Rust Core Event Bus.

This module provides the EventContext class which is the agent's handle for:
- Publishing/subscribing to events via Event Bus
- Invoking tools via Bridge
- Managing memory via Core

This is the agent's connection to Rust Core's event-driven architecture,
working together with Envelope for event encapsulation.
"""

from __future__ import annotations

import asyncio
import hashlib
import json
import time
import uuid
from typing import TYPE_CHECKING, Any, Awaitable, Callable, Dict, List, Optional

from opentelemetry import trace

from .envelope import Envelope

if TYPE_CHECKING:
    from ..bridge import BridgeClient

EventHandler = Callable[["EventContext", str, Envelope], Awaitable[None]]

# Get tracer for tool invocation spans
tracer = trace.get_tracer(__name__)


class EventContext:
    """Event context - agent's interface to Rust Core Event Bus.

    Provides event publishing, tool invocation, and memory operations.
    Each agent has one EventContext instance that handles all communication
    with Rust Core's event bus and bridge services.
    """

    def __init__(self, agent_id: str, client: BridgeClient):
        """Initialize context.

        Args:
            agent_id: This agent's ID
            client: Bridge client for communication
        """
        self.agent_id = agent_id
        self.client = client
        self._pending: Dict[str, asyncio.Future[Envelope]] = {}

    # Event API
    async def emit(
        self, topic: str, *, type: str, payload: bytes = b"", envelope: Optional[Envelope] = None
    ) -> None:
        """Emit an event to a topic.

        Args:
            topic: Topic to publish to
            type: Event type
            payload: Event payload (bytes)
            envelope: Optional pre-built envelope

        Note:
            QoS level is configured at subscription time in the Bridge (QosBatched by default),
            not per-event. The Bridge uses channel size of 2048 for batched processing.
        """
        from ..bridge.proto import bridge_pb2 as pb_bridge
        from ..bridge.proto import event_pb2 as pb_event

        env = envelope or Envelope.new(type=type, payload=payload, sender=self.agent_id)
        # Inject trace context from current span before sending
        env.inject_trace_context()
        ev = env.to_proto(pb_event.Event)
        msg = pb_bridge.ClientEvent(publish=pb_bridge.Publish(topic=topic, event=ev))
        # Send via stream producer (in Agent)
        await self._send(msg)

    async def request(
        self, topic: str, *, type: str, payload: bytes = b"", timeout_ms: int = 5000
    ) -> Envelope:
        """Send a request and wait for a reply.

        Args:
            topic: Topic to send request to
            type: Event type
            payload: Request payload
            timeout_ms: Timeout in milliseconds

        Returns:
            Reply envelope
        """
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
        """Reply to a received envelope.

        Args:
            original: The envelope being replied to
            type: Reply event type
            payload: Reply payload
        """
        thread_topic = original.reply_to or f"agent.{original.sender}.replies"
        env = Envelope.new(
            type=type,
            payload=payload,
            sender=self.agent_id,
            correlation_id=original.correlation_id or original.id,
            thread_id=original.thread_id,
        )
        await self.emit(thread_topic, type=env.type, payload=env.payload, envelope=env)

    async def tool(
        self,
        name: str,
        *,
        payload: Any = None,
        timeout_ms: int = 5000,
        headers: Optional[Dict[str, str]] = None,
    ) -> str:
        """Invoke a tool via the Bridge.

        Args:
            name: Tool name to invoke
            payload: Arguments to pass to the tool (will be JSON-serialized)
            timeout_ms: Timeout in milliseconds
            headers: Optional additional headers

        Returns:
            Tool output as JSON string
        """
        from ..bridge.proto import action_pb2 as pb_action

        # Create span for tool invocation
        with tracer.start_as_current_span(
            "tool.invoke",
            attributes={
                "tool.name": name,
                "agent.id": self.agent_id,
                "timeout.ms": timeout_ms,
            },
        ) as span:
            # Serialize payload to JSON string
            arguments = ""
            if payload is not None:
                arguments = json.dumps(payload) if not isinstance(payload, str) else payload

            call_id = str(uuid.uuid4())
            correlation_id = call_id

            # Merge custom headers with default headers
            call_headers = {
                "x-correlation-id": correlation_id,
                "x-agent-id": self.agent_id,
            }
            if headers:
                call_headers.update(headers)

            call = pb_action.ToolCall(
                id=call_id,
                name=name,
                arguments=arguments,
                headers=call_headers,
                timeout_ms=timeout_ms,
                correlation_id=correlation_id,
            )

            try:
                res = await self.client.forward_tool_call(call)

                # Record result status
                span.set_attribute("tool.status", res.status)

                if res.status == pb_action.ToolStatus.TOOL_OK:
                    span.set_attribute("tool.output.size", len(res.output))
                    span.set_status(trace.Status(trace.StatusCode.OK))
                    return res.output
                else:
                    error_msg = res.error.message if res.error else "unknown"
                    span.set_status(trace.Status(trace.StatusCode.ERROR, error_msg))
                    span.record_exception(RuntimeError(error_msg))
                    raise RuntimeError(f"Tool call failed: {error_msg}")
            except Exception as e:
                span.set_status(trace.Status(trace.StatusCode.ERROR, str(e)))
                span.record_exception(e)
                raise

    async def join_thread(self, thread_id: str) -> None:
        """Join a conversation thread (future implementation)."""
        # MVP: use topic naming convention (doc): thread.{thread_id}.events
        # Requires subscription at registration time.
        return None

    # Internal wiring
    async def _send(self, client_event) -> None:
        """Send a client event via the outbound queue."""
        if not hasattr(self, "_outbound_queue"):
            raise RuntimeError("Context not bound to Agent stream")
        await self._outbound_queue.put(client_event)

    def _bind(self, outbound_queue: asyncio.Queue) -> None:
        """Bind context to an outbound queue."""
        self._outbound_queue = outbound_queue

    def _on_delivery(self, delivery) -> None:
        """Handle a delivery from the stream."""
        if delivery.event is None:
            return
        env = Envelope.from_proto(delivery.event)
        cid = env.correlation_id
        if cid and cid in self._pending:
            fut = self._pending[cid]
            if not fut.done():
                fut.set_result(env)

    # Memory operations (Core Memory Integration)

    async def save_plan(
        self,
        symbol: str,
        action: str,
        confidence: float,
        reasoning: str = "",
        method: str = "llm",
        metadata: Optional[Dict[str, str]] = None,
    ) -> str:
        """Save a trading plan to Core memory.

        Args:
            symbol: Trading symbol (e.g., "BTC")
            action: Trading action ("BUY", "SELL", "HOLD")
            confidence: Confidence score (0.0-1.0)
            reasoning: Explanation for the decision
            method: Method used ("llm" or "rule-based")
            metadata: Additional metadata

        Returns:
            plan_hash: Unique hash for this plan
        """
        from ..bridge.proto import memory_pb2 as pb_memory

        # Generate plan hash for deduplication
        plan_content = f"{symbol}|{action}|{reasoning}"
        plan_hash = hashlib.md5(plan_content.encode()).hexdigest()[:8]

        plan = pb_memory.PlanRecord(
            timestamp_ms=int(time.time() * 1000),
            symbol=symbol,
            action=action,
            confidence=confidence,
            reasoning=reasoning,
            plan_hash=plan_hash,
            method=method,
            metadata=metadata or {},
        )

        req = pb_memory.SavePlanRequest(
            session_id=self.agent_id,
            plan=plan,
        )

        resp = await self.client.save_plan(req)
        if not resp.success:
            raise RuntimeError(f"Failed to save plan: {resp.error_message}")

        return resp.plan_hash

    async def get_recent_plans(
        self,
        symbol: str,
        limit: int = 5,
    ) -> List[Dict[str, Any]]:
        """Get recent trading plans for a symbol from Core memory.

        Args:
            symbol: Trading symbol
            limit: Maximum number of plans to retrieve

        Returns:
            List of plan dictionaries
        """
        from ..bridge.proto import memory_pb2 as pb_memory

        req = pb_memory.GetRecentPlansRequest(
            session_id=self.agent_id,
            symbol=symbol,
            limit=limit,
        )

        resp = await self.client.get_recent_plans(req)
        if not resp.success:
            raise RuntimeError(f"Failed to get recent plans: {resp.error_message}")

        plans = []
        for plan in resp.plans:
            plans.append(
                {
                    "timestamp_ms": plan.timestamp_ms,
                    "symbol": plan.symbol,
                    "action": plan.action,
                    "confidence": plan.confidence,
                    "reasoning": plan.reasoning,
                    "plan_hash": plan.plan_hash,
                    "method": plan.method,
                    "metadata": dict(plan.metadata),
                }
            )

        return plans

    async def check_duplicate_plan(
        self,
        symbol: str,
        action: str,
        reasoning: str = "",
        time_window_sec: int = 300,
    ) -> tuple[bool, Optional[Dict[str, Any]]]:
        """Check if a plan is duplicate within time window.

        Args:
            symbol: Trading symbol
            action: Trading action
            reasoning: Plan reasoning
            time_window_sec: Time window in seconds (default 300 = 5 minutes)

        Returns:
            (is_duplicate, duplicate_plan_dict or None)
        """
        from ..bridge.proto import memory_pb2 as pb_memory

        plan_content = f"{symbol}|{action}|{reasoning}"
        plan_hash = hashlib.md5(plan_content.encode()).hexdigest()[:8]

        plan = pb_memory.PlanRecord(
            timestamp_ms=int(time.time() * 1000),
            symbol=symbol,
            action=action,
            confidence=0.0,  # Not used for duplicate check
            reasoning=reasoning,
            plan_hash=plan_hash,
            method="",
            metadata={},
        )

        req = pb_memory.CheckDuplicateRequest(
            session_id=self.agent_id,
            plan=plan,
            time_window_sec=time_window_sec,
        )

        resp = await self.client.check_duplicate(req)

        if resp.is_duplicate and resp.duplicate_plan:
            dup = resp.duplicate_plan
            return True, {
                "timestamp_ms": dup.timestamp_ms,
                "symbol": dup.symbol,
                "action": dup.action,
                "confidence": dup.confidence,
                "reasoning": dup.reasoning,
                "plan_hash": dup.plan_hash,
                "time_since_ms": resp.time_since_duplicate_ms,
            }

        return False, None

    async def mark_plan_executed(
        self,
        plan_hash: str,
        symbol: str,
        action: str,
        confidence: float,
        status: str,
        executed: bool,
        order_id: str = "",
        order_size_usdt: float = 0.0,
        error_message: str = "",
    ) -> None:
        """Mark a plan as executed in Core memory (for idempotency).

        Args:
            plan_hash: Hash of the plan
            symbol: Trading symbol
            action: Trading action
            confidence: Confidence score
            status: Execution status ("success", "error", "skipped")
            executed: Whether order was actually executed
            order_id: Exchange order ID
            order_size_usdt: Order size in USDT
            error_message: Error message if failed
        """
        from ..bridge.proto import memory_pb2 as pb_memory

        execution = pb_memory.ExecutionRecord(
            timestamp_ms=int(time.time() * 1000),
            plan_hash=plan_hash,
            symbol=symbol,
            action=action,
            confidence=confidence,
            status=status,
            executed=executed,
            order_id=order_id,
            order_size_usdt=order_size_usdt,
            error_message=error_message,
        )

        req = pb_memory.MarkExecutedRequest(
            session_id=self.agent_id,
            plan_hash=plan_hash,
            execution=execution,
        )

        resp = await self.client.mark_executed(req)
        if not resp.success:
            raise RuntimeError(f"Failed to mark executed: {resp.error_message}")

    async def check_plan_executed(
        self,
        plan_hash: str,
    ) -> tuple[bool, Optional[Dict[str, Any]]]:
        """Check if a plan was already executed (idempotency check).

        Args:
            plan_hash: Hash of the plan

        Returns:
            (is_executed, execution_record_dict or None)
        """
        from ..bridge.proto import memory_pb2 as pb_memory

        req = pb_memory.CheckExecutedRequest(
            session_id=self.agent_id,
            plan_hash=plan_hash,
        )

        resp = await self.client.check_executed(req)

        if resp.is_executed and resp.execution:
            exec_rec = resp.execution
            return True, {
                "timestamp_ms": exec_rec.timestamp_ms,
                "plan_hash": exec_rec.plan_hash,
                "symbol": exec_rec.symbol,
                "action": exec_rec.action,
                "confidence": exec_rec.confidence,
                "status": exec_rec.status,
                "executed": exec_rec.executed,
                "order_id": exec_rec.order_id,
                "order_size_usdt": exec_rec.order_size_usdt,
                "error_message": exec_rec.error_message,
            }

        return False, None

    async def get_execution_stats(
        self,
        symbol: str,
    ) -> Dict[str, Any]:
        """Get execution statistics for a symbol.

        Args:
            symbol: Trading symbol

        Returns:
            Statistics dictionary with total, successful, failed counts and win rate
        """
        from ..bridge.proto import memory_pb2 as pb_memory

        req = pb_memory.GetExecutionStatsRequest(
            session_id=self.agent_id,
            symbol=symbol,
        )

        resp = await self.client.get_execution_stats(req)

        recent_executions = []
        for exec_rec in resp.recent_executions:
            recent_executions.append(
                {
                    "timestamp_ms": exec_rec.timestamp_ms,
                    "plan_hash": exec_rec.plan_hash,
                    "symbol": exec_rec.symbol,
                    "action": exec_rec.action,
                    "status": exec_rec.status,
                    "executed": exec_rec.executed,
                    "order_id": exec_rec.order_id,
                }
            )

        return {
            "total_executions": resp.total_executions,
            "successful_executions": resp.successful_executions,
            "failed_executions": resp.failed_executions,
            "win_rate": resp.win_rate,
            "duplicate_prevented": resp.duplicate_prevented,
            "recent_executions": recent_executions,
        }


__all__ = ["EventContext"]
