"""Trend Agent - Technical Analysis

Analyzes price trends and emits technical indicators.
"""

import asyncio
import json

from loom import Agent, load_project_config


async def trend_handler(ctx, topic: str, event) -> None:
    """Analyze trend from price update."""
    data = json.loads(event.payload.decode("utf-8"))
    price = data["price"]
    symbol = data["symbol"]

    # Simple trend analysis (TODO: Use real indicators like SMA/EMA/RSI)
    trend = "up" if int(price) % 2 == 0 else "down"
    confidence = 0.6 + (abs(hash(str(price))) % 40) / 100  # Simulated confidence

    result = {
        "symbol": symbol,
        "price": price,
        "trend": trend,
        "confidence": confidence,
        "timestamp_ms": data["timestamp_ms"],
    }

    print(f"[trend] {symbol} trend: {trend} (conf: {confidence:.2f})")

    await ctx.emit(
        "analysis.trend",
        type="analysis.trend",
        payload=json.dumps(result).encode("utf-8"),
    )


async def main():
    config = load_project_config()
    agent_config = config.agents.get("trend-agent", {})
    topics = agent_config.get("topics", ["market.price.*"])

    agent = Agent(
        agent_id="trend-agent",
        topics=topics,
        on_event=trend_handler,
    )

    print(f"[trend] Trend Agent starting")
    print(f"[trend] Subscribed to: {topics}")

    await agent.start()

    # Keep running
    try:
        await asyncio.Event().wait()
    except KeyboardInterrupt:
        print("[trend] Shutting down...")
        await agent.stop()


if __name__ == "__main__":
    asyncio.run(main())
