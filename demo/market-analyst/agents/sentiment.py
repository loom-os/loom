"""Sentiment Agent - Market Sentiment Analysis

Analyzes market sentiment using news/social data.
Can integrate with LLM and MCP tools for real sentiment analysis.
"""

import asyncio
import json
import random

from opentelemetry import trace

from loom import Agent, load_project_config

tracer = trace.get_tracer(__name__)


async def sentiment_handler(ctx, topic: str, event) -> None:
    """Analyze sentiment from price update."""
    with tracer.start_as_current_span(
        "sentiment.analyze",
        attributes={"sentiment.topic": topic},
    ) as span:
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

        # Record sentiment metrics
        span.set_attribute("sentiment.symbol", symbol)
        span.set_attribute("sentiment.price", price)
        span.set_attribute("sentiment.result", sentiment)
        span.set_attribute("sentiment.score", score)

        print(f"[sentiment] {symbol} sentiment: {sentiment} (score: {score:.2f})")

        await ctx.emit(
            "analysis.sentiment",
            type="analysis.sentiment",
            payload=json.dumps(result).encode("utf-8"),
        )

        span.set_status(trace.Status(trace.StatusCode.OK))


async def main():
    config = load_project_config()
    agent_config = config.agents.get("sentiment-agent", {})
    topics = agent_config.get("topics", ["market.price.*"])

    agent = Agent(
        agent_id="sentiment-agent",
        topics=topics,
        on_event=sentiment_handler,
    )

    print(f"[sentiment] Sentiment Agent starting")
    print(f"[sentiment] Subscribed to: {topics}")

    await agent.start()

    # Keep running
    try:
        await asyncio.Event().wait()
    except KeyboardInterrupt:
        print("[sentiment] Shutting down...")
        await agent.stop()


if __name__ == "__main__":
    asyncio.run(main())
