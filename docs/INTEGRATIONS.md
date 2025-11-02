# Integrations

Loom interoperates with popular ecosystems. We complement, not replace, your existing stack.

## LangChain / LlamaIndex

- Treat chains/tools/agents as out‑of‑process plugins via gRPC.
- Adapter: map Loom topics to chain inputs; return tool outputs as Loom actions/events.
- When: leverage rich tool libraries without re‑writing logic.

## vLLM and Semantic Router

- Use vLLM as local or remote serving behind the Model Router.
- Optional: feed vLLM Semantic Router signals into routing policy (privacy/latency/cost hints).
- When: need high‑throughput inference and fine‑grained routing.

## Kubernetes

- Run Loom runtime and gRPC plugins as Deployments.
- Use ConfigMap/Secret for policy and credentials; standard logging/metrics.
- When: server/edge clusters, multi‑tenant setups, A/B routing.

## n8n and workflow tools

- Provide a Loom node: subscribe to topics, emit actions/events.
- When: low‑code automation, quick integrations with SaaS/APIs.

## Suggested priority

1. vLLM backend adapter
2. LangChain tool adapter
3. n8n node
4. Kubernetes example manifests
