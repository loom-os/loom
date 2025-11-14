"""Data Agent - Market Price Feed

Emits real-time market price updates for configured symbols.
In production, this would connect to an exchange WebSocket/REST API.
"""

import asyncio
import json
import random
import time

from loom import Agent, load_project_config

SYMBOL = "BTC"


async def data_loop(ctx, interval_sec: float = 1.0) -> None:
    """Continuously emit price updates."""
    print(f"[data] Starting price feed for {SYMBOL} (interval: {interval_sec}s)")

    while True:
        # TODO: Replace with real exchange API (Binance, Coinbase, etc.)
        price = random.uniform(40000.0, 60000.0)  # Simulated BTC price

        payload = {
            "symbol": SYMBOL,
            "price": price,
            "timestamp_ms": int(time.time() * 1000),
            "volume": random.uniform(100, 1000),
        }

        print(f"[data] Emitting price update: {SYMBOL} ${price:.2f}")

        await ctx.emit(
            f"market.price.{SYMBOL}",
            type="price.update",
            payload=json.dumps(payload).encode("utf-8"),
        )

        await asyncio.sleep(interval_sec)


async def main():
    """Main entry point."""
    # Load configuration
    config = load_project_config()
    agent_config = config.agents.get("data-agent", {})

    # Get settings
    topics = agent_config.get("topics", [f"market.price.{SYMBOL}"])
    interval = agent_config.get("refresh_interval_sec", 1.0)

    # Create agent (data agent only emits, no subscriptions needed)
    agent = Agent(
        agent_id="data-agent",
        topics=[],  # No subscriptions, only emits
        on_event=None,
    )

    print(f"[data] Data Agent starting...")
    print(f"[data] Will emit to: {topics}")
    print(f"[data] Interval: {interval}s")

    await agent.start()

    # Start data loop
    asyncio.create_task(data_loop(agent._ctx, interval_sec=interval))

    # Keep running
    try:
        await asyncio.Event().wait()
    except KeyboardInterrupt:
        print("[data] Shutting down...")
        await agent.stop()


if __name__ == "__main__":
    asyncio.run(main())
