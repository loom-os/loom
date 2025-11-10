"""Example trio: Planner -> Researcher -> Writer

Run three processes (or threads) each invoking its Agent.
For brevity, single script spawns asyncio tasks.
Requires loom bridge server running (cargo run -p loom-bridge --bin loom-bridge-server).
"""
import asyncio
from loom import Agent, capability

@capability("research.search", version="1.0")
def search(query: str) -> dict:
    return {"query": query, "results": ["https://example.com/doc1", "https://example.com/doc2"]}

async def planner_handler(ctx, topic, event):
    # When receiving a user question event, forward to researcher
    if event.type == "user.question":
        await ctx.emit("topic.research", type="research.request", payload=event.payload)

async def researcher_handler(ctx, topic, event):
    if event.type == "research.request":
        results = search(query=event.payload.decode())
        await ctx.emit("topic.writer", type="writer.draft", payload=("Draft based on " + str(results)).encode())

async def writer_handler(ctx, topic, event):
    if event.type == "writer.draft":
        final = event.payload + b"\nSUMMARY: OK"
        print("Final output:\n", final.decode())

async def main():
    planner = Agent("planner", topics=["topic.planner", "topic.research"], on_event=planner_handler)
    researcher = Agent("researcher", topics=["topic.research", "topic.writer"], capabilities=[search], on_event=researcher_handler)
    writer = Agent("writer", topics=["topic.writer"], on_event=writer_handler)

    await planner.start()
    await researcher.start()
    await writer.start()

    # Seed question
    await planner._ctx.emit("topic.planner", type="user.question", payload=b"What is Loom?")
    await asyncio.sleep(1.5)

    await planner.stop(); await researcher.stop(); await writer.stop()

if __name__ == "__main__":
    asyncio.run(main())
