"""Researcher Agent - Web Search and Summarization

Receives sub-queries from Lead Agent, performs web searches,
analyzes results, and returns structured research sections.
"""

import asyncio
import json
import random
import time
from dataclasses import dataclass
from typing import Any, Dict, List, Optional

from opentelemetry import trace

from loom import Agent, LLMProvider, load_project_config

tracer = trace.get_tracer(__name__)


# =============================================================================
# Data Structures
# =============================================================================


@dataclass
class SearchResult:
    """A single search result."""

    title: str
    url: str
    snippet: str


@dataclass
class ResearchRequest:
    """Incoming research request from Lead."""

    researcher_id: str
    sub_query_id: str
    query: str
    focus_area: str
    max_sources: int = 5
    timeout_sec: int = 30


# Global LLM provider
llm_provider: Optional[LLMProvider] = None


# =============================================================================
# Web Search
# =============================================================================


async def web_search(ctx, query: str, max_results: int = 5) -> List[SearchResult]:
    """Perform web search using MCP tool.

    Args:
        ctx: Agent context
        query: Search query
        max_results: Maximum results to return

    Returns:
        List of SearchResult objects
    """
    with tracer.start_as_current_span(
        "researcher.web_search",
        attributes={"query": query, "max_results": max_results},
    ) as span:
        try:
            # Try MCP web-search tool
            result = await ctx.tool(
                "web-search",
                payload={"query": query, "count": max_results},
                timeout_ms=10000,
            )

            data = json.loads(result)
            results = []

            for item in data.get("results", [])[:max_results]:
                results.append(SearchResult(
                    title=item.get("title", "Untitled"),
                    url=item.get("url", "#"),
                    snippet=item.get("snippet", item.get("description", "")),
                ))

            span.set_attribute("search.result_count", len(results))
            return results

        except Exception as e:
            span.add_event("search_failed", {"error": str(e)})
            print(f"[researcher] Web search failed: {e}, using mock data")

            # Fallback to mock results for development
            return _mock_search_results(query, max_results)


def _mock_search_results(query: str, max_results: int) -> List[SearchResult]:
    """Generate mock search results for development."""
    mock_sources = [
        ("AI Agents Overview 2024", "https://example.com/ai-agents-2024",
         "Comprehensive overview of AI agent frameworks and capabilities in 2024."),
        ("LangChain vs AutoGen", "https://example.com/langchain-autogen",
         "Comparison of popular AI agent frameworks for different use cases."),
        ("Building Production AI Agents", "https://example.com/prod-agents",
         "Best practices for deploying AI agents in production environments."),
        ("Agent Memory Systems", "https://example.com/agent-memory",
         "How modern AI agents handle long-term memory and context."),
        ("Multi-Agent Collaboration", "https://example.com/multi-agent",
         "Patterns and protocols for multi-agent systems."),
        ("AI Agent Challenges", "https://example.com/agent-challenges",
         "Current limitations and challenges in AI agent development."),
        ("Future of AI Agents", "https://example.com/agent-future",
         "Predictions and roadmap for AI agent technology."),
    ]

    # Shuffle and return subset
    selected = random.sample(mock_sources, min(max_results, len(mock_sources)))
    return [
        SearchResult(title=t, url=u, snippet=s)
        for t, u, s in selected
    ]


# =============================================================================
# Content Analysis
# =============================================================================


async def analyze_and_summarize(
    ctx,
    query: str,
    focus_area: str,
    search_results: List[SearchResult],
) -> Dict[str, Any]:
    """Analyze search results and generate a summary section.

    Args:
        ctx: Agent context
        query: Original sub-query
        focus_area: Area of focus for this research
        search_results: Web search results to analyze

    Returns:
        Dictionary with title, content, and sources
    """
    with tracer.start_as_current_span(
        "researcher.analyze",
        attributes={"focus_area": focus_area, "source_count": len(search_results)},
    ):
        # Build context from search results
        search_context = "\n\n".join([
            f"**{r.title}** ({r.url})\n{r.snippet}"
            for r in search_results
        ])

        if not llm_provider:
            # Fallback without LLM
            return {
                "title": focus_area.title(),
                "content": f"Research on: {query}\n\nKey findings from {len(search_results)} sources:\n" +
                          "\n".join([f"- {r.snippet[:100]}..." for r in search_results]),
                "sources": [{"title": r.title, "url": r.url} for r in search_results],
            }

        prompt = f"""You are a research analyst. Based on the following search results, write a comprehensive section about: {focus_area}

SEARCH QUERY: {query}

SEARCH RESULTS:
{search_context}

Write a well-structured section (3-5 paragraphs) that:
1. Synthesizes key information from the sources
2. Provides specific examples and details
3. Maintains an objective, informative tone

Format the output as JSON:
{{
    "title": "Section Title",
    "content": "Full section content with multiple paragraphs...",
    "key_points": ["point 1", "point 2", "point 3"]
}}

Return only the JSON, no other text."""

        try:
            response = await llm_provider.generate(
                prompt=prompt,
                system="You are a research analyst. Return only valid JSON.",
                temperature=0.5,
                max_tokens=1000,
            )

            # Parse response
            response = response.strip()
            if response.startswith("```"):
                response = response.split("```")[1]
                if response.startswith("json"):
                    response = response[4:]
                response = response.split("```")[0]

            data = json.loads(response)

            return {
                "title": data.get("title", focus_area.title()),
                "content": data.get("content", ""),
                "key_points": data.get("key_points", []),
                "sources": [{"title": r.title, "url": r.url} for r in search_results],
            }

        except Exception as e:
            print(f"[researcher] Analysis failed: {e}")
            return {
                "title": focus_area.title(),
                "content": f"Research findings for: {query}",
                "sources": [{"title": r.title, "url": r.url} for r in search_results],
            }


# =============================================================================
# Research Execution
# =============================================================================


async def execute_research(ctx, request: ResearchRequest) -> Dict[str, Any]:
    """Execute full research pipeline for a request.

    Args:
        ctx: Agent context
        request: Research request from Lead

    Returns:
        Complete research section
    """
    with tracer.start_as_current_span(
        "researcher.execute",
        attributes={
            "researcher_id": request.researcher_id,
            "sub_query_id": request.sub_query_id,
            "focus_area": request.focus_area,
        },
    ):
        print(f"[researcher:{request.researcher_id}] Starting research...")
        print(f"[researcher:{request.researcher_id}]   Query: {request.query}")
        print(f"[researcher:{request.researcher_id}]   Focus: {request.focus_area}")

        start_time = time.time()

        # Step 1: Web search
        print(f"[researcher:{request.researcher_id}] Searching...")
        search_results = await web_search(ctx, request.query, request.max_sources)
        print(f"[researcher:{request.researcher_id}]   Found {len(search_results)} results")

        # Step 2: Analyze and summarize
        print(f"[researcher:{request.researcher_id}] Analyzing...")
        section = await analyze_and_summarize(
            ctx,
            request.query,
            request.focus_area,
            search_results,
        )

        elapsed = time.time() - start_time
        print(f"[researcher:{request.researcher_id}] ✓ Complete in {elapsed:.1f}s")

        return {
            "researcher_id": request.researcher_id,
            "sub_query_id": request.sub_query_id,
            "title": section.get("title", request.focus_area.title()),
            "content": section.get("content", ""),
            "sources": section.get("sources", []),
            "key_points": section.get("key_points", []),
            "elapsed_sec": elapsed,
        }


# =============================================================================
# Event Handler
# =============================================================================


async def researcher_handler(ctx, topic: str, event) -> None:
    """Handle research requests from Lead Agent."""
    # Parse request
    try:
        data = json.loads(event.payload.decode("utf-8"))
    except Exception as e:
        print(f"[researcher] Failed to parse event: {e}")
        return

    # Only handle research requests
    if event.type != "research.request" and not topic.startswith("research.request"):
        return

    # Extract request details
    sub_query = data.get("sub_query", {})
    config = data.get("config", {})

    request = ResearchRequest(
        researcher_id=data.get("researcher_id", f"researcher-{int(time.time())}"),
        sub_query_id=sub_query.get("id", "unknown"),
        query=sub_query.get("query", ""),
        focus_area=sub_query.get("focus_area", "general"),
        max_sources=config.get("max_sources", 5),
        timeout_sec=config.get("timeout_sec", 30),
    )

    if not request.query:
        print(f"[researcher] Received empty query, ignoring")
        return

    # Execute research
    try:
        report = await execute_research(ctx, request)

        # Send report back to Lead
        await ctx.emit(
            "researcher.report",
            type="researcher.report",
            payload=json.dumps(report).encode("utf-8"),
        )

    except asyncio.TimeoutError:
        print(f"[researcher:{request.researcher_id}] Research timed out")
        await ctx.emit(
            "researcher.report",
            type="researcher.report",
            payload=json.dumps({
                "researcher_id": request.researcher_id,
                "sub_query_id": request.sub_query_id,
                "title": request.focus_area.title(),
                "content": f"Research timed out for: {request.query}",
                "sources": [],
                "error": "timeout",
            }).encode("utf-8"),
        )

    except Exception as e:
        print(f"[researcher:{request.researcher_id}] Research failed: {e}")
        await ctx.emit(
            "researcher.report",
            type="researcher.report",
            payload=json.dumps({
                "researcher_id": request.researcher_id,
                "sub_query_id": request.sub_query_id,
                "title": request.focus_area.title(),
                "content": f"Research failed: {e}",
                "sources": [],
                "error": str(e),
            }).encode("utf-8"),
        )


# =============================================================================
# Main
# =============================================================================


async def main():
    global llm_provider

    config = load_project_config()
    agent_config = config.agents.get("researcher-agent", {})

    # Subscribe to research request wildcard
    topics = ["research.request.*"]
    llm_name = agent_config.get("llm_provider", "deepseek")

    agent = Agent(
        agent_id="researcher-agent",
        topics=topics,
        on_event=researcher_handler,
    )

    print("[researcher] ═══════════════════════════════════════════════════")
    print("[researcher] DeepResearch Researcher Agent starting")
    print("[researcher] ═══════════════════════════════════════════════════")
    print(f"[researcher] Subscribed to: {topics}")
    print(f"[researcher] LLM Provider: {llm_name}")

    await agent.start()

    # Initialize LLM provider
    try:
        llm_provider = LLMProvider.from_config(agent._ctx, llm_name, config)
        print(f"[researcher] LLM initialized: {llm_provider.config.model}")
    except Exception as e:
        print(f"[researcher] LLM init failed: {e} (will use fallback logic)")

    print("[researcher] Ready! Waiting for research requests...")
    print("[researcher] ═══════════════════════════════════════════════════\n")

    # Keep running
    try:
        await asyncio.Event().wait()
    except KeyboardInterrupt:
        print("\n[researcher] Shutting down...")
        await agent.stop()


if __name__ == "__main__":
    asyncio.run(main())
