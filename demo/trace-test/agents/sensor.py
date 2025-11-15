"""Sensor Agent - Generates test data every 2 seconds."""

import asyncio
import json
import time

from loom import Agent, init_telemetry, shutdown_telemetry
from opentelemetry import trace

# Get tracer for creating root spans
tracer = trace.get_tracer(__name__)


async def main():
    # Initialize telemetry BEFORE creating agent
    init_telemetry(service_name="trace-test-sensor")

    agent = Agent(
        agent_id="sensor-agent",
        topics=[],  # No subscriptions, only emits
        on_event=None,
    )

    print("[sensor] Starting sensor agent")
    await agent.start()

    # Generate data periodically
    counter = 0
    while True:
        counter += 1
        data = {
            "sensor_id": "sensor-1",
            "value": 20 + (counter % 10),
            "timestamp": time.time(),
            "counter": counter,
        }

        print(f"[sensor] Emitting data #{counter}: {data['value']}")

        # Create root span for each emission to start a new trace
        with tracer.start_as_current_span(
            "sensor.emit_data",
            attributes={
                "sensor.id": "sensor-1",
                "data.counter": counter,
                "data.value": data["value"],
            },
        ):
            await agent._ctx.emit(
                "sensor.data",
                type="sensor.reading",
                payload=json.dumps(data).encode("utf-8"),
            )

        await asyncio.sleep(2)


if __name__ == "__main__":
    try:
        asyncio.run(main())
    except KeyboardInterrupt:
        print("[sensor] Shutting down...")
        shutdown_telemetry()
