"""Sentiment Agent - Market Sentiment Analysis

Analyzes market sentiment using news/social data.
Can integrate with LLM and MCP tools for real sentiment analysis.
"""

import asyncio
import json
import random

from loom import Agent, load_project_config


async def sentiment_handler(ctx, topic: str, event) -> None:
    """Analyze sentiment from price update."""
    data = json.loads(event.payload.decode("utf-8"))
    price = data["price"]
    symbol = data["symbol"]

    # Simulate sentiment analysis (TODO: Use LLM + web search MCP tool)
    await asyncio.sleep(random.uniform(0.2, 0.5))

    sentiment = random.choice(["bullish", "neutral", "bearish"])
    score = random.uniform(-1.0, 1.0)

    result = {
        "symbol": symbol,
        "price": price,
        "sentiment": sentiment,
        "score": score,
        "sources": ["twitter", "news", "reddit"],  # Placeholder
        "timestamp_ms": data["timestamp_ms"],
    }

    print(f"[sentiment] {symbol} sentiment: {sentiment} (score: {score:.2f})")

    await ctx.emit(
        "analysis.sentiment",
        type="analysis.sentiment",
        payload=json.dumps(result).encode("utf-8"),
    )


async def main():
    config = load_project_config()
    agent_config = config.agents.get("sentiment-agent", {})
    topics = agent_config.get("topics", ["market.price.BTC"])

    agent = Agent(
        agent_id="sentiment-agent",
        topics=topics,
        on_event=sentiment_handler,
    )

    print(f"[sentiment] Sentiment Agent starting, subscribed to: {topics}")
    agent.run()


if __name__ == "__main__":
    asyncio.run(main())
