## Model Router (Router)

Responsibility

- Evaluate routing policies and select model/provider instances for requests.
- Emit routing decisions for observability and testing.

Key files

- `core/src/router.rs` — routing engine and policy evaluation.

Policy dimensions

- Privacy: filter out providers that violate data residency or privacy constraints.
- Latency: prefer models with known low-latency characteristics when budgets are tight.
- Cost: select lower-cost providers when quality requirements are met.
- Quality: map request intent to capable models (capability matching).

Common error paths and test cases

- Routing fallthrough: when no provider matches, the system must surface a deterministic error and emit a `routing_decision` indicating no match.
- Policy conflicts: verify conflict resolution (priority, weights) yields reproducible decisions.
- Threshold boundaries: test near-threshold behavior for latency/cost rules.

Tuning and observability

- Policy thresholds (latency, cost, score) should be configurable and tested.
- Emit `routing_decision` events to enable assertions in integration tests.
