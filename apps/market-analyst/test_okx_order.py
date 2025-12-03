#!/usr/bin/env python3
"""Test OKX order placement via WebSocket.

This script tests the complete order flow:
1. Connect to OKX WebSocket
2. Authenticate
3. Subscribe to private orders channel
4. Place a small market order
5. Wait for confirmation

Run: python test_okx_order.py
"""

import asyncio
import json
import hmac
import base64
import time
import os
from datetime import datetime, timezone
from dotenv import load_dotenv

try:
    import aiohttp
except ImportError:
    print("ERROR: aiohttp not installed. Run: pip install aiohttp")
    exit(1)

# Load environment variables
load_dotenv()

# Configuration
API_KEY = os.getenv("OKX_API_KEY")
SECRET_KEY = os.getenv("OKX_SECRET_KEY")
PASSPHRASE = os.getenv("OKX_PASSPHRASE")
USE_DEMO = os.getenv("OKX_USE_DEMO", "true").lower() == "true"

# Test order parameters
INST_ID = "BTC-USDT"  # Trading pair
SIDE = "buy"          # buy or sell
SIZE = "10"           # Order size in USDT (minimum is ~5 USDT for BTC-USDT)
TRADE_MODE = "cash"   # cash for spot

# WebSocket URL
if USE_DEMO:
    WS_URL = "wss://wspap.okx.com:8443/ws/v5/private?brokerId=9999"
else:
    WS_URL = "wss://ws.okx.com:8443/ws/v5/private"


def generate_signature(timestamp: str, method: str, request_path: str) -> str:
    """Generate OKX API signature."""
    message = timestamp + method + request_path
    mac = hmac.new(
        SECRET_KEY.encode('utf-8'),
        message.encode('utf-8'),
        digestmod='sha256'
    )
    return base64.b64encode(mac.digest()).decode('utf-8')


async def test_order():
    """Test order placement."""

    # Validate credentials
    if not all([API_KEY, SECRET_KEY, PASSPHRASE]):
        print("❌ Missing OKX API credentials!")
        print("Required environment variables:")
        print("  - OKX_API_KEY")
        print("  - OKX_SECRET_KEY")
        print("  - OKX_PASSPHRASE")
        print("\nTip: Create a .env file in demo/market-analyst/")
        return False

    print("=" * 60)
    print("OKX Order Placement Test")
    print("=" * 60)
    print(f"Mode: {'DEMO' if USE_DEMO else 'PRODUCTION'}")
    print(f"WebSocket: {WS_URL}")
    print(f"Instrument: {INST_ID}")
    print(f"Side: {SIDE.upper()}")
    print(f"Size: {SIZE}")
    print(f"Trade Mode: {TRADE_MODE}")
    print("=" * 60)
    print()

    session = None
    ws = None

    try:
        # Step 1: Connect
        print("[1/6] Connecting to WebSocket...")
        session = aiohttp.ClientSession()
        ws = await session.ws_connect(WS_URL, timeout=aiohttp.ClientTimeout(total=30))
        print("✅ Connected\n")

        # Step 2: Authenticate
        print("[2/6] Authenticating...")
        timestamp = str(int(datetime.now(timezone.utc).timestamp()))
        sign = generate_signature(timestamp, 'GET', '/users/self/verify')

        login_msg = {
            "op": "login",
            "args": [{
                "apiKey": API_KEY,
                "passphrase": PASSPHRASE,
                "timestamp": timestamp,
                "sign": sign
            }]
        }

        await ws.send_json(login_msg)
        print(f"Sent: {json.dumps(login_msg, indent=2)}\n")

        # Wait for login response
        authenticated = False
        async for msg in ws:
            if msg.type == aiohttp.WSMsgType.TEXT:
                data = json.loads(msg.data)
                print(f"Received: {json.dumps(data, indent=2)}\n")

                if data.get('event') == 'login':
                    if data.get('code') == '0':
                        authenticated = True
                        print("✅ Authentication successful\n")
                        break
                    else:
                        print(f"❌ Authentication failed: {data.get('msg')}\n")
                        return False

        if not authenticated:
            print("❌ Authentication timeout\n")
            return False

        # Step 3: Subscribe to orders channel
        print("[3/6] Subscribing to private orders channel...")
        sub_msg = {
            "op": "subscribe",
            "args": [
                {"channel": "orders", "instType": "SPOT"}
            ]
        }
        await ws.send_json(sub_msg)
        print(f"Sent: {json.dumps(sub_msg, indent=2)}\n")

        # Wait for subscription confirmation
        subscribed = False
        for _ in range(5):  # Check a few messages
            msg = await asyncio.wait_for(ws.receive(), timeout=2.0)
            if msg.type == aiohttp.WSMsgType.TEXT:
                data = json.loads(msg.data)
                print(f"Received: {json.dumps(data, indent=2)}\n")
                if data.get('event') == 'subscribe' and data.get('arg', {}).get('channel') == 'orders':
                    subscribed = True
                    print("✅ Subscribed to orders channel\n")
                    break

        if not subscribed:
            print("⚠️  Subscription confirmation not received (continuing anyway)\n")

        # Step 4: Place order
        print("[4/6] Placing market order...")
        # OKX id field: must be string, use timestamp in milliseconds
        request_id = str(int(time.time()*1000))
        # clOrdId: alphanumeric, max 32 chars, should not contain special chars except underscore/hyphen
        cl_ord_id = f"loom{int(time.time()*1000)}"  # Simpler format without underscore prefix

        order_msg = {
            "id": request_id,
            "op": "order",
            "args": [{
                "instId": INST_ID,
                "tdMode": TRADE_MODE,
                "side": SIDE,
                "ordType": "market",
                "sz": SIZE,
                "clOrdId": cl_ord_id,
                "tgtCcy": "quote_ccy"  # Size in USDT (quote currency)
            }]
        }

        await ws.send_json(order_msg)
        print(f"Sent: {json.dumps(order_msg, indent=2)}\n")

        # Step 5: Wait for order response
        print("[5/6] Waiting for order confirmation...")
        order_confirmed = False
        order_result = None

        start_time = time.time()
        timeout = 10.0

        while time.time() - start_time < timeout:
            try:
                msg = await asyncio.wait_for(ws.receive(), timeout=1.0)
                if msg.type == aiohttp.WSMsgType.TEXT:
                    data = json.loads(msg.data)
                    print(f"Received: {json.dumps(data, indent=2)}\n")

                    # Check for order response (matches by id field)
                    if data.get('op') == 'order' and data.get('id') == request_id:
                        if data.get('code') == '0':
                            order_confirmed = True
                            order_result = data
                            print("✅ Order placed successfully!\n")
                            break
                        else:
                            print(f"❌ Order failed: {data.get('msg')}\n")
                            return False

                    # Check for orders channel update
                    elif data.get('arg', {}).get('channel') == 'orders':
                        orders = data.get('data', [])
                        for order in orders:
                            if order.get('clOrdId') == cl_ord_id:
                                order_confirmed = True
                                order_result = data
                                print("✅ Order confirmed via orders channel!\n")
                                break
                        if order_confirmed:
                            break

                    # Check for error
                    elif data.get('event') == 'error':
                        print(f"❌ Error: {data.get('msg')}\n")
                        return False

            except asyncio.TimeoutError:
                continue

        if not order_confirmed:
            print(f"❌ Order timeout (no confirmation after {timeout}s)\n")
            return False

        # Step 6: Display results
        print("[6/6] Order Summary")
        print("=" * 60)
        if order_result:
            if order_result.get('op') == 'order':
                order_data = order_result.get('data', [{}])[0]
            else:
                order_data = order_result.get('data', [{}])[0]

            print(f"Order ID: {order_data.get('ordId', 'N/A')}")
            print(f"Client Order ID: {order_data.get('clOrdId', cl_ord_id)}")
            print(f"Instrument: {order_data.get('instId', INST_ID)}")
            print(f"Side: {order_data.get('side', SIDE).upper()}")
            print(f"Size: {order_data.get('sz', SIZE)}")
            print(f"Status: {order_data.get('state', 'unknown')}")
            print(f"Average Fill Price: {order_data.get('avgPx', 'N/A')}")
            print(f"Filled Size: {order_data.get('accFillSz', 'N/A')}")
        print("=" * 60)
        print()

        return True

    except Exception as e:
        print(f"❌ Exception: {type(e).__name__}: {e}\n")
        import traceback
        traceback.print_exc()
        return False

    finally:
        # Cleanup
        if ws:
            await ws.close()
        if session:
            await session.close()
        print("Connection closed.")


async def main():
    """Main entry point."""
    success = await test_order()

    if success:
        print("\n✅ Test PASSED - Order placement works!")
        print("\nNext steps:")
        print("1. Check OKX demo UI to verify the order")
        print("2. If successful, the executor agent code can use the same pattern")
        return 0
    else:
        print("\n❌ Test FAILED - See errors above")
        print("\nTroubleshooting:")
        print("1. Verify OKX API credentials in .env file")
        print("2. Check if demo trading account is enabled")
        print("3. Verify account has sufficient balance")
        print("4. Check OKX API status: https://www.okx.com/support/hc/en-us/categories/360000897692")
        return 1


if __name__ == "__main__":
    exit_code = asyncio.run(main())
    exit(exit_code)
