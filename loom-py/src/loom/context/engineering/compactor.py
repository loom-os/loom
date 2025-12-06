"""Step Compactor for Context Engineering.

This module implements step compaction strategies that convert old Steps
into ultra-minimal CompactSteps when context budget is tight.

Compaction Philosophy:
1. Recent steps keep full observation
2. Older steps get progressively compressed
3. Failed steps always retain error info
4. Successful patterns can be grouped
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Optional

from .step import CompactStep, Step


@dataclass
class CompactionConfig:
    """Configuration for step compaction.

    Attributes:
        recent_window: Number of recent steps to keep full (default: 5)
        max_compact_steps: Maximum compact steps to keep (default: 20)
        group_similar: Group similar consecutive operations (default: True)
        preserve_failures: Always keep failed steps visible (default: True)
    """

    recent_window: int = 5
    max_compact_steps: int = 20
    group_similar: bool = True
    preserve_failures: bool = True


@dataclass
class CompactedHistory:
    """Result of compaction operation.

    Attributes:
        recent_steps: Full Steps from recent window
        compact_steps: Compacted older steps
        dropped_count: Number of steps dropped entirely
        total_original: Total steps before compaction
    """

    recent_steps: list[Step]
    compact_steps: list[CompactStep]
    dropped_count: int = 0
    total_original: int = 0

    def format_for_prompt(self) -> str:
        """Format compacted history for inclusion in prompt.

        Returns:
            Formatted string for LLM context
        """
        lines = []

        # Add compact history if any
        if self.compact_steps:
            lines.append("Previous actions (summarized):")
            for cs in self.compact_steps:
                lines.append(str(cs))
            if self.dropped_count > 0:
                lines.append(f"  ... ({self.dropped_count} earlier steps omitted)")
            lines.append("")

        # Add recent full steps
        if self.recent_steps:
            lines.append("Recent actions:")
            for step in self.recent_steps:
                status = "✓" if step.success else "✗"
                lines.append(f"[{step.id}] {status} {step.observation}")
            lines.append("")

        return "\n".join(lines)


class StepCompactor:
    """Compacts step history to fit context budget.

    Usage:
        compactor = StepCompactor()
        history = compactor.compact(steps, max_steps=25)
        prompt_section = history.format_for_prompt()
    """

    def __init__(self, config: Optional[CompactionConfig] = None):
        """Initialize compactor with config.

        Args:
            config: Compaction configuration
        """
        self.config = config or CompactionConfig()

    def compact(
        self,
        steps: list[Step],
        max_steps: Optional[int] = None,
    ) -> CompactedHistory:
        """Compact step history.

        Args:
            steps: List of steps (oldest first)
            max_steps: Override max total steps to keep

        Returns:
            CompactedHistory with recent and compacted steps
        """
        if not steps:
            return CompactedHistory(
                recent_steps=[],
                compact_steps=[],
                total_original=0,
            )

        max_total = max_steps or (self.config.recent_window + self.config.max_compact_steps)
        total = len(steps)

        # If within budget, no compaction needed
        if total <= self.config.recent_window:
            return CompactedHistory(
                recent_steps=list(steps),
                compact_steps=[],
                total_original=total,
            )

        # Split into recent and older
        recent = steps[-self.config.recent_window :]
        older = steps[: -self.config.recent_window]

        # Compact older steps
        compact_steps = []
        dropped = 0

        if self.config.group_similar:
            compact_steps, dropped = self._group_and_compact(older, max_total)
        else:
            compact_steps, dropped = self._simple_compact(older, max_total)

        return CompactedHistory(
            recent_steps=recent,
            compact_steps=compact_steps,
            dropped_count=dropped,
            total_original=total,
        )

    def _simple_compact(self, steps: list[Step], max_total: int) -> tuple[list[CompactStep], int]:
        """Simple compaction: convert each step to CompactStep.

        Args:
            steps: Steps to compact
            max_total: Max total steps allowed

        Returns:
            (compact_steps, dropped_count)
        """
        max_compact = max_total - self.config.recent_window
        if max_compact <= 0:
            return [], len(steps)

        # Keep most recent ones, convert to compact
        if len(steps) <= max_compact:
            return [s.to_compact() for s in steps], 0

        # Need to drop some
        to_keep = steps[-max_compact:]
        dropped = len(steps) - max_compact

        return [s.to_compact() for s in to_keep], dropped

    def _group_and_compact(
        self, steps: list[Step], max_total: int
    ) -> tuple[list[CompactStep], int]:
        """Group similar consecutive steps and compact.

        Args:
            steps: Steps to compact
            max_total: Max total steps allowed

        Returns:
            (compact_steps, dropped_count)
        """
        if not steps:
            return [], 0

        max_compact = max_total - self.config.recent_window
        if max_compact <= 0:
            return [], len(steps)

        # Group consecutive same-tool operations
        groups: list[list[Step]] = []
        current_group: list[Step] = []
        current_tool: Optional[str] = None

        for step in steps:
            tool_category = self._get_tool_category(step.tool_name)

            if tool_category == current_tool:
                current_group.append(step)
            else:
                if current_group:
                    groups.append(current_group)
                current_group = [step]
                current_tool = tool_category

        if current_group:
            groups.append(current_group)

        # Compact each group
        compact_steps = []
        for group in groups:
            if len(group) == 1:
                compact_steps.append(group[0].to_compact())
            else:
                # Summarize the group
                compact_steps.append(self._summarize_group(group))

        # If still too many, drop oldest
        if len(compact_steps) > max_compact:
            dropped = len(compact_steps) - max_compact
            compact_steps = compact_steps[-max_compact:]
            return compact_steps, dropped

        return compact_steps, 0

    def _get_tool_category(self, tool_name: str) -> str:
        """Get tool category for grouping.

        Args:
            tool_name: Full tool name

        Returns:
            Category string (e.g., "file", "shell", "search")
        """
        name_lower = tool_name.lower()

        if any(x in name_lower for x in ["read", "write", "edit", "file", "fs:"]):
            return "file"
        elif any(x in name_lower for x in ["shell", "run", "exec", "command"]):
            return "shell"
        elif any(x in name_lower for x in ["search", "grep", "find"]):
            return "search"
        elif any(x in name_lower for x in ["web", "http", "fetch", "url"]):
            return "web"
        else:
            return tool_name

    def _summarize_group(self, group: list[Step]) -> CompactStep:
        """Summarize a group of similar steps.

        Args:
            group: List of similar steps

        Returns:
            Single CompactStep summarizing the group
        """
        if not group:
            return CompactStep(id="group", summary="(empty group)")

        first = group[0]
        last = group[-1]
        count = len(group)
        category = self._get_tool_category(first.tool_name)

        # Count successes/failures
        successes = sum(1 for s in group if s.success)
        failures = count - successes

        # Generate summary based on category
        if category == "file":
            summary = f"[{first.id}..{last.id}] {count} file operations"
        elif category == "shell":
            summary = f"[{first.id}..{last.id}] {count} commands executed"
        elif category == "search":
            summary = f"[{first.id}..{last.id}] {count} searches"
        else:
            summary = f"[{first.id}..{last.id}] {count}x {category}"

        if failures > 0:
            summary += f" ({failures} failed)"

        return CompactStep(id=f"{first.id}..{last.id}", summary=summary)


__all__ = [
    "StepCompactor",
    "CompactionConfig",
    "CompactedHistory",
]
