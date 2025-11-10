# loom-bridge

A gRPC bridge for external SDK agents (Python/JS) to connect to Loom Core.

- Register agents with subscriptions and capabilities
- Bidirectional event stream
- ForwardAction invocation via ActionBroker
- Heartbeat endpoint

## Run

Set address (optional, defaults to 0.0.0.0:50051):

```
export LOOM_BRIDGE_ADDR=127.0.0.1:50051
cargo run -p loom-bridge --bin loom-bridge-server
```

## Client handshake (important)

The server expects the first stream message to be an Ack containing `agent_id`.
Enqueue this Ack into the outbound channel before awaiting the RPC response to avoid a deadlock.

See `bridge/tests/integration/e2e_basic.rs` for a working example using `ReceiverStream`.

## Tests

- Integration tests: `bridge/tests/integration` (e2e_basic, e2e_forward_action)
- Unit tests: `bridge/tests/unit` (register_agent, heartbeat, forward_action)
