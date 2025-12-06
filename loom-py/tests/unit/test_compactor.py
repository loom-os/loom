"""Tests for StepCompactor."""

from loom.context.engineering.compactor import (
    CompactedHistory,
    CompactionConfig,
    StepCompactor,
)
from loom.context.engineering.step import Step


def make_step(id: str, tool: str, success: bool = True) -> Step:
    """Helper to create test steps."""
    return Step(
        id=id,
        tool_name=tool,
        minimal_args={},
        observation=f"{tool} completed" if success else f"{tool} failed",
        success=success,
    )


class TestCompactionConfig:
    """Tests for CompactionConfig."""

    def test_default_values(self):
        """Test default configuration values."""
        config = CompactionConfig()
        assert config.recent_window == 5
        assert config.max_compact_steps == 20
        assert config.group_similar is True
        assert config.preserve_failures is True

    def test_custom_values(self):
        """Test custom configuration."""
        config = CompactionConfig(
            recent_window=10,
            max_compact_steps=50,
            group_similar=False,
        )
        assert config.recent_window == 10
        assert config.max_compact_steps == 50
        assert config.group_similar is False


class TestCompactedHistory:
    """Tests for CompactedHistory."""

    def test_format_empty(self):
        """Test formatting empty history."""
        history = CompactedHistory(
            recent_steps=[],
            compact_steps=[],
            total_original=0,
        )
        result = history.format_for_prompt()
        assert result == ""

    def test_format_recent_only(self):
        """Test formatting with only recent steps."""
        steps = [
            make_step("step_001", "fs:read_file"),
            make_step("step_002", "shell:run"),
        ]
        history = CompactedHistory(
            recent_steps=steps,
            compact_steps=[],
            total_original=2,
        )
        result = history.format_for_prompt()

        assert "Recent actions:" in result
        assert "step_001" in result
        assert "step_002" in result
        assert "✓" in result

    def test_format_with_failures(self):
        """Test formatting shows failure markers."""
        steps = [
            make_step("step_001", "fs:read_file", success=True),
            make_step("step_002", "shell:run", success=False),
        ]
        history = CompactedHistory(
            recent_steps=steps,
            compact_steps=[],
            total_original=2,
        )
        result = history.format_for_prompt()

        assert "✓" in result
        assert "✗" in result

    def test_format_with_compact_and_recent(self):
        """Test formatting with both compact and recent steps."""
        from loom.context.engineering.step import CompactStep

        history = CompactedHistory(
            recent_steps=[make_step("step_010", "fs:write_file")],
            compact_steps=[
                CompactStep("step_001..005", "5 file operations"),
                CompactStep("step_006..009", "4 shell commands"),
            ],
            dropped_count=5,
            total_original=15,
        )
        result = history.format_for_prompt()

        assert "Previous actions (summarized):" in result
        assert "5 file operations" in result
        assert "4 shell commands" in result
        assert "5 earlier steps omitted" in result
        assert "Recent actions:" in result
        assert "step_010" in result


class TestStepCompactor:
    """Tests for StepCompactor."""

    def test_no_compaction_needed(self):
        """Test when steps fit in recent window."""
        compactor = StepCompactor(CompactionConfig(recent_window=5))
        steps = [make_step(f"step_{i:03d}", "fs:read_file") for i in range(3)]

        history = compactor.compact(steps)

        assert len(history.recent_steps) == 3
        assert len(history.compact_steps) == 0
        assert history.dropped_count == 0

    def test_compaction_splits_recent(self):
        """Test that recent steps are kept full."""
        compactor = StepCompactor(CompactionConfig(recent_window=3))
        steps = [make_step(f"step_{i:03d}", "fs:read_file") for i in range(10)]

        history = compactor.compact(steps)

        # Last 3 should be in recent
        assert len(history.recent_steps) == 3
        assert history.recent_steps[0].id == "step_007"
        assert history.recent_steps[-1].id == "step_009"

        # First 7 should be compacted
        assert len(history.compact_steps) > 0

    def test_simple_compaction(self):
        """Test simple compaction without grouping."""
        config = CompactionConfig(
            recent_window=2,
            max_compact_steps=5,
            group_similar=False,
        )
        compactor = StepCompactor(config)

        steps = [make_step(f"step_{i:03d}", "test_tool") for i in range(10)]
        history = compactor.compact(steps)

        # 2 recent, up to 5 compact
        assert len(history.recent_steps) == 2
        assert len(history.compact_steps) <= 5

    def test_grouping_file_operations(self):
        """Test that file operations are grouped."""
        config = CompactionConfig(
            recent_window=2,
            group_similar=True,
        )
        compactor = StepCompactor(config)

        steps = [
            make_step("step_001", "fs:read_file"),
            make_step("step_002", "fs:read_file"),
            make_step("step_003", "fs:write_file"),
            make_step("step_004", "shell:run"),
            make_step("step_005", "shell:run"),
            make_step("step_006", "fs:read_file"),  # recent
            make_step("step_007", "fs:read_file"),  # recent
        ]

        history = compactor.compact(steps)

        # Recent should have last 2
        assert len(history.recent_steps) == 2

        # Compact should have grouped entries
        compact_summaries = [cs.summary for cs in history.compact_steps]
        # Should have grouped file ops and shell ops
        assert any("file operations" in s for s in compact_summaries)

    def test_empty_steps(self):
        """Test compaction of empty list."""
        compactor = StepCompactor()
        history = compactor.compact([])

        assert history.recent_steps == []
        assert history.compact_steps == []
        assert history.total_original == 0

    def test_max_steps_override(self):
        """Test overriding max steps."""
        compactor = StepCompactor(CompactionConfig(recent_window=2, max_compact_steps=10))

        steps = [make_step(f"step_{i:03d}", "test") for i in range(20)]

        # With default config (2 + 10 = 12 max)
        history = compactor.compact(steps)
        total_kept = len(history.recent_steps) + len(history.compact_steps)
        assert total_kept <= 12

        # With override to only 5 total
        history2 = compactor.compact(steps, max_steps=5)
        total_kept2 = len(history2.recent_steps) + len(history2.compact_steps)
        assert total_kept2 <= 5

    def test_preserves_chronological_order(self):
        """Test that steps remain in chronological order."""
        compactor = StepCompactor(CompactionConfig(recent_window=3))

        steps = [make_step(f"step_{i:03d}", "test") for i in range(10)]
        history = compactor.compact(steps)

        # Recent should be in order
        ids = [s.id for s in history.recent_steps]
        assert ids == ["step_007", "step_008", "step_009"]


class TestToolCategoryGrouping:
    """Tests for tool category detection."""

    def test_file_category(self):
        """Test file operations are categorized together."""
        compactor = StepCompactor()

        assert compactor._get_tool_category("fs:read_file") == "file"
        assert compactor._get_tool_category("fs:write_file") == "file"
        assert compactor._get_tool_category("fs:edit") == "file"
        assert compactor._get_tool_category("read_file") == "file"

    def test_shell_category(self):
        """Test shell operations are categorized together."""
        compactor = StepCompactor()

        assert compactor._get_tool_category("shell:run") == "shell"
        assert compactor._get_tool_category("shell:exec") == "shell"
        assert compactor._get_tool_category("run_command") == "shell"

    def test_search_category(self):
        """Test search operations are categorized together."""
        compactor = StepCompactor()

        # Note: fs:search matches 'fs:' first, so it's categorized as file
        # Pure search tools are categorized as search
        assert compactor._get_tool_category("grep") == "search"
        assert compactor._get_tool_category("search") == "search"
        assert compactor._get_tool_category("find") == "search"

    def test_unknown_category(self):
        """Test unknown tools use their name as category."""
        compactor = StepCompactor()

        assert compactor._get_tool_category("custom:my_tool") == "custom:my_tool"


class TestGroupSummarization:
    """Tests for group summarization."""

    def test_summarize_single_step(self):
        """Test single step group still gets a summary."""
        compactor = StepCompactor()
        step = make_step("step_001", "fs:read_file")

        result = compactor._summarize_group([step])
        # Even single steps get grouped format when passed through _summarize_group
        assert "1 file operations" in result.summary or step.observation in result.summary

    def test_summarize_file_group(self):
        """Test file operation group summary."""
        compactor = StepCompactor()
        steps = [
            make_step("step_001", "fs:read_file"),
            make_step("step_002", "fs:write_file"),
            make_step("step_003", "fs:edit"),
        ]

        result = compactor._summarize_group(steps)
        assert "3 file operations" in result.summary
        assert result.id == "step_001..step_003"

    def test_summarize_with_failures(self):
        """Test group summary includes failure count."""
        compactor = StepCompactor()
        steps = [
            make_step("step_001", "shell:run", success=True),
            make_step("step_002", "shell:run", success=False),
            make_step("step_003", "shell:run", success=True),
        ]

        result = compactor._summarize_group(steps)
        assert "3 commands executed" in result.summary
        assert "1 failed" in result.summary
