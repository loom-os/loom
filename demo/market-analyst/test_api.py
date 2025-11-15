#!/usr/bin/env python3
"""Test data sources."""

import asyncio
import sys
from pathlib import Path
from agents.data import CryptoCompareClient, CoinGeckoClient
sys.path.insert(0, str(Path(__file__).parent.resolve()))


async def test_cryptocompare():
    print("Testing CryptoCompare API...")
    client = CryptoCompareClient(use_sync=True)
    result = await client.get_ticker('BTC')
    if result:
        print('✓ CryptoCompare API working!')
        print(f"  Price: ${result.get('lastPrice')}")
        print(f"  24h Change: {result.get('priceChangePercent')}%")
        print(f"  Volume: ${result.get('volume')}")
        return True
    else:
        print('✗ CryptoCompare API failed')
        return False


async def test_coingecko():
    print("\nTesting CoinGecko API...")
    client = CoinGeckoClient(use_sync=True)
    result = await client.get_ticker('BTC')
    if result:
        print('✓ CoinGecko API working!')
        print(f"  Price: ${result.get('lastPrice')}")
        print(f"  24h Change: {result.get('priceChangePercent')}%")
        return True
    else:
        print('✗ CoinGecko API failed')
        return False


async def main():
    print("=" * 60)
    print("Data Source API Test")
    print("=" * 60 + "\n")

    success_count = 0

    if await test_cryptocompare():
        success_count += 1

    if await test_coingecko():
        success_count += 1

    print("\n" + "=" * 60)
    print(f"Result: {success_count}/2 APIs working")
    print("=" * 60)


if __name__ == "__main__":
    asyncio.run(main())
