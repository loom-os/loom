"""
Market Analyst Crew (async fan-out / aggregation demo)

This example shows a truly asynchronous multi-agent workflow:

- data_agent: emits price updates every second on `price.update`
- trend_agent / risk_agent / sentiment_agent: subscribe to `price.update`
  and emit partial analyses on `partial.trend`, `partial.risk`, `partial.sentiment`
- planner_agent: aggregates partial analyses and emits `plan.ready`
  once it has enough information or a timeout is reached

Requirements:

- Loom bridge server running locally:

  cargo run -p loom-bridge --bin loom-bridge-server

Run:

  python -m loom.examples.market_analyst_async
"""

from __future__ import annotations

import asyncio
import json
import random
import time
from dataclasses import dataclass, field
from typing import Any, Dict

from loom import Agent

SYMBOL = "BTC"


async def data_loop(ctx, *, interval_sec: float = 1.0) -> None:
    """Continuously emit price updates."""
    while True:
        price = random.uniform(100.0, 200.0)
        payload = {
            "symbol": SYMBOL,
            "price": price,
            "timestamp_ms": int(time.time() * 1000),
        }
        print(f"[data] price.update {payload}")
        await ctx.emit(
            "price.update",
            type="price.update",
            payload=json.dumps(payload).encode("utf-8"),
        )
        await asyncio.sleep(interval_sec)


async def trend_handler(ctx, topic: str, event) -> None:
    """Simple trend analysis based on price parity."""
    data = json.loads(event.payload.decode("utf-8"))
    price = data["price"]
    trend = "up" if int(price) % 2 == 0 else "down"
    out = {
        "symbol": data["symbol"],
        "price": price,
        "trend": trend,
        "ts": data["timestamp_ms"],
    }
    print(f"[trend] partial.trend {out}")
    await ctx.emit(
        "partial.trend",
        type="partial.trend",
        payload=json.dumps(out).encode("utf-8"),
    )


async def risk_handler(ctx, topic: str, event) -> None:
    """Fake risk analysis, emits a risk score in [0,1]."""
    data = json.loads(event.payload.decode("utf-8"))
    # Simulate heavier computation
    await asyncio.sleep(random.uniform(0.1, 0.4))
    risk_score = random.uniform(0.0, 1.0)
    out = {
        "symbol": data["symbol"],
        "price": data["price"],
        "risk": risk_score,
        "ts": data["timestamp_ms"],
    }
    print(f"[risk] partial.risk {out}")
    await ctx.emit(
        "partial.risk",
        type="partial.risk",
        payload=json.dumps(out).encode("utf-8"),
    )


async def sentiment_handler(ctx, topic: str, event) -> None:
    """Fake sentiment analysis."""
    data = json.loads(event.payload.decode("utf-8"))
    await asyncio.sleep(random.uniform(0.2, 0.6))
    sentiment = random.choice(["positive", "neutral", "negative"])
    out = {
        "symbol": data["symbol"],
        "price": data["price"],
        "sentiment": sentiment,
        "ts": data["timestamp_ms"],
    }
    print(f"[sentiment] partial.sentiment {out}")
    await ctx.emit(
        "partial.sentiment",
        type="partial.sentiment",
        payload=json.dumps(out).encode("utf-8"),
    )


@dataclass
class PlannerBuffer:
    timeout_sec: float = 3.0
    # symbol -> {"first_ts": float, "partials": dict[topic, dict]}
    entries: Dict[str, Dict[str, Any]] = field(default_factory=dict)

    def update(self, symbol: str, topic: str, payload: Dict[str, Any]) -> None:
        now = time.time()
        entry = self.entries.setdefault(symbol, {"first_ts": now, "partials": {}})
        if not entry["partials"]:
            entry["first_ts"] = now
        entry["partials"][topic] = payload

    def _expired(self, symbol: str) -> bool:
        entry = self.entries.get(symbol)
        if not entry:
            return False
        return time.time() - entry["first_ts"] >= self.timeout_sec

    def ready(self, symbol: str) -> bool:
        entry = self.entries.get(symbol)
        if not entry:
            return False
        partials = entry["partials"]
        return (
            "partial.trend" in partials
            and "partial.risk" in partials
            and "partial.sentiment" in partials
        )

    def take(self, symbol: str) -> Dict[str, Any] | None:
        entry = self.entries.pop(symbol, None)
        if not entry:
            return None
        return entry["partials"]

    def expired_symbols(self) -> list[str]:
        return [s for s in list(self.entries.keys()) if self._expired(s)]


planner_buffer = PlannerBuffer()


def make_plan(partials: Dict[str, Dict[str, Any]]) -> Dict[str, Any]:
    """Very simple decision logic that could later be replaced by an LLM."""
    trend = partials.get("partial.trend", {}).get("trend")
    risk = partials.get("partial.risk", {}).get("risk", 1.0)
    sentiment = partials.get("partial.sentiment", {}).get("sentiment", "neutral")

    action = "HOLD"
    reason = []

    if trend == "up":
        reason.append("trend is up")
    else:
        reason.append("trend is not strongly up")

    if risk < 0.5:
        reason.append("risk is acceptable")
    else:
        reason.append("risk is high")

    if sentiment == "positive":
        reason.append("sentiment is positive")
    elif sentiment == "negative":
        reason.append("sentiment is negative")

    if trend == "up" and risk < 0.5 and sentiment != "negative":
        action = "BUY"
    elif trend == "down" and risk > 0.7:
        action = "SELL"

    return {
        "symbol": SYMBOL,
        "action": action,
        "reason": "; ".join(reason),
        "partials": partials,
    }


async def planner_handler(ctx, topic: str, event) -> None:
    """Collect partial analyses and decide when to emit a plan."""
    data = json.loads(event.payload.decode("utf-8"))
    symbol = data.get("symbol", SYMBOL)
    planner_buffer.update(symbol, topic, data)

    # If we already have all three partials, we can plan immediately
    if planner_buffer.ready(symbol):
        partials = planner_buffer.take(symbol) or {}
        plan = make_plan(partials)
        print(f"[planner] plan.ready (complete) {plan}")
        await ctx.emit(
            "plan.ready",
            type="plan.ready",
            payload=json.dumps(plan).encode("utf-8"),
        )


async def planner_timeout_loop(ctx) -> None:
    """Background loop to handle timeouts (emit plan with partial information)."""
    while True:
        await asyncio.sleep(0.5)
        for symbol in planner_buffer.expired_symbols():
            partials = planner_buffer.take(symbol)
            if not partials:
                continue
            plan = make_plan(partials)
            print(f"[planner] plan.ready (timeout) {plan}")
            await ctx.emit(
                "plan.ready",
                type="plan.ready",
                payload=json.dumps(plan).encode("utf-8"),
            )


async def main() -> None:
    # Data source agent (no subscriptions, only emits)
    data_agent = Agent("data-agent", topics=[])

    # Analysis agents subscribe to price updates
    trend_agent = Agent("trend-agent", topics=["price.update"], on_event=trend_handler)
    risk_agent = Agent("risk-agent", topics=["price.update"], on_event=risk_handler)
    sentiment_agent = Agent("sentiment-agent", topics=["price.update"], on_event=sentiment_handler)

    # Planner subscribes to partial analyses
    planner_agent = Agent(
        "planner-agent",
        topics=["partial.trend", "partial.risk", "partial.sentiment"],
        on_event=planner_handler,
    )

    # Start all agents and background loops
    await asyncio.gather(
        data_agent.start(),
        trend_agent.start(),
        risk_agent.start(),
        sentiment_agent.start(),
        planner_agent.start(),
    )

    # Seed background tasks after agents are connected
    asyncio.create_task(data_loop(data_agent._ctx))
    asyncio.create_task(planner_timeout_loop(planner_agent._ctx))

    # Let the system run for a while, then shut down
    try:
        await asyncio.sleep(20.0)
    finally:
        await asyncio.gather(
            data_agent.stop(),
            trend_agent.stop(),
            risk_agent.stop(),
            sentiment_agent.stop(),
            planner_agent.stop(),
        )


if __name__ == "__main__":
    asyncio.run(main())
