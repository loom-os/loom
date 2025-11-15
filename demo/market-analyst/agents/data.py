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

try:
    import requests
    REQUESTS_AVAILABLE = True
except ImportError:
    REQUESTS_AVAILABLE = False

from loom import Agent, load_project_config

# Default cryptocurrency symbol to track
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


class CoinGeckoClient:
    """CoinGecko API client for cryptocurrency prices (no API key required)."""

    BASE_URL = "https://api.coingecko.com/api/v3"

    # Map common symbols to CoinGecko IDs
    SYMBOL_MAP = {
        "BTC": "bitcoin",
        "ETH": "ethereum",
        "USDT": "tether",
        "BNB": "binancecoin",
    }

    def __init__(self, use_sync: bool = False):
        self.session: Optional[aiohttp.ClientSession] = None
        self.use_sync = use_sync

    async def __aenter__(self):
        if not self.use_sync and AIOHTTP_AVAILABLE:
            self.session = aiohttp.ClientSession()
        return self

    async def __aexit__(self, *args):
        if self.session:
            await self.session.close()

    async def get_ticker(self, symbol: str) -> Optional[dict]:
        """Get price data for a cryptocurrency.

        Args:
            symbol: Symbol (e.g., "BTC", "ETH")

        Returns:
            Dict with price data or None if failed
        """
        coin_id = self.SYMBOL_MAP.get(symbol)
        if not coin_id:
            return None

        # Try sync method with requests if async fails or use_sync is True
        if self.use_sync and REQUESTS_AVAILABLE:
            return await asyncio.to_thread(self._get_ticker_sync, coin_id)

        # Async method with aiohttp
        if not self.session:
            return None

        try:
            url = f"{self.BASE_URL}/simple/price"
            params = {
                "ids": coin_id,
                "vs_currencies": "usd",
                "include_24hr_vol": "true",
                "include_24hr_change": "true",
                "include_last_updated_at": "true",
            }

            async with self.session.get(url, params=params, timeout=aiohttp.ClientTimeout(total=5)) as resp:
                if resp.status == 200:
                    data = await resp.json()
                    if coin_id in data:
                        return self._normalize_coingecko_data(data[coin_id])
                    return None
                else:
                    print(f"[data] CoinGecko API error: {resp.status}")
                    return None
        except Exception as e:
            print(f"[data] Failed to fetch from CoinGecko (async): {e}")
            # Try sync fallback if available
            if REQUESTS_AVAILABLE:
                print("[data] Trying sync fallback with requests...")
                return await asyncio.to_thread(self._get_ticker_sync, coin_id)
            return None

    def _get_ticker_sync(self, coin_id: str) -> Optional[dict]:
        """Synchronous fallback using requests library."""
        try:
            url = f"{self.BASE_URL}/simple/price"
            params = {
                "ids": coin_id,
                "vs_currencies": "usd",
                "include_24hr_vol": "true",
                "include_24hr_change": "true",
            }

            response = requests.get(url, params=params, timeout=10)
            if response.status_code == 200:
                data = response.json()
                if coin_id in data:
                    return self._normalize_coingecko_data(data[coin_id])
            else:
                print(f"[data] CoinGecko API error (sync): {response.status_code}")
            return None
        except Exception as e:
            print(f"[data] Failed to fetch from CoinGecko (sync): {e}")
            return None

    def _normalize_coingecko_data(self, coin_data: dict) -> dict:
        """Normalize CoinGecko data to our format."""
        price = coin_data.get("usd", 0)
        return {
            "lastPrice": str(price),
            "volume": str(coin_data.get("usd_24h_vol", 0)),
            "priceChangePercent": str(coin_data.get("usd_24h_change", 0)),
            "highPrice": str(price * 1.02),  # Approximation
            "lowPrice": str(price * 0.98),   # Approximation
        }


class CryptoCompareClient:
    """CryptoCompare API client (no API key required for basic use)."""

    BASE_URL = "https://min-api.cryptocompare.com/data"

    def __init__(self, use_sync: bool = False):
        self.session: Optional[aiohttp.ClientSession] = None
        self.use_sync = use_sync

    async def __aenter__(self):
        if not self.use_sync and AIOHTTP_AVAILABLE:
            self.session = aiohttp.ClientSession()
        return self

    async def __aexit__(self, *args):
        if self.session:
            await self.session.close()

    async def get_ticker(self, symbol: str) -> Optional[dict]:
        """Get price data for a cryptocurrency.

        Args:
            symbol: Symbol (e.g., "BTC", "ETH")

        Returns:
            Dict with price data or None if failed
        """
        # Try sync method first (more reliable)
        if REQUESTS_AVAILABLE:
            return await asyncio.to_thread(self._get_ticker_sync, symbol)

        # Async method
        if not self.session:
            return None

        try:
            # Get current price
            url = f"{self.BASE_URL}/pricemultifull"
            params = {
                "fsyms": symbol,
                "tsyms": "USD"
            }

            async with self.session.get(url, params=params, timeout=aiohttp.ClientTimeout(total=5)) as resp:
                if resp.status == 200:
                    data = await resp.json()
                    if "RAW" in data and symbol in data["RAW"] and "USD" in data["RAW"][symbol]:
                        return self._normalize_cryptocompare_data(data["RAW"][symbol]["USD"])
                    return None
                else:
                    print(f"[data] CryptoCompare API error: {resp.status}")
                    return None
        except Exception as e:
            print(f"[data] Failed to fetch from CryptoCompare (async): {e}")
            return None

    def _get_ticker_sync(self, symbol: str) -> Optional[dict]:
        """Synchronous fallback using requests library."""
        try:
            url = f"{self.BASE_URL}/pricemultifull"
            params = {
                "fsyms": symbol,
                "tsyms": "USD"
            }

            response = requests.get(url, params=params, timeout=10)
            if response.status_code == 200:
                data = response.json()
                if "RAW" in data and symbol in data["RAW"] and "USD" in data["RAW"][symbol]:
                    return self._normalize_cryptocompare_data(data["RAW"][symbol]["USD"])
            else:
                print(f"[data] CryptoCompare API error (sync): {response.status_code}")
            return None
        except Exception as e:
            print(f"[data] Failed to fetch from CryptoCompare (sync): {e}")
            return None

    def _normalize_cryptocompare_data(self, usd_data: dict) -> dict:
        """Normalize CryptoCompare data to our format."""
        return {
            "lastPrice": str(usd_data.get("PRICE", 0)),
            "volume": str(usd_data.get("VOLUME24HOUR", 0)),
            "priceChangePercent": str(usd_data.get("CHANGEPCT24HOUR", 0)),
            "highPrice": str(usd_data.get("HIGH24HOUR", 0)),
            "lowPrice": str(usd_data.get("LOW24HOUR", 0)),
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


async def data_loop(ctx, interval_sec: float = 1.0, exchange_symbol: str = "BTCUSDT", use_real_data: bool = True, data_source: str = "binance") -> None:
    """Continuously emit price updates.

    Args:
        ctx: Agent context
        interval_sec: Update interval in seconds
        exchange_symbol: Exchange trading pair (e.g., "BTCUSDT" for Binance) or symbol (e.g., "BTC" for CoinGecko)
        use_real_data: Whether to use real exchange API or simulated data
        data_source: Data source to use ("binance" or "coingecko")
    """
    print(f"[data] Starting price feed for {SYMBOL}")
    print(f"[data] Mode: {'REAL' if use_real_data and AIOHTTP_AVAILABLE else 'SIMULATED'}")
    print(f"[data] Data source: {data_source}")
    print(f"[data] Symbol: {exchange_symbol}")
    print(f"[data] Interval: {interval_sec}s")

    client = None
    if use_real_data:
        if data_source == "coingecko":
            print("[data] Initializing CoinGecko client...")
            client = await CoinGeckoClient(use_sync=REQUESTS_AVAILABLE).__aenter__()
        elif data_source == "cryptocompare":
            print("[data] Initializing CryptoCompare client...")
            client = await CryptoCompareClient(use_sync=REQUESTS_AVAILABLE).__aenter__()
        else:  # binance
            print("[data] Initializing Binance client...")
            client = await BinanceClient().__aenter__()

    last_price = REALISTIC_PRICES.get(SYMBOL, 50000.0)  # Use realistic baseline

    try:
        while True:
            ticker_data = None
            source = "simulated"

            # Try to get real data first
            if client:
                ticker_data = await client.get_ticker(exchange_symbol)
                if ticker_data:
                    source = data_source

            # Fall back to realistic simulation if needed
            if not ticker_data:
                ticker_data = simulate_price(last_price, SYMBOL)
                source = "simulated-realistic"

            # Parse ticker data
            price = float(ticker_data.get("lastPrice", last_price))
            volume = float(ticker_data.get("volume", 0))
            price_change_pct = float(ticker_data.get("priceChangePercent", 0))
            high_price = float(ticker_data.get("highPrice", price))
            low_price = float(ticker_data.get("lowPrice", price))

            # Update last price for continuity
            last_price = price

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

            print(f"[data] {source.upper():20s} | {SYMBOL} ${price:,.2f} | 24h: {price_change_pct:+.2f}% | Vol: ${volume:,.0f}")

            await ctx.emit(
                f"market.price.{SYMBOL}",
                type="price.update",
                payload=json.dumps(payload).encode("utf-8"),
            )

            await asyncio.sleep(interval_sec)

    finally:
        if client:
            await client.__aexit__(None, None, None)


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
        # Determine data source and symbol based on config
        data_source = agent_config.get("data_source", "cryptocompare")  # Default to CryptoCompare (most reliable)
        use_real_data = AIOHTTP_AVAILABLE or REQUESTS_AVAILABLE

        # For CoinGecko, use simple symbol (BTC, ETH)
        # For Binance, use trading pair (BTCUSDT, ETHUSDT)
        # For CryptoCompare, use simple symbol (BTC, ETH)
        if data_source == "binance":
            symbol_for_api = exchange_symbol  # Use trading pair like "BTCUSDT"
        else:  # coingecko or cryptocompare
            symbol_for_api = SYMBOL  # Use simple symbol like "BTC"

        asyncio.create_task(
            data_loop(
                agent._ctx,
                interval_sec=interval,
                exchange_symbol=symbol_for_api,
                use_real_data=use_real_data,
                data_source=data_source,
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
