# Crypto Advisor – Backlog (Prioritized)

Legend: P0 = critical for MVP, P1 = nice-to-have for MVP+, P2 = later

## P0 – MVP (paper trading)

- [ ] Data ingestion – exchange adapter (WebSocket/REST)
  - [ ] Candles (1s/5s/1m) and trades
  - [ ] Top-of-book deltas with debouncing
  - [ ] Reconnect/backoff, clock skew handling
- [ ] Indicators – sliding-window TA
  - [ ] SMA crossover (fast/slow)
  - [ ] RSI, MACD, ATR (volatility regime)
  - [ ] Publish indicator.\* events
- [ ] Sentiment – minimal
  - [ ] RSS/news adapter; simple rule-based scoring and dedupe
  - [ ] Publish sentiment.post and sentiment.score
- [ ] Signal fusion
  - [ ] Combine indicators + sentiment to signal.{buy|sell|flat}
  - [ ] Confidence estimation via LocalModel stub
- [ ] Risk & policy
  - [ ] Per-trade cap, daily loss cap, cooldown window
  - [ ] Veto or annotate signals; publish risk.\* events
- [ ] Execution – paper broker
  - [ ] place_order/cancel_order actions; fills with slippage + fees
  - [ ] Basic portfolio accounting; publish order.\* and portfolio.state
- [ ] Telemetry & logs
  - [ ] Latency, PnL, drawdown; event-sourced decisions with reasons
- [ ] UX – minimal
  - [ ] CLI/dashboard log stream; approve/stop toggle

## P1 – Hardening and quality

- [ ] Backtest/replay harness over recorded streams
- [ ] Strategy evaluation metrics (Sharpe, hit rate, turnover)
- [ ] Multiple venues fallback; rate-limit guards and circuit breaker
- [ ] Token budget enforcement (cloud calls) with Router policies
- [ ] Configurable strategy params per asset

## P2 – Extensions

- [ ] Small ONNX/TFLite local models for anomaly detection or micro-signal scoring
- [ ] X/Twitter sentiment (API) with source trust weighting
- [ ] Limited auto-trading under caps with per-trade approvals
- [ ] Strategy ensembles (mean-reversion + breakout) and adaptive risk
- [ ] Weekly wrap-ups with citations (cloud refine), exported reports

## Technical Notes

- All agents communicate over EventBus with QoS: market._ realtime/batched, sentiment._ batched, risk/portfolio background.
- Use Router’s Local/Hybrid paths: local for indicators/risk, hybrid for long-form summaries.
- Start with paper trading; keep compliance-safe defaults and explicit non-advice labeling.
