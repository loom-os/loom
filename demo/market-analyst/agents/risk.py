"""Risk Agent - Risk Analysis

Calculates risk metrics and emits risk assessments.
"""

import asyncio
import json
import random

from loom import Agent, load_project_config


async def risk_handler(ctx, topic: str, event) -> None:
    """Calculate risk from price update."""
    data = json.loads(event.payload.decode("utf-8"))
    price = data["price"]
    symbol = data["symbol"]

    # Simulate risk calculation (TODO: Real VaR, volatility, etc.)
    await asyncio.sleep(random.uniform(0.1, 0.3))  # Simulate computation

    risk_score = random.uniform(0.2, 0.8)
    volatility = random.uniform(0.15, 0.45)

    result = {
        "symbol": symbol,
        "price": price,
        "risk_score": risk_score,
        "volatility": volatility,
        "timestamp_ms": data["timestamp_ms"],
    }

    print(f"[risk] {symbol} risk: {risk_score:.2f}, vol: {volatility:.2f}")

    await ctx.emit(
        "analysis.risk",
        type="analysis.risk",
        payload=json.dumps(result).encode("utf-8"),
    )


async def main():
    config = load_project_config()
    agent_config = config.agents.get("risk-agent", {})
    topics = agent_config.get("topics", ["market.price.BTC"])

    agent = Agent(
        agent_id="risk-agent",
        topics=topics,
        on_event=risk_handler,
    )

    print(f"[risk] Risk Agent starting, subscribed to: {topics}")
    agent.run()


if __name__ == "__main__":
    asyncio.run(main())
