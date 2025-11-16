"""Planner Agent - Analysis Aggregation & Planning

Aggregates partial analyses and generates trading plans.
Integrates with LLM (DeepSeek) for intelligent reasoning.
"""

import asyncio
import json
import time
from collections import defaultdict
from dataclasses import dataclass, field
from typing import Any, Dict, Optional

from opentelemetry import trace

from loom import Agent, LLMProvider, load_project_config

# Get tracer for business logic spans
tracer = trace.get_tracer(__name__)

SYMBOL = "BTC"


@dataclass
class PlannerBuffer:
    """Buffer for collecting partial analyses."""

    timeout_sec: float = 3.0
    entries: Dict[str, Dict[str, Any]] = field(default_factory=dict)

    def update(self, symbol: str, topic: str, payload: Dict[str, Any]) -> None:
        """Update buffer with new analysis."""
        now = time.time()
        if symbol not in self.entries:
            self.entries[symbol] = {"first_ts": now, "partials": {}}
        self.entries[symbol]["partials"][topic] = payload

    def ready(self, symbol: str) -> bool:
        """Check if all analyses received or timeout expired."""
        entry = self.entries.get(symbol)
        if not entry:
            return False

        partials = entry["partials"]
        has_all = len(partials) >= 3  # trend + risk + sentiment

        if has_all:
            return True

        # Check timeout
        elapsed = time.time() - entry["first_ts"]
        return elapsed >= self.timeout_sec

    def take(self, symbol: str) -> Optional[Dict[str, Any]]:
        """Take and clear partials for symbol."""
        return self.entries.pop(symbol, {}).get("partials")

    def expired_symbols(self) -> list[str]:
        """Get symbols with expired timeout."""
        now = time.time()
        expired = []
        for symbol, entry in self.entries.items():
            if (now - entry["first_ts"]) >= self.timeout_sec:
                expired.append(symbol)
        return expired


planner_buffer = PlannerBuffer(timeout_sec=3.0)
llm_provider = None  # Will be initialized with context


async def make_plan(ctx, partials: Dict[str, Any], use_llm: bool = True) -> Dict[str, Any]:
    """Generate trading plan from partial analyses with memory awareness.

    Uses LLM (DeepSeek) for intelligent reasoning when available.
    Falls back to rule-based logic if LLM unavailable.
    Includes memory: queries recent plans and filters duplicates.
    """
    with tracer.start_as_current_span(
        "planner.make_plan",
        attributes={
            "planner.partials.count": len(partials),
            "planner.has_trend": "analysis.trend" in partials,
            "planner.has_risk": "analysis.risk" in partials,
            "planner.has_sentiment": "analysis.sentiment" in partials,
            "planner.use_llm": use_llm,
        },
    ) as span:
        trend_data = partials.get("analysis.trend", {})
        risk_data = partials.get("analysis.risk", {})
        sentiment_data = partials.get("analysis.sentiment", {})

        # Extract key metrics
        trend = trend_data.get("trend", "unknown")
        risk_score = risk_data.get("risk_score", 0.5)
        sentiment = sentiment_data.get("sentiment", "neutral")

        # 🆕 Query recent plans from Core Memory
        memory_context = ""
        try:
            recent_plans = await ctx.get_recent_plans(SYMBOL, limit=5)
            if recent_plans:
                memory_lines = [f"Recent trading decisions for {SYMBOL}:"]
                now_ms = int(time.time() * 1000)
                for p in recent_plans:
                    time_ago_sec = (now_ms - p["timestamp_ms"]) // 1000
                    memory_lines.append(
                        f"- {time_ago_sec}s ago: {p['action']} (confidence: {p['confidence']:.2f}) - {p['reasoning'][:80]}"
                    )
                memory_context = "\n".join(memory_lines)
                span.set_attribute("planner.memory.recent_plans", len(recent_plans))
            else:
                memory_context = f"No recent trading history for {SYMBOL}."
                span.set_attribute("planner.memory.recent_plans", 0)
        except Exception as e:
            print(f"[planner] Failed to query memory: {e}")
            memory_context = "Memory unavailable."
            span.add_event("memory_query_failed", {"error": str(e)})

        # Try LLM-based reasoning first
        if use_llm and llm_provider:
            try:
                # Build structured prompt for LLM with memory context
                prompt = f"""You are a professional cryptocurrency trading advisor. Based on the following market analysis and recent trading history, provide a clear trading recommendation.

{memory_context}

MARKET DATA FOR {SYMBOL}:

Trend Analysis:
{json.dumps(trend_data, indent=2)}

Risk Analysis:
{json.dumps(risk_data, indent=2)}

Sentiment Analysis:
{json.dumps(sentiment_data, indent=2)}

IMPORTANT: Review the recent trading history above to AVOID contradictory or duplicate decisions within short time windows. If a similar action was recently taken, consider HOLD or provide new reasoning.

Provide your recommendation in the following JSON format:
{{
    "action": "BUY" | "SELL" | "HOLD",
    "confidence": 0.0-1.0,
    "reasoning": "Brief explanation of your decision"
}}

Consider:
- Market trend direction and momentum
- Risk levels and volatility
- Market sentiment from recent news
- Current price context
- Recent trading history to avoid redundant decisions

Be concise and actionable."""

                system_prompt = "You are an expert cryptocurrency trading advisor. Provide clear, data-driven recommendations in JSON format only."

                # Call LLM
                response_text = await llm_provider.generate(
                    prompt=prompt,
                    system=system_prompt,
                    temperature=0.3,  # Lower temperature for more consistent output
                    max_tokens=500,
                )

                # Parse LLM response
                # Try to extract JSON from response
                response_text = response_text.strip()
                if "```json" in response_text:
                    response_text = response_text.split("```json")[1].split("```")[0].strip()
                elif "```" in response_text:
                    response_text = response_text.split("```")[1].split("```")[0].strip()

                llm_decision = json.loads(response_text)

                action = llm_decision.get("action", "HOLD")
                confidence = float(llm_decision.get("confidence", 0.5))
                reasoning = llm_decision.get("reasoning", "LLM-generated decision")

                # 🆕 Check for duplicate plans in Core Memory (5-minute window)
                is_duplicate = False
                try:
                    dup_check, dup_info = await ctx.check_duplicate_plan(
                        SYMBOL, action, reasoning, time_window_sec=300
                    )
                    if dup_check:
                        print(
                            f"[planner] Duplicate detected: {action} was planned {(time.time() * 1000 - dup_info['timestamp_ms']) / 1000:.0f}s ago"
                        )
                        span.add_event("duplicate_plan_detected", {
                            "original_plan_hash": dup_info["plan_hash"],
                            "time_ago_sec": int((time.time() * 1000 - dup_info["timestamp_ms"]) / 1000),
                        })
                        # Override to HOLD with modified reasoning
                        action = "HOLD"
                        confidence = max(0.3, confidence - 0.2)
                        reasoning = f"[DUPLICATE FILTERED] Similar plan recently created. Original: {reasoning[:60]}..."
                        is_duplicate = True
                except Exception as e:
                    print(f"[planner] Duplicate check failed: {e}")
                    span.add_event("duplicate_check_failed", {"error": str(e)})

                result = {
                    "symbol": SYMBOL,
                    "action": action,
                    "confidence": confidence,
                    "reasoning": reasoning,
                    "sources": {
                        "trend": trend_data,
                        "risk": risk_data,
                        "sentiment": sentiment_data,
                    },
                    "complete": len(partials) >= 3,
                    "method": "llm",
                    "timestamp_ms": int(time.time() * 1000),
                }

                # 🆕 Save plan to Core Memory
                try:
                    plan_hash = await ctx.save_plan(
                        SYMBOL, action, confidence, reasoning, "llm"
                    )
                    result["plan_hash"] = plan_hash
                    span.set_attribute("planner.plan_hash", plan_hash)
                    span.set_attribute("planner.is_duplicate", is_duplicate)
                    print(f"[planner] Saved plan to memory: hash={plan_hash}, duplicate={is_duplicate}")
                except Exception as e:
                    print(f"[planner] Failed to save plan: {e}")
                    span.add_event("save_plan_failed", {"error": str(e)})

                # Record LLM success
                span.set_attribute("planner.method", "llm")
                span.set_attribute("planner.action", result["action"])
                span.set_attribute("planner.confidence", result["confidence"])
                span.set_status(trace.Status(trace.StatusCode.OK))

                return result

            except Exception as e:
                print(f"[planner] LLM error: {e}, falling back to rule-based logic")
                span.add_event("llm_fallback", {"error": str(e)})

        # Fallback: Rule-based logic
        if trend == "up" and risk_score < 0.5 and sentiment in ["bullish", "neutral"]:
            action = "BUY"
            confidence = 0.75
        elif trend == "down" or risk_score > 0.7:
            action = "HOLD"
            confidence = 0.60
        else:
            action = "HOLD"
            confidence = 0.50

        reasoning = f"Trend: {trend}, Risk: {risk_score:.2f}, Sentiment: {sentiment}"

        # 🆕 Check for duplicate plans in Core Memory (5-minute window)
        is_duplicate = False
        try:
            dup_check, dup_info = await ctx.check_duplicate_plan(
                SYMBOL, action, reasoning, time_window_sec=300
            )
            if dup_check:
                print(
                    f"[planner] Duplicate detected: {action} was planned {(time.time() * 1000 - dup_info['timestamp_ms']) / 1000:.0f}s ago"
                )
                span.add_event("duplicate_plan_detected", {
                    "original_plan_hash": dup_info["plan_hash"],
                    "time_ago_sec": int((time.time() * 1000 - dup_info["timestamp_ms"]) / 1000),
                })
                # Override to HOLD with modified reasoning
                action = "HOLD"
                confidence = max(0.3, confidence - 0.2)
                reasoning = f"[DUPLICATE FILTERED] Similar plan recently created. Original: {reasoning[:60]}..."
                is_duplicate = True
        except Exception as e:
            print(f"[planner] Duplicate check failed: {e}")
            span.add_event("duplicate_check_failed", {"error": str(e)})

        result = {
            "symbol": SYMBOL,
            "action": action,
            "confidence": confidence,
            "reasoning": reasoning,
            "sources": {
                "trend": trend_data,
                "risk": risk_data,
                "sentiment": sentiment_data,
            },
            "complete": len(partials) >= 3,
            "method": "rule-based",
            "timestamp_ms": int(time.time() * 1000),
        }

        # 🆕 Save plan to Core Memory
        try:
            plan_hash = await ctx.save_plan(
                SYMBOL, action, confidence, reasoning, "rule-based"
            )
            result["plan_hash"] = plan_hash
            span.set_attribute("planner.plan_hash", plan_hash)
            span.set_attribute("planner.is_duplicate", is_duplicate)
            print(f"[planner] Saved plan to memory: hash={plan_hash}, duplicate={is_duplicate}")
        except Exception as e:
            print(f"[planner] Failed to save plan: {e}")
            span.add_event("save_plan_failed", {"error": str(e)})

        # Record rule-based result
        span.set_attribute("planner.method", "rule-based")
        span.set_attribute("planner.action", result["action"])
        span.set_attribute("planner.confidence", result["confidence"])
        span.set_attribute("planner.trend", trend)
        span.set_attribute("planner.risk_score", risk_score)
        span.set_attribute("planner.sentiment", sentiment)
        span.set_status(trace.Status(trace.StatusCode.OK))

        return result


async def planner_handler(ctx, topic: str, event) -> None:
    """Collect partial analyses and emit plan when ready."""
    with tracer.start_as_current_span(
        "planner.aggregate",
        attributes={
            "planner.topic": topic,
            "planner.symbol": SYMBOL,
        },
    ) as span:
        data = json.loads(event.payload.decode("utf-8"))
        symbol = data.get("symbol", SYMBOL)

        planner_buffer.update(symbol, topic, data)

        # Record buffer state
        entry = planner_buffer.entries.get(symbol, {})
        partials = entry.get("partials", {})
        span.set_attribute("planner.buffer.size", len(partials))
        span.set_attribute("planner.buffer.topics", list(partials.keys()))

        # Check if ready to plan
        if planner_buffer.ready(symbol):
            partials = planner_buffer.take(symbol) or {}
            span.set_attribute("planner.ready", True)
            span.set_attribute("planner.complete", len(partials) >= 3)

            plan = await make_plan(ctx, partials)

            status = "complete" if plan["complete"] else "timeout"
            method = plan.get("method", "unknown")

            # Record plan result
            span.set_attribute("planner.status", status)
            span.set_attribute("planner.plan.action", plan["action"])
            span.set_attribute("planner.plan.confidence", plan["confidence"])
            span.set_attribute("planner.plan.method", method)

            print(f"[planner] Plan ready ({status}/{method}): {plan['action']} (conf: {plan['confidence']:.2f})")
            print(f"[planner]   Reasoning: {plan['reasoning']}")

            await ctx.emit(
                "plan.ready",
                type="plan.ready",
                payload=json.dumps(plan).encode("utf-8"),
            )

            span.set_status(trace.Status(trace.StatusCode.OK))
        else:
            span.set_attribute("planner.ready", False)
            span.add_event("waiting_for_more_analyses")


async def planner_timeout_loop(ctx) -> None:
    """Check for expired symbols periodically."""
    while True:
        await asyncio.sleep(0.5)

        for symbol in planner_buffer.expired_symbols():
            partials = planner_buffer.take(symbol) or {}
            if partials:
                plan = await make_plan(ctx, partials)
                method = plan.get("method", "unknown")
                print(f"[planner] Plan ready (timeout/{method}): {plan['action']} (conf: {plan['confidence']:.2f})")
                print(f"[planner]   Partial data: {list(partials.keys())}")

                await ctx.emit(
                    "plan.ready",
                    type="plan.ready",
                    payload=json.dumps(plan).encode("utf-8"),
                )


async def main():
    global llm_provider

    config = load_project_config()
    agent_config = config.agents.get("planner-agent", {})

    topics = agent_config.get("topics", ["analysis.trend", "analysis.risk", "analysis.sentiment"])
    timeout_ms = agent_config.get("timeout_ms", 3000)
    llm_name = agent_config.get("llm_provider", "deepseek")

    # Update buffer timeout
    planner_buffer.timeout_sec = timeout_ms / 1000

    agent = Agent(
        agent_id="planner-agent",
        topics=topics,
        on_event=planner_handler,
    )

    print(f"[planner] Planner Agent starting")
    print(f"[planner] Subscribed to: {topics}")
    print(f"[planner] Timeout: {timeout_ms}ms")
    print(f"[planner] LLM Provider: {llm_name}")

    await agent.start()

    # Initialize LLM provider from config
    try:
        llm_provider = LLMProvider.from_config(agent._ctx, llm_name, config)
        print(f"[planner] LLM provider initialized: {llm_name}")
        if llm_provider.config.api_key:
            masked_key = llm_provider.config.api_key[:8] + "..." if len(llm_provider.config.api_key) > 8 else "***"
            print(f"[planner]   API Key: {masked_key}")
        print(f"[planner]   Model: {llm_provider.config.model}")
        print(f"[planner]   Base URL: {llm_provider.config.base_url}")
    except Exception as e:
        print(f"[planner] Failed to initialize LLM provider: {e}")
        print(f"[planner] Will use rule-based planning only")

    # Start timeout checker
    asyncio.create_task(planner_timeout_loop(agent._ctx))

    # Keep running
    try:
        await asyncio.Event().wait()
    except KeyboardInterrupt:
        print("[planner] Shutting down...")
        await agent.stop()


if __name__ == "__main__":
    asyncio.run(main())
