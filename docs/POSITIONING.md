# Positioning

Loom is an event‑driven AI operating layer (runtime), not a host OS. We orchestrate sensing → reasoning → acting across apps and devices while the host OS handles processes, drivers, and enforcement.

## One‑liner

From prompts to events — Loom runs always‑on, stateful, asynchronous agents that subscribe to real‑time multimodal streams and emit actions with QoS, backpressure, and intelligent edge/cloud routing.

## What it is vs. isn’t

- Is: a runtime layer for event‑driven, stateful agents; an orchestration of events → state → actions; a thin, portable core for mobile and edge.
- Isn’t: a host operating system; a traditional request‑response chatbot framework; a monolithic “AI OS”.

## Why different from chatbots

Chatbots are synchronous and stateless by default. Real‑world agents are continuous, asynchronous, and stateful. Loom models the world as event streams, keeps working/long‑term state, and routes inference across local and cloud backends based on privacy, latency, and cost.

## Category name

Event‑Driven AI Operating Layer (runtime).

## Elevator pitch

Loom provides an event bus with QoS/backpressure, a stateful agent runtime, an intelligent model router (edge/cloud/hybrid), and a tiered plugin system (Rust, WASM, out‑of‑process via gRPC). It’s designed mobile‑first, with small footprint and fast cold start, yet scales to desktop/server deployments and rich integrations.
