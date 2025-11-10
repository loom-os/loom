## Action Broker

Responsibility

- Register and dispatch capability implementations (actions).
- Enforce capability-level permissions and propagate errors back to callers.

Key files

- `core/src/action_broker.rs` — registration, permission checks, and dispatch logic.

Key interfaces

- Capability registration API: register capability name, descriptor, and handler.
- Invocation API: call capability with inputs and obtain results or structured errors.

Common error paths and test cases

- Unauthorized invocation: ensure permission checks block execution and return structured errors.
- Capability runtime errors: capability panics or failures must be converted to ActionResult errors and published.
- Timeout and cancellation: verify long-running actions respect deadlines and cancel tokens.

Tuning

- Invocation concurrency limits per capability.
- Timeouts and retry policies for remote capability providers.

Example (mock capability)

- A test mock provider should implement the capability interface and publish an `ActionResult` event consumed by the EventBus to validate round-trip behavior.
