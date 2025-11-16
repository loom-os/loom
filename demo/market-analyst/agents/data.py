"""Data Agent - Market Price Feed

Emits real-time market price updates from OKX WebSocket.
Falls back to simulated data if connection fails.
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

from loom import Agent, load_project_config

# Default cryptocurrency symbol to track
SYMBOL = "BTC"


class OKXWebSocketClient:
    """OKX WebSocket client for real-time market data.

    Connects to OKX public WebSocket and subscribes to ticker updates.
    Provides real-time price, volume, and 24h statistics.
    """

    WS_URL = "wss://ws.okx.com:8443/ws/v5/public"

    def __init__(self):
        self.ws: Optional[aiohttp.ClientWebSocketResponse] = None
        self.session: Optional[aiohttp.ClientSession] = None
        self._connected = False
        self._last_ticker = {}

    async def connect(self):
        """Establish WebSocket connection."""
        if not AIOHTTP_AVAILABLE:
            print("[data-okx] aiohttp not available")
            return False

        try:
            self.session = aiohttp.ClientSession()
            self.ws = await self.session.ws_connect(
                self.WS_URL,
                timeout=aiohttp.ClientTimeout(total=30)
            )
            self._connected = True
            print("[data-okx] WebSocket connected")
            return True
        except Exception as e:
            print(f"[data-okx] Failed to connect: {e}")
            if self.session:
                await self.session.close()
            return False

    async def subscribe(self, inst_id: str):
        """Subscribe to ticker channel for a trading pair.

        Args:
            inst_id: Instrument ID (e.g., "BTC-USDT", "ETH-USDT")
        """
        if not self.ws or not self._connected:
            print("[data-okx] Not connected, cannot subscribe")
            return False

        try:
            subscribe_msg = {
                "op": "subscribe",
                "args": [
                    {
                        "channel": "tickers",
                        "instId": inst_id
                    }
                ]
            }

            await self.ws.send_json(subscribe_msg)
            print(f"[data-okx] Subscribed to {inst_id} ticker")
            return True
        except Exception as e:
            print(f"[data-okx] Failed to subscribe: {e}")
            return False

    async def receive_updates(self, callback):
        """Receive ticker updates and invoke callback.

        Args:
            callback: Async function to call with ticker data
        """
        if not self.ws or not self._connected:
            print("[data-okx] Not connected")
            return

        try:
            async for msg in self.ws:
                if msg.type == aiohttp.WSMsgType.TEXT:
                    data = json.loads(msg.data)

                    # Handle subscription confirmation
                    if data.get("event") == "subscribe":
                        print(f"[data-okx] Subscription confirmed: {data.get('arg', {}).get('instId')}")
                        continue

                    # Handle ticker data
                    if data.get("arg", {}).get("channel") == "tickers" and data.get("data"):
                        ticker = data["data"][0]
                        self._last_ticker = ticker
                        await callback(ticker)

                elif msg.type == aiohttp.WSMsgType.ERROR:
                    print(f"[data-okx] WebSocket error: {self.ws.exception()}")
                    break

                elif msg.type == aiohttp.WSMsgType.CLOSED:
                    print("[data-okx] WebSocket closed")
                    break

        except asyncio.CancelledError:
            print("[data-okx] Receive loop cancelled")
            raise
        except Exception as e:
            print(f"[data-okx] Error receiving updates: {e}")

    async def close(self):
        """Close WebSocket connection."""
        self._connected = False
        if self.ws:
            await self.ws.close()
        if self.session:
            await self.session.close()
        print("[data-okx] WebSocket closed")

    def normalize_ticker(self, ticker: dict, symbol: str) -> dict:
        """Normalize OKX ticker data to our format.

        Args:
            ticker: OKX ticker data
            symbol: Symbol (e.g., "BTC")

        Returns:
            Normalized ticker dict
        """
        return {
            "lastPrice": ticker.get("last", "0"),
            "volume": ticker.get("volCcy24h", "0"),  # 24h volume in quote currency
            "priceChangePercent": str(float(ticker.get("changePercent24h", "0")) * 100),  # Convert to percentage
            "highPrice": ticker.get("high24h", "0"),
            "lowPrice": ticker.get("low24h", "0"),
        }


# Realistic baseline prices (updated periodically for reasonable simulation)
REALISTIC_PRICES = {
    "BTC": 91500.0,   # Bitcoin realistic 2024 range
    "ETH": 3200.0,    # Ethereum realistic 2024 range
}


def simulate_price(base_price: float = 50000.0, symbol: str = "BTC") -> dict:
    """Generate simulated market data with realistic variance."""
    # Use realistic baseline if available
    if symbol in REALISTIC_PRICES:
        base_price = REALISTIC_PRICES[symbol]

    # Small variance for realistic simulation (±2%)
    variance = random.uniform(0.98, 1.02)
    current_price = base_price * variance

    return {
        "lastPrice": str(current_price),
        "volume": str(random.uniform(20000000, 30000000) if symbol == "BTC" else random.uniform(10000000, 15000000)),
        "priceChangePercent": str(random.uniform(-3.0, 3.0)),
        "highPrice": str(current_price * 1.02),
        "lowPrice": str(current_price * 0.98),
    }


async def data_loop_okx_websocket(ctx, inst_id: str = "BTC-USDT") -> None:
    """OKX WebSocket-based data loop (event-driven).

    Args:
        ctx: Agent context
        inst_id: OKX instrument ID (e.g., "BTC-USDT", "ETH-USDT")
    """
    okx_client = OKXWebSocketClient()

    async def handle_ticker(ticker: dict):
        """Handle incoming ticker update from OKX WebSocket."""
        try:
            # Normalize ticker data
            normalized = okx_client.normalize_ticker(ticker, SYMBOL)

            # Parse data
            price = float(normalized.get("lastPrice", 0))
            volume = float(normalized.get("volume", 0))
            price_change_pct = float(normalized.get("priceChangePercent", 0))
            high_price = float(normalized.get("highPrice", price))
            low_price = float(normalized.get("lowPrice", price))

            payload = {
                "symbol": SYMBOL,
                "price": price,
                "volume": volume,
                "price_change_percent": price_change_pct,
                "high_24h": high_price,
                "low_24h": low_price,
                "timestamp_ms": int(time.time() * 1000),
                "source": "okx-websocket",
            }

            print(f"[data] OKX-WEBSOCKET      | {SYMBOL} ${price:,.2f} | 24h: {price_change_pct:+.2f}% | Vol: ${volume:,.0f}")

            await ctx.emit(
                f"market.price.{SYMBOL}",
                type="price.update",
                payload=json.dumps(payload).encode("utf-8"),
            )

        except Exception as e:
            print(f"[data-okx] Error handling ticker: {e}")

    try:
        # Connect to OKX WebSocket
        if not await okx_client.connect():
            print("[data-okx] Failed to connect, falling back to simulation")
            # Fall back to simulated data
            last_price = REALISTIC_PRICES.get(SYMBOL, 50000.0)
            while True:
                ticker_data = simulate_price(last_price, SYMBOL)
                price = float(ticker_data.get("lastPrice", last_price))
                await handle_ticker({
                    "last": str(price),
                    "volCcy24h": ticker_data.get("volume", "0"),
                    "changePercent24h": str(float(ticker_data.get("priceChangePercent", "0")) / 100),
                    "high24h": ticker_data.get("highPrice", str(price)),
                    "low24h": ticker_data.get("lowPrice", str(price)),
                })
                last_price = price
                await asyncio.sleep(1.0)
            return

        # Subscribe to ticker
        if not await okx_client.subscribe(inst_id):
            print("[data-okx] Failed to subscribe")
            await okx_client.close()
            return

        # Receive updates (blocking)
        await okx_client.receive_updates(handle_ticker)

    finally:
        await okx_client.close()


async def main():
    """Main entry point."""
    try:
        # Load configuration
        config = load_project_config()
        agent_config = config.agents.get("data-agent", {})

        # Get settings
        symbols = agent_config.get("symbols", ["BTC-USDT"])
        inst_id = symbols[0] if symbols else "BTC-USDT"

        # Create agent (data agent only emits, no subscriptions needed)
        agent = Agent(
            agent_id="data-agent",
            topics=[],  # No subscriptions, only emits
            on_event=None,
        )

        print(f"[data] Data Agent starting...")
        print(f"[data] Exchange: OKX WebSocket")
        print(f"[data] Instrument: {inst_id}")
        print(f"[data] Will emit to: market.price.{SYMBOL}")

        await agent.start()

        # Start OKX WebSocket data loop
        asyncio.create_task(
            data_loop_okx_websocket(agent._ctx, inst_id=inst_id)
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
