"""Processor Agent - Receives sensor data, processes it, and emits processed data."""

import asyncio
import json

from loom import Agent, init_telemetry, shutdown_telemetry


async def processor_handler(ctx, topic: str, event) -> None:
    """Process sensor data and emit processed result."""
    data = json.loads(event.payload.decode("utf-8"))

    # Simulate processing
    processed = {
        "original_value": data["value"],
        "processed_value": data["value"] * 1.5,
        "sensor_id": data["sensor_id"],
        "counter": data["counter"],
        "processing_timestamp": data["timestamp"],
    }

    print(f"[processor] Processed data #{data['counter']}: {data['value']} → {processed['processed_value']}")

    await ctx.emit(
        "processed.data",
        type="processed.reading",
        payload=json.dumps(processed).encode("utf-8"),
    )


async def main():
    # Initialize telemetry BEFORE creating agent
    init_telemetry(service_name="trace-test-processor")

    agent = Agent(
        agent_id="processor-agent",
        topics=["sensor.data"],
        on_event=processor_handler,
    )

    print("[processor] Starting processor agent")
    print("[processor] Subscribed to: sensor.data")
    await agent.start()

    # Keep running
    try:
        await asyncio.Event().wait()
    except KeyboardInterrupt:
        print("[processor] Shutting down...")
        await agent.stop()
        shutdown_telemetry()


if __name__ == "__main__":
    asyncio.run(main())
