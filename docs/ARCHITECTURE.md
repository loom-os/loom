# Loom Architecture

## Overview

Loom is an event-driven AI operating system that models intelligent agents as **stateful event-responsive entities**.

## Core Design Principles

1. **Event-First**: All inputs modeled as events, not synchronous calls
2. **Async-First**: Fully asynchronous using Tokio runtime
3. **Stateful**: Agents maintain persistent state and ephemeral context
4. **Composable**: Extensible through plugins and tools
5. **Observable**: Built-in tracing, metrics, and logging

## System Layers

```
┌─────────────────────────────────────┐
│     Application Layer               │  examples/
│  Demo Apps, Custom Agents           │
├─────────────────────────────────────┤
│     Plugin Layer                    │  plugins/
│  Feature Extractors, Models, Tools  │
├─────────────────────────────────────┤
│     Runtime Layer                   │  core/
│  Event Bus, Agent Runtime, Router   │
├─────────────────────────────────────┤
│     Infrastructure Layer            │  infra/
│  Storage, Network, Telemetry        │
└─────────────────────────────────────┘
```

## Core Components

### Event Bus

Asynchronous pub/sub message system with:

- **QoS Levels**:

  - `Realtime`: Low latency, best-effort delivery
  - `Batched`: Batch processing, guaranteed delivery
  - `Background`: Background tasks, delay-tolerant

- **Topic Routing**:

  ```
  camera.front.face      → Front camera face detection
  mic.primary.speech     → Primary microphone speech
  sensor.imu.motion      → IMU motion events
  agent.{id}.intent      → Agent intent events
  ```

- **Backpressure Handling**: Sampling, dropping old events, aggregation

- **Event Structure**:
  ```rust
  Event {
      id: String,
      type: String,
      timestamp_ms: i64,
      source: String,
      metadata: Map<String>,
      payload: Bytes,
      confidence: f32,
      tags: Vec<String>,
      priority: i32,
  }
  ```

### Agent Runtime

Actor-based stateful agents with:

- **Lifecycle Management**: Create, start, stop, delete agents
- **State Persistence**: RocksDB for long-term state
- **Event Distribution**: Mailbox pattern for event delivery
- **Behavior Execution**: Async event handlers

**Agent Model**:

```rust
Agent {
    config: AgentConfig,
    state: {
        persistent_state: Bytes,   // Persisted in RocksDB
        ephemeral_context: Bytes,  // In-memory sliding window
        last_update_ms: i64,
    },
    behavior: AgentBehavior,
    mailbox: mpsc::Receiver,
}
```

**Memory System**:

- **Episodic**: Event sequences
- **Semantic**: Knowledge graph
- **Working**: Active context

### Model Router

Intelligent routing engine that decides where to run inference:

**Decision Algorithm**:

```
Input: Event, AgentContext, RoutingPolicy
Output: Route, Confidence, Reason

1. Check privacy policy
   if privacy == "local-only" → Local

2. Check local capability
   if not local_supported → Cloud

3. Run local quick inference
   local_confidence = local_model(event)

4. Apply threshold rules
   if local_confidence >= threshold → Local

5. Check latency budget
   if latency_budget < 100ms → Local

6. Check cost constraints
   if cloud_cost > cap → LocalFallback

7. Hybrid strategy
   if 0.5 < local_confidence < threshold → Hybrid

8. Default → Cloud
```

**Routing Strategies**:

- **Rule-Based** (current): Fast but rigid if-else rules
- **ML-Based** (future): Learned classifier using event features and performance history

### Plugin System

Extensible plugin architecture:

**Plugin Types**:

1. **Feature Extractor**: Face detection, pose estimation, audio features
2. **Model Backend**: TFLite, ONNX, TorchScript wrappers
3. **Tool/API**: Calendar, search, database integration
4. **Actuator**: TTS, UI rendering, robot control

**Interface**:

```protobuf
service Plugin {
  rpc Init(PluginMeta) returns (Status);
  rpc HandleEvent(Event) returns (PluginResponse);
  rpc Health() returns (HealthStatus);
  rpc Shutdown() returns (Status);
}
```

**Security Isolation**:

- WASM sandboxing (recommended)
- Separate process + RPC
- Resource limits (CPU/memory/network)
- Capability declaration and authorization

### Storage Layer

**Storage Types**:

1. **KV Store (RocksDB)**: Agent state, metadata, event logs
2. **Vector DB**: Long-term memory embeddings (Milvus/FAISS/Weaviate)
3. **Object Store** (optional): Large files (video/audio) via S3/MinIO

**Data Lifecycle**:

```
Hot (memory) → 5 min → Warm (RocksDB) → 1 day → Cold (Vector DB) → 30 days → Archive/Delete
```

### Telemetry

Built-in observability:

**Metrics**:

- Throughput: events/sec
- Latency: P50/P99/Max
- Routing: local_rate, cloud_rate
- Resources: CPU/GPU/Memory
- Cost: Estimated cloud API usage

**Tracing** (OpenTelemetry):

```
Span: PublishEvent
  └─ Span: RouteDecision
      ├─ Span: LocalInference
      └─ Span: CloudRequest
```

**Logging**: Structured JSON logs (DEBUG/INFO/WARN/ERROR) with sensitive data masking

## Data Flow Examples

### Example 1: Real-time Face Emotion Recognition

```
Camera → VideoFrame Event
  → EventBus.publish("camera.front")
    → FaceAgent receives event
      → Router: Privacy OK, LocalModel confidence=0.92 → Local
        → Plugin: face-detector → {expression: "happy"}
          → Agent generates Action: ui_update {emoji: "😊"}
            → UI updates
```

### Example 2: Hybrid Voice Assistant

```
Mic → AudioChunk Event
  → EventBus.publish("mic.primary.speech")
    → VoiceAgent receives
      → Router: Hybrid strategy
        ├─ Local: whisper-tiny → "what's the weather" (0.7 conf)
        │   → UI shows immediate feedback
        └─ Cloud: GPT-4 → refined intent
            → Weather API tool call
              → TTS plugin speaks result
```

## Component Interaction

**Interaction Matrix**:

| Component     | Event Bus | Agent Runtime | Router | Plugin | Storage |
| ------------- | --------- | ------------- | ------ | ------ | ------- |
| Event Bus     | -         | Send events   | -      | -      | Log     |
| Agent Runtime | Subscribe | -             | Query  | Call   | R/W     |
| Router        | -         | Return route  | -      | -      | Perf    |
| Plugin        | Publish   | -             | -      | -      | -       |
| Storage       | -         | -             | -      | -      | -       |

**Key Collaboration Patterns**:

1. **Event-Driven Pipeline**: `Event Source → Event Bus → Agent → Plugin → Action`
2. **Stateful Processing**: Agent reads/updates state from Storage on each event
3. **Routing Optimization**: Agent queries Router for Local/Cloud/Hybrid decision
4. **Plugin Composition**: Agent calls multiple plugins and fuses results

## Performance Targets

**Latency**:

- Event Bus: < 1ms (in-memory routing)
- Agent Dispatch: < 5ms
- Local Model: 10-100ms
- Cloud Model: 200-2000ms

**Throughput**:

- Event Bus: 10k events/sec (single node)
- Agent Runtime: 100 concurrent agents
- Storage: 5k writes/sec

**Resources**:

- Memory: < 2GB (edge devices)
- GPU: Shared inference engine
- Network: Optimized payload size

## Security & Privacy

**Privacy Protection**:

- Tiered policies (Public/Sensitive/Private/LocalOnly)
- Optional payload encryption
- Minimal data upload (embeddings > raw data)

**Access Control**:

- Plugin capability declaration
- Runtime permission checks
- Audit logging

**Compliance**:

- GDPR: User data deletion
- Transparency: Explainable decisions
- Consent management

---

For detailed API documentation, see the source code.
