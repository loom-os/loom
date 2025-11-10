# Loom Bridge (gRPC) — Protocol and Usage

The Bridge connects external SDK agents (Python/JS) to Loom Core via gRPC. It provides:

- Agent registration (subscriptions and capability descriptors)
- Bidirectional event streaming (client → publish; server → deliveries)
- Action forwarding to ActionBroker
- Optional heartbeat
- Reconnection-friendly behavior

## Services

service Bridge

- RegisterAgent(AgentRegisterRequest) → AgentRegisterResponse
- EventStream(stream ClientEvent) ↔ (stream ServerEvent)
- ForwardAction(ActionCall) → ActionResult
- Heartbeat(HeartbeatRequest) → HeartbeatResponse

## Stream handshake

- The server expects the first stream message to be an Ack carrying `agent_id`.
- Clients must enqueue this Ack into the outbound stream BEFORE awaiting the RPC result, otherwise both sides can deadlock (server waits for Ack; client waits for response).

Client outline (tonic):

- Create `mpsc::channel` → wrap with `ReceiverStream` as outbound
- Send first `ClientEvent::Ack { message_id: agent_id }` into the channel
- Call `client.event_stream(outbound).await?` and use the returned inbound stream

## Event publish/receive

- After registering with `subscribed_topics`, any publish to those topics is delivered on the server→client stream as `ServerEvent::Delivery`.
- QoS mapping: current default uses `QoS_Batched` with bounded channel sizes and drops applied to realtime when backpressured.

## Action forwarding modes

- Client-initiated: Call `ForwardAction(ActionCall)` to run capabilities registered in the Loom Core `ActionBroker`.
- Server-initiated (planned): The protocol includes `ServerEvent::action_call` and `ClientEvent::action_result` variants for pushing actions to agents and receiving results back on the same stream. The service will add an entry point to trigger server push in a future patch.

## Heartbeat

- Optional unary endpoint `Heartbeat` or inline stream ping/pong (`ClientEvent::ping` / `ServerEvent::pong`).

## Reconnection

- Bridge is stateless. On stream end, the server cleans up the agent’s sender; clients can re-register with the same agent_id and re-open a stream.

## Testing notes

- Integration tests live under `bridge/tests/integration`: basic register/stream/publish and ForwardAction echo.
- Unit tests live under `bridge/tests/unit`: register_agent, heartbeat, forward_action success/error.
- For stream tests, always send the Ack before awaiting `event_stream` to avoid handshake deadlocks.

## Future improvements

- Server-initiated action push + correlation of action_result
- Backpressure/metrics exposure on the Bridge surface
- AuthN/Z and namespaces/ACLs
