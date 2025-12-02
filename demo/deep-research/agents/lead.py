"""Lead Agent - Research Orchestrator

Receives user queries, decomposes into sub-tasks, spawns researcher agents,
and aggregates results into a final report.
"""

import asyncio
import json
import os
import time
from dataclasses import dataclass, field
from datetime import datetime
from typing import Any, Dict, List, Optional

from opentelemetry import trace

from loom import Agent, LLMProvider, load_project_config

tracer = trace.get_tracer(__name__)

# =============================================================================
# Data Structures
# =============================================================================


@dataclass
class SubQuery:
    """A decomposed sub-query for a researcher."""

    id: str
    query: str
    focus_area: str
    priority: int = 1


@dataclass
class ResearchSection:
    """A completed research section from a researcher."""

    sub_query_id: str
    title: str
    content: str
    sources: List[Dict[str, str]] = field(default_factory=list)
    timestamp_ms: int = 0


@dataclass
class ResearchState:
    """State for tracking ongoing research."""

    original_query: str
    sub_queries: List[SubQuery] = field(default_factory=list)
    pending_researchers: List[str] = field(default_factory=list)
    completed_sections: List[ResearchSection] = field(default_factory=list)
    start_time: float = 0.0


# Global state (will be per-thread in production)
research_state: Optional[ResearchState] = None
llm_provider: Optional[LLMProvider] = None


# =============================================================================
# Query Decomposition
# =============================================================================


async def decompose_query(ctx, query: str, max_sub_queries: int = 3) -> List[SubQuery]:
    """Use LLM to decompose a query into sub-queries.

    Args:
        ctx: Agent context
        query: User's research query
        max_sub_queries: Maximum number of sub-queries to generate

    Returns:
        List of SubQuery objects
    """
    with tracer.start_as_current_span(
        "lead.decompose_query",
        attributes={"query": query, "max_sub_queries": max_sub_queries},
    ):
        if not llm_provider:
            # Fallback: simple decomposition without LLM
            return [
                SubQuery(
                    id="sq-1",
                    query=f"{query} frameworks and tools",
                    focus_area="tools",
                ),
                SubQuery(
                    id="sq-2",
                    query=f"{query} real-world applications",
                    focus_area="applications",
                ),
                SubQuery(
                    id="sq-3",
                    query=f"{query} challenges and future",
                    focus_area="challenges",
                ),
            ]

        prompt = f"""You are a research planner. Decompose the following research query into {max_sub_queries} distinct sub-queries that together will provide comprehensive coverage.

USER QUERY: {query}

For each sub-query, provide:
1. A focused search query (optimized for web search)
2. The focus area it covers

Return as JSON array:
[
    {{"id": "sq-1", "query": "specific search query", "focus_area": "area name", "priority": 1}},
    ...
]

Only return the JSON array, no other text."""

        try:
            response = await llm_provider.generate(
                prompt=prompt,
                system="You are a research planning assistant. Return only valid JSON.",
                temperature=0.3,
            )

            # Parse JSON response
            response = response.strip()
            if response.startswith("```"):
                response = response.split("```")[1]
                if response.startswith("json"):
                    response = response[4:]

            sub_queries_data = json.loads(response)

            return [
                SubQuery(
                    id=sq.get("id", f"sq-{i}"),
                    query=sq["query"],
                    focus_area=sq.get("focus_area", "general"),
                    priority=sq.get("priority", 1),
                )
                for i, sq in enumerate(sub_queries_data[:max_sub_queries])
            ]

        except Exception as e:
            print(f"[lead] Decomposition failed: {e}, using fallback")
            return [
                SubQuery(
                    id="sq-1",
                    query=f"{query} overview",
                    focus_area="overview",
                ),
            ]


# =============================================================================
# Agent Spawning
# =============================================================================


async def spawn_researchers(ctx, sub_queries: List[SubQuery]) -> List[str]:
    """Spawn researcher agents for each sub-query.

    Args:
        ctx: Agent context
        sub_queries: List of sub-queries to research

    Returns:
        List of spawned researcher agent IDs
    """
    researcher_ids = []

    for sq in sub_queries:
        with tracer.start_as_current_span(
            "lead.spawn_researcher",
            attributes={"sub_query_id": sq.id, "focus_area": sq.focus_area},
        ):
            # TODO: Implement actual agent spawning via ctx.spawn_agent()
            # For now, we'll emit events to a shared researcher topic
            researcher_id = f"researcher-{sq.id}-{int(time.time() * 1000)}"

            print(f"[lead] Spawning researcher {researcher_id} for: {sq.query[:50]}...")

            # Emit research request
            await ctx.emit(
                f"research.request.{researcher_id}",
                type="research.request",
                payload=json.dumps({
                    "researcher_id": researcher_id,
                    "sub_query": {
                        "id": sq.id,
                        "query": sq.query,
                        "focus_area": sq.focus_area,
                    },
                    "config": {
                        "max_sources": 5,
                        "timeout_sec": 30,
                    },
                }).encode("utf-8"),
            )

            researcher_ids.append(researcher_id)

    return researcher_ids


# =============================================================================
# Report Aggregation
# =============================================================================


async def aggregate_report(
    ctx,
    query: str,
    sections: List[ResearchSection],
) -> str:
    """Aggregate research sections into a final report.

    Args:
        ctx: Agent context
        query: Original user query
        sections: Completed research sections

    Returns:
        Final report as Markdown string
    """
    with tracer.start_as_current_span(
        "lead.aggregate_report",
        attributes={"section_count": len(sections)},
    ):
        # Build report structure
        timestamp = datetime.now().strftime("%Y-%m-%d %H:%M:%S")

        report_parts = [
            f"# Research Report: {query}",
            f"\n_Generated: {timestamp}_\n",
            "## Executive Summary\n",
        ]

        # Add sections
        for i, section in enumerate(sections, 1):
            report_parts.append(f"## {i}. {section.title}\n")
            report_parts.append(section.content)
            report_parts.append("")

        # Add sources
        report_parts.append("\n## Sources\n")
        source_idx = 1
        for section in sections:
            for source in section.sources:
                title = source.get("title", "Untitled")
                url = source.get("url", "#")
                report_parts.append(f"[{source_idx}] [{title}]({url})")
                source_idx += 1

        report = "\n".join(report_parts)

        # Use LLM to write executive summary
        if llm_provider and sections:
            try:
                summary_prompt = f"""Based on the following research sections, write a brief executive summary (2-3 paragraphs):

{report}

Write only the executive summary, no headers."""

                summary = await llm_provider.generate(
                    prompt=summary_prompt,
                    system="You are a research report writer. Be concise and informative.",
                    temperature=0.5,
                    max_tokens=500,
                )

                # Insert summary after the header
                report = report.replace(
                    "## Executive Summary\n",
                    f"## Executive Summary\n\n{summary.strip()}\n",
                )

            except Exception as e:
                print(f"[lead] Summary generation failed: {e}")

        return report


async def save_report(report: str, query: str) -> str:
    """Save report to workspace.

    Args:
        report: Report content
        query: Original query (for filename)

    Returns:
        Path to saved report
    """
    # Generate filename
    slug = query.lower()[:30].replace(" ", "_").replace("?", "")
    timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
    filename = f"{slug}_{timestamp}.md"

    # Ensure directory exists
    reports_dir = os.path.join(os.path.dirname(__file__), "..", "workspace", "reports")
    os.makedirs(reports_dir, exist_ok=True)

    filepath = os.path.join(reports_dir, filename)

    with open(filepath, "w") as f:
        f.write(report)

    # Also update latest.md symlink
    latest_path = os.path.join(reports_dir, "latest.md")
    if os.path.exists(latest_path):
        os.remove(latest_path)
    os.symlink(filename, latest_path)

    print(f"[lead] Report saved: {filepath}")
    return filepath


# =============================================================================
# Event Handlers
# =============================================================================


async def handle_user_query(ctx, query: str) -> None:
    """Handle incoming user query.

    Args:
        ctx: Agent context
        query: User's research query
    """
    global research_state

    with tracer.start_as_current_span(
        "lead.handle_user_query",
        attributes={"query": query},
    ):
        print(f"\n[lead] ═══════════════════════════════════════════════════")
        print(f"[lead] Received query: {query}")
        print(f"[lead] ═══════════════════════════════════════════════════\n")

        # Initialize research state
        research_state = ResearchState(
            original_query=query,
            start_time=time.time(),
        )

        # Step 1: Decompose query
        print("[lead] Step 1: Decomposing query...")
        sub_queries = await decompose_query(ctx, query)
        research_state.sub_queries = sub_queries

        for sq in sub_queries:
            print(f"[lead]   • [{sq.focus_area}] {sq.query}")

        # Step 2: Spawn researchers
        print(f"\n[lead] Step 2: Spawning {len(sub_queries)} researchers...")
        researcher_ids = await spawn_researchers(ctx, sub_queries)
        research_state.pending_researchers = researcher_ids

        print(f"[lead] Waiting for researchers to complete...")


async def handle_researcher_report(ctx, report_data: Dict[str, Any]) -> None:
    """Handle completed research from a researcher.

    Args:
        ctx: Agent context
        report_data: Research report from a researcher
    """
    global research_state

    if not research_state:
        print("[lead] WARNING: Received report but no research in progress")
        return

    with tracer.start_as_current_span(
        "lead.handle_researcher_report",
        attributes={"researcher_id": report_data.get("researcher_id", "unknown")},
    ):
        researcher_id = report_data.get("researcher_id", "unknown")
        section = ResearchSection(
            sub_query_id=report_data.get("sub_query_id", ""),
            title=report_data.get("title", "Research Section"),
            content=report_data.get("content", ""),
            sources=report_data.get("sources", []),
            timestamp_ms=int(time.time() * 1000),
        )

        research_state.completed_sections.append(section)

        if researcher_id in research_state.pending_researchers:
            research_state.pending_researchers.remove(researcher_id)

        completed = len(research_state.completed_sections)
        total = len(research_state.sub_queries)
        print(f"[lead] ✓ Received section: {section.title} ({completed}/{total})")

        # Check if all researchers have reported
        if not research_state.pending_researchers:
            await finalize_research(ctx)


async def finalize_research(ctx) -> None:
    """Finalize research and generate report.

    Args:
        ctx: Agent context
    """
    global research_state

    if not research_state:
        return

    with tracer.start_as_current_span("lead.finalize_research"):
        elapsed = time.time() - research_state.start_time
        print(f"\n[lead] ═══════════════════════════════════════════════════")
        print(f"[lead] All researchers completed in {elapsed:.1f}s")
        print(f"[lead] Aggregating {len(research_state.completed_sections)} sections...")
        print(f"[lead] ═══════════════════════════════════════════════════\n")

        # Aggregate into final report
        report = await aggregate_report(
            ctx,
            research_state.original_query,
            research_state.completed_sections,
        )

        # Save report
        filepath = await save_report(report, research_state.original_query)

        # Emit completion event
        await ctx.emit(
            "research.complete",
            type="research.complete",
            payload=json.dumps({
                "query": research_state.original_query,
                "report_path": filepath,
                "section_count": len(research_state.completed_sections),
                "elapsed_sec": elapsed,
            }).encode("utf-8"),
        )

        print(f"\n[lead] ✅ Research complete!")
        print(f"[lead] Report: {filepath}")

        # Reset state for next query
        research_state = None


# =============================================================================
# Main Event Handler
# =============================================================================


async def lead_handler(ctx, topic: str, event) -> None:
    """Main event handler for Lead Agent."""
    data = json.loads(event.payload.decode("utf-8"))

    if topic == "user.query" or event.type == "user.query":
        query = data.get("query", "")
        if query:
            await handle_user_query(ctx, query)

    elif topic.startswith("researcher.report") or event.type == "researcher.report":
        await handle_researcher_report(ctx, data)


# =============================================================================
# Main
# =============================================================================


async def main():
    global llm_provider

    config = load_project_config()
    agent_config = config.agents.get("lead-agent", {})

    topics = agent_config.get("topics", ["user.query", "researcher.report"])
    llm_name = agent_config.get("llm_provider", "deepseek")

    agent = Agent(
        agent_id="lead-agent",
        topics=topics,
        on_event=lead_handler,
    )

    print("[lead] ═══════════════════════════════════════════════════")
    print("[lead] DeepResearch Lead Agent starting")
    print("[lead] ═══════════════════════════════════════════════════")
    print(f"[lead] Subscribed to: {topics}")
    print(f"[lead] LLM Provider: {llm_name}")

    await agent.start()

    # Initialize LLM provider
    try:
        llm_provider = LLMProvider.from_config(agent._ctx, llm_name, config)
        print(f"[lead] LLM initialized: {llm_provider.config.model}")
    except Exception as e:
        print(f"[lead] LLM init failed: {e} (will use fallback logic)")

    print("[lead] Ready! Send queries to 'user.query' topic")
    print("[lead] ═══════════════════════════════════════════════════\n")

    # Keep running
    try:
        await asyncio.Event().wait()
    except KeyboardInterrupt:
        print("\n[lead] Shutting down...")
        await agent.stop()


if __name__ == "__main__":
    asyncio.run(main())
