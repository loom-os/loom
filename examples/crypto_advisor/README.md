# Crypto Advisor (Planned Example)

A real-time, event-driven investment assistant for crypto assets. It fuses market microstructure (candles, trades, order books), sentiment (social/news), and user preferences (risk budget, style) to generate recommendations and, later, supervised execution.

Status: Spec only. Implementation postponed until the core OS reaches a higher completion level.

## Why Loom fits

- Stream-native: markets and social signals are high-frequency streams that benefit from QoS/backpressure and bounded queues.
- Hybrid routing: run indicators and risk checks locally; escalate to cloud for long-form reasoning. Privacy- and budget-aware.
- Multi-agent: cleanly separates indicator, sentiment, signal fusion, risk, execution, and watchdog agents with observable events.
- Auditability: every decision/action is event-sourced (reason, confidence, policy), essential for finance workflows.

## MVP scope (paper trading)

- Assets: Spot BTC/ETH on a single exchange (e.g., Binance/Coinbase).
- Data sources:
  - WebSocket: candles (1s/5s/1m), trades, top-of-book deltas.
  - HTTP: reference prices, balances (paper).
  - Sentiment: X/Twitter API if available; otherwise RSS/news + basic heuristics.
- Signals:
  - Technical: SMA crossover, RSI, MACD, ATR (volatility regime).
  - Sentiment: rule-based/lexicon + volume spike heuristics (later small model via ONNX/TFLite).
- Risk/policy:
  - Per-trade cap, daily loss cap, leverage=1, stop-loss/take-profit, cooldown windows.
- Execution:
  - Paper broker simulator with slippage and fees; approval gates for larger orders.
- UX:
  - Simple dashboard/log stream: positions, signals, actions; one-click approve/stop.

Acceptance (MVP):

- End-to-end loop stable for ≥ 1 week on paper; all actions logged with reasons.
- Latency p95 from signal to action < 200 ms on desktop.
- Cloud token spend below target; demonstrate local-first savings.

## Architecture (topics and agents)

Events (examples):

- market.candle, market.trade, orderbook.delta (QoSRealtime/QoSBatched)
- sentiment.post, sentiment.score (QoSBatched)
- portfolio.state, risk.limit_update (QoSBackground)
- signal.{buy|sell|flat}, order.{place|fill|cancel}, strategy.metrics

Agents:

- IndicatorAgent → publishes indicator._ from market._
- SentimentAgent → ingests posts/news → sentiment.score
- SignalAgent → fuses indicators + sentiment → signal.\* with confidence
- RiskAgent → applies exposure/limits, can veto/annotate signals
- ExecutionAgent → turns approved signals into (paper) orders and handles fills
- WatchdogAgent → kill-switch on anomalies or loss caps; health monitoring

Local model usage:

- LocalModel trait for quick confidence and lightweight classifiers/anomaly detectors
- Cloud refine for long-form summaries or weekly wrap-ups; Router enforces policy

## Data and connectors

- Exchange adapters (plugins): WebSocket/REST clients with reconnect and rate-limiting.
- Sentiment adapters: X/Twitter API (if keys), RSS/news providers; dedupe and source trust weights.
- Storage: RocksDB for snapshots; optional parquet/CSV for offline backtest.
- Telemetry: latency histograms, PnL, drawdowns, token usage.

## Risks and mitigations

- Microstructure noise: use volume filters, sanity-check snapshots, cooldown after spikes.
- Overfitting/leakage: strict splits and walk-forward validation; transaction cost modeling.
- Compliance/legal: not financial advice; paper trading default; explicit user consent for auto-trade; holding caps.
- API fragility: provider fallbacks; exponential backoff; circuit breakers.

## Phased roadmap

- Phase 0: Read-only recommendations with reasons + confidence; paper PnL and metrics.
- Phase 1: Limited auto-trade under hard caps with approvals; sandbox/sim exchange.
- Phase 2: Strategy ensemble (mean-reversion + breakout), adaptive risk, hedging; human override.

## Implementation pointers (when picking up)

- Start with the example agents under `core/examples/` as references for EventBus and ActionBroker usage.
- Use the `LocalModel` stub for confidence estimation; later plug ONNX/TFLite models behind feature flags.
- Define proto messages for `market.*`, `sentiment.*`, and `broker.*` actions (paper broker first).
- Add an offline replay harness to evaluate strategies on recorded streams.
