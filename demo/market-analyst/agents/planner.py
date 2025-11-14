"""Planner Agent - Analysis Aggregation & Planning

Aggregates partial analyses and generates trading plans.
Integrates with LLM (DeepSeek) for intelligent reasoning.
"""

import asyncio
import json
import time
from collections import defaultdict
from dataclasses import dataclass, field
from typing import Any, Dict, Optional

from loom import Agent, load_project_config

SYMBOL = "BTC"


@dataclass
class PlannerBuffer:
    """Buffer for collecting partial analyses."""

    timeout_sec: float = 3.0
    entries: Dict[str, Dict[str, Any]] = field(default_factory=dict)

    def update(self, symbol: str, topic: str, payload: Dict[str, Any]) -> None:
        """Update buffer with new analysis."""
        now = time.time()
        if symbol not in self.entries:
            self.entries[symbol] = {"first_ts": now, "partials": {}}
        self.entries[symbol]["partials"][topic] = payload

    def ready(self, symbol: str) -> bool:
        """Check if all analyses received or timeout expired."""
        entry = self.entries.get(symbol)
        if not entry:
            return False

        partials = entry["partials"]
        has_all = len(partials) >= 3  # trend + risk + sentiment

        if has_all:
            return True

        # Check timeout
        elapsed = time.time() - entry["first_ts"]
        return elapsed >= self.timeout_sec

    def take(self, symbol: str) -> Optional[Dict[str, Any]]:
        """Take and clear partials for symbol."""
        return self.entries.pop(symbol, {}).get("partials")

    def expired_symbols(self) -> list[str]:
        """Get symbols with expired timeout."""
        now = time.time()
        expired = []
        for symbol, entry in self.entries.items():
            if (now - entry["first_ts"]) >= self.timeout_sec:
                expired.append(symbol)
        return expired


planner_buffer = PlannerBuffer(timeout_sec=3.0)


def make_plan(partials: Dict[str, Any]) -> Dict[str, Any]:
    """Generate trading plan from partial analyses.

    TODO: Integrate with DeepSeek LLM via ctx.tool("llm.generate", ...)
    """
    trend_data = partials.get("analysis.trend", {})
    risk_data = partials.get("analysis.risk", {})
    sentiment_data = partials.get("analysis.sentiment", {})

    # Simple rule-based logic (TODO: Replace with LLM reasoning)
    trend = trend_data.get("trend", "unknown")
    risk_score = risk_data.get("risk_score", 0.5)
    sentiment = sentiment_data.get("sentiment", "neutral")

    # Decision logic
    if trend == "up" and risk_score < 0.5 and sentiment in ["bullish", "neutral"]:
        action = "BUY"
        confidence = 0.75
    elif trend == "down" or risk_score > 0.7:
        action = "HOLD"
        confidence = 0.60
    else:
        action = "HOLD"
        confidence = 0.50

    return {
        "symbol": SYMBOL,
        "action": action,
        "confidence": confidence,
        "reasoning": f"Trend: {trend}, Risk: {risk_score:.2f}, Sentiment: {sentiment}",
        "sources": {
            "trend": trend_data,
            "risk": risk_data,
            "sentiment": sentiment_data,
        },
        "complete": len(partials) >= 3,
        "timestamp_ms": int(time.time() * 1000),
    }


async def planner_handler(ctx, topic: str, event) -> None:
    """Collect partial analyses and emit plan when ready."""
    data = json.loads(event.payload.decode("utf-8"))
    symbol = data.get("symbol", SYMBOL)

    planner_buffer.update(symbol, topic, data)

    # Check if ready to plan
    if planner_buffer.ready(symbol):
        partials = planner_buffer.take(symbol) or {}
        plan = make_plan(partials)

        status = "complete" if plan["complete"] else "timeout"
        print(f"[planner] Plan ready ({status}): {plan['action']} (conf: {plan['confidence']:.2f})")
        print(f"[planner]   Reasoning: {plan['reasoning']}")

        await ctx.emit(
            "plan.ready",
            type="plan.ready",
            payload=json.dumps(plan).encode("utf-8"),
        )


async def planner_timeout_loop(ctx) -> None:
    """Check for expired symbols periodically."""
    while True:
        await asyncio.sleep(0.5)

        for symbol in planner_buffer.expired_symbols():
            partials = planner_buffer.take(symbol) or {}
            if partials:
                plan = make_plan(partials)
                print(f"[planner] Plan ready (timeout): {plan['action']} (conf: {plan['confidence']:.2f})")
                print(f"[planner]   Partial data: {list(partials.keys())}")

                await ctx.emit(
                    "plan.ready",
                    type="plan.ready",
                    payload=json.dumps(plan).encode("utf-8"),
                )


async def main():
    config = load_project_config()
    agent_config = config.agents.get("planner-agent", {})

    topics = agent_config.get("topics", ["analysis.trend", "analysis.risk", "analysis.sentiment"])
    timeout_ms = agent_config.get("timeout_ms", 3000)

    # Update buffer timeout
    planner_buffer.timeout_sec = timeout_ms / 1000

    agent = Agent(
        agent_id="planner-agent",
        topics=topics,
        on_event=planner_handler,
    )

    print(f"[planner] Planner Agent starting")
    print(f"[planner] Subscribed to: {topics}")
    print(f"[planner] Timeout: {timeout_ms}ms")

    await agent.start()

    # Start timeout checker
    asyncio.create_task(planner_timeout_loop(agent._ctx))

    # Keep running
    try:
        await asyncio.Event().wait()
    except KeyboardInterrupt:
        print("[planner] Shutting down...")
        await agent.stop()


if __name__ == "__main__":
    asyncio.run(main())
