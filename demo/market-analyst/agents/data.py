"""Data Agent - Market Price Feed

Emits real-time market price updates from Binance exchange.
Falls back to simulated data if API unavailable.
"""

import asyncio
import json
import random
import time
from typing import Optional

try:
    import aiohttp
    AIOHTTP_AVAILABLE = True
except ImportError:
    AIOHTTP_AVAILABLE = False
    print("[data] Warning: aiohttp not installed, using simulated data")

from loom import Agent, load_project_config

SYMBOL = "BTC"


class BinanceClient:
    """Simple Binance REST API client for public market data."""

    BASE_URL = "https://api.binance.com/api/v3"

    def __init__(self):
        self.session: Optional[aiohttp.ClientSession] = None

    async def __aenter__(self):
        if AIOHTTP_AVAILABLE:
            self.session = aiohttp.ClientSession()
        return self

    async def __aexit__(self, *args):
        if self.session:
            await self.session.close()

    async def get_ticker(self, symbol: str) -> Optional[dict]:
        """Get 24h ticker price data for a symbol.

        Args:
            symbol: Trading pair symbol (e.g., "BTCUSDT")

        Returns:
            Dict with price data or None if failed
        """
        if not self.session:
            return None

        try:
            url = f"{self.BASE_URL}/ticker/24hr"
            params = {"symbol": symbol}

            async with self.session.get(url, params=params, timeout=aiohttp.ClientTimeout(total=5)) as resp:
                if resp.status == 200:
                    return await resp.json()
                else:
                    print(f"[data] Binance API error: {resp.status}")
                    return None
        except Exception as e:
            print(f"[data] Failed to fetch from Binance: {e}")
            return None


def simulate_price(base_price: float = 50000.0) -> dict:
    """Generate simulated market data."""
    return {
        "lastPrice": str(random.uniform(base_price * 0.95, base_price * 1.05)),
        "volume": str(random.uniform(100, 1000)),
        "priceChangePercent": str(random.uniform(-5.0, 5.0)),
        "highPrice": str(base_price * 1.05),
        "lowPrice": str(base_price * 0.95),
    }


async def data_loop(ctx, interval_sec: float = 1.0, exchange_symbol: str = "BTCUSDT", use_real_data: bool = True) -> None:
    """Continuously emit price updates.

    Args:
        ctx: Agent context
        interval_sec: Update interval in seconds
        exchange_symbol: Exchange trading pair (e.g., "BTCUSDT" for Binance)
        use_real_data: Whether to use real exchange API or simulated data
    """
    print(f"[data] Starting price feed for {SYMBOL}")
    print(f"[data] Mode: {'REAL' if use_real_data and AIOHTTP_AVAILABLE else 'SIMULATED'}")
    print(f"[data] Exchange symbol: {exchange_symbol}")
    print(f"[data] Interval: {interval_sec}s")

    binance = None
    if use_real_data and AIOHTTP_AVAILABLE:
        binance = await BinanceClient().__aenter__()

    last_price = 50000.0  # For simulated data continuity

    try:
        while True:
            ticker_data = None

            # Try to get real data first
            if binance:
                ticker_data = await binance.get_ticker(exchange_symbol)

            # Fall back to simulation if needed
            if not ticker_data:
                ticker_data = simulate_price(last_price)
                source = "simulated"
            else:
                source = "binance"
                last_price = float(ticker_data.get("lastPrice", last_price))

            # Parse ticker data
            price = float(ticker_data.get("lastPrice", last_price))
            volume = float(ticker_data.get("volume", 0))
            price_change_pct = float(ticker_data.get("priceChangePercent", 0))
            high_price = float(ticker_data.get("highPrice", price))
            low_price = float(ticker_data.get("lowPrice", price))

            payload = {
                "symbol": SYMBOL,
                "price": price,
                "volume": volume,
                "price_change_percent": price_change_pct,
                "high_24h": high_price,
                "low_24h": low_price,
                "timestamp_ms": int(time.time() * 1000),
                "source": source,
            }

            print(f"[data] {source.upper()} | {SYMBOL} ${price:,.2f} | 24h: {price_change_pct:+.2f}%")

            await ctx.emit(
                f"market.price.{SYMBOL}",
                type="price.update",
                payload=json.dumps(payload).encode("utf-8"),
            )

            await asyncio.sleep(interval_sec)

    finally:
        if binance:
            await binance.__aexit__(None, None, None)


async def main():
    """Main entry point."""
    try:
        # Load configuration
        config = load_project_config()
        agent_config = config.agents.get("data-agent", {})

        # Get settings
        topics = agent_config.get("topics", [f"market.price.{SYMBOL}"])
        interval = agent_config.get("refresh_interval_sec", 1.0)
        exchange = agent_config.get("exchange", "binance")
        symbols = agent_config.get("symbols", ["BTCUSDT"])
        exchange_symbol = symbols[0] if symbols else "BTCUSDT"

        # Create agent (data agent only emits, no subscriptions needed)
        agent = Agent(
            agent_id="data-agent",
            topics=[],  # No subscriptions, only emits
            on_event=None,
        )

        print(f"[data] Data Agent starting...")
        print(f"[data] Will emit to: {topics}")
        print(f"[data] Exchange: {exchange}")
        print(f"[data] Symbols: {symbols}")

        await agent.start()

        # Start data loop
        use_real_data = AIOHTTP_AVAILABLE and exchange.lower() == "binance"
        asyncio.create_task(
            data_loop(
                agent._ctx,
                interval_sec=interval,
                exchange_symbol=exchange_symbol,
                use_real_data=use_real_data,
            )
        )

        # Keep running
        try:
            await asyncio.Event().wait()
        except KeyboardInterrupt:
            print("[data] Shutting down...")
            await agent.stop()

    except Exception as e:
        print(f"[data] FATAL ERROR: {type(e).__name__}: {e}")
        import traceback
        traceback.print_exc()
        raise


if __name__ == "__main__":
    asyncio.run(main())
