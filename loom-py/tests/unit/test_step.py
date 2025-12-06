"""Tests for Step types and StepReducer."""

from loom.context.engineering.reducer import (
    DefaultReducer,
    FileEditReducer,
    FileReadReducer,
    FileWriteReducer,
    SearchReducer,
    ShellReducer,
    StepReducer,
    WebFetchReducer,
)
from loom.context.engineering.step import (
    CompactStep,
    Step,
    compute_content_hash,
    generate_step_id,
)


class TestStepAttributes:
    """Test Step object attributes to prevent attribute errors."""

    def test_step_has_observation_not_outcome(self):
        """Verify Step uses 'observation' attribute, not 'outcome'."""
        step = Step(
            id="step_001",
            tool_name="test:tool",
            minimal_args={"key": "value"},
            observation="Test observation",
            success=True,
        )

        # Should have observation
        assert hasattr(step, "observation")
        assert step.observation == "Test observation"

        # Should NOT have outcome
        assert not hasattr(step, "outcome")

    def test_step_outcome_ref_for_offloaded_data(self):
        """Test outcome_ref attribute for offloaded data references."""
        step = Step(
            id="step_001",
            tool_name="web:search",
            minimal_args={"query": "test"},
            observation="Search completed with 5 results",
            success=True,
            outcome_ref=".loom/cache/search/result.json",
        )

        assert step.outcome_ref == ".loom/cache/search/result.json"
        assert step.observation == "Search completed with 5 results"

    def test_step_all_required_attributes(self):
        """Verify all expected Step attributes exist."""
        step = Step(
            id="step_001",
            tool_name="test:tool",
            minimal_args={},
            observation="test",
            success=True,
        )

        # Required attributes
        assert hasattr(step, "id")
        assert hasattr(step, "tool_name")
        assert hasattr(step, "minimal_args")
        assert hasattr(step, "observation")
        assert hasattr(step, "success")
        assert hasattr(step, "timestamp_ms")

        # Optional attributes
        assert hasattr(step, "outcome_ref")
        assert hasattr(step, "error")
        assert hasattr(step, "metadata")


class TestStep:
    """Tests for Step dataclass."""

    def test_step_creation(self):
        """Test basic Step creation."""
        step = Step(
            id="step_001",
            tool_name="fs:read_file",
            minimal_args={"path": "/tmp/test.txt"},
            observation="Read test.txt (50 lines, 1.2KB)",
            success=True,
        )
        assert step.id == "step_001"
        assert step.tool_name == "fs:read_file"
        assert step.success is True
        assert step.error is None
        assert step.outcome_ref is None

    def test_step_with_error(self):
        """Test Step with error state."""
        step = Step(
            id="step_002",
            tool_name="fs:write_file",
            minimal_args={"path": "/root/forbidden.txt"},
            observation="Failed to write: Permission denied",
            success=False,
            error="Permission denied",
        )
        assert step.success is False
        assert step.error == "Permission denied"

    def test_step_to_compact(self):
        """Test converting Step to CompactStep."""
        step = Step(
            id="step_003",
            tool_name="shell:run",
            minimal_args={"command": "ls -la"},
            observation="Ran `ls -la` → 15 lines output",
            success=True,
        )
        compact = step.to_compact()
        assert isinstance(compact, CompactStep)
        assert compact.id == "step_003"
        assert compact.summary == "Ran `ls -la` → 15 lines output"

    def test_step_serialization(self):
        """Test Step to_dict and from_dict."""
        step = Step(
            id="step_004",
            tool_name="web:fetch",
            minimal_args={"url": "https://example.com"},
            observation="Fetched example.com (4.5KB)",
            success=True,
            metadata={"size": 4500},
        )
        data = step.to_dict()
        restored = Step.from_dict(data)

        assert restored.id == step.id
        assert restored.tool_name == step.tool_name
        assert restored.minimal_args == step.minimal_args
        assert restored.observation == step.observation
        assert restored.success == step.success
        assert restored.metadata == step.metadata


class TestCompactStep:
    """Tests for CompactStep dataclass."""

    def test_compact_step_str(self):
        """Test CompactStep string representation."""
        compact = CompactStep(id="step_001", summary="Read config.json (25 lines)")
        assert str(compact) == "• Read config.json (25 lines)"


class TestGenerateStepId:
    """Tests for generate_step_id function."""

    def test_generate_step_id(self):
        """Test step ID generation."""
        assert generate_step_id(1) == "step_001"
        assert generate_step_id(10) == "step_010"
        assert generate_step_id(100) == "step_100"
        assert generate_step_id(999) == "step_999"


class TestComputeContentHash:
    """Tests for compute_content_hash function."""

    def test_hash_consistency(self):
        """Test that same content gives same hash."""
        content = "Hello, World!"
        hash1 = compute_content_hash(content)
        hash2 = compute_content_hash(content)
        assert hash1 == hash2
        assert len(hash1) == 16

    def test_hash_difference(self):
        """Test that different content gives different hash."""
        hash1 = compute_content_hash("Hello")
        hash2 = compute_content_hash("World")
        assert hash1 != hash2


class TestFileReadReducer:
    """Tests for FileReadReducer."""

    def test_successful_read(self):
        """Test reducing a successful file read."""
        reducer = FileReadReducer()
        content = "line1\nline2\nline3\n" * 100  # ~1.8KB

        step = reducer.reduce(
            step_id="step_001",
            tool_name="fs:read_file",
            args={"path": "/home/user/project/config.json"},
            result=content,
            success=True,
        )

        assert step.id == "step_001"
        assert step.tool_name == "fs:read_file"
        assert step.minimal_args == {"path": "/home/user/project/config.json"}
        assert "config.json" in step.observation
        assert "lines" in step.observation
        assert step.success is True
        assert step.metadata["lines"] == 301

    def test_failed_read(self):
        """Test reducing a failed file read."""
        reducer = FileReadReducer()

        step = reducer.reduce(
            step_id="step_002",
            tool_name="fs:read_file",
            args={"path": "/nonexistent/file.txt"},
            result=None,
            success=False,
            error="File not found",
        )

        assert step.success is False
        assert "Failed to read" in step.observation
        assert step.error == "File not found"


class TestFileWriteReducer:
    """Tests for FileWriteReducer."""

    def test_successful_write(self):
        """Test reducing a successful file write."""
        reducer = FileWriteReducer()
        content = "def hello():\n    print('Hello!')\n"

        step = reducer.reduce(
            step_id="step_001",
            tool_name="fs:write_file",
            args={"path": "/tmp/hello.py", "content": content},
            result="OK",
            success=True,
        )

        assert "Wrote hello.py" in step.observation
        assert step.success is True
        assert step.metadata["lines"] == 3  # 2 newlines + 1 = 3 lines


class TestFileEditReducer:
    """Tests for FileEditReducer."""

    def test_edit_with_additions(self):
        """Test reducing an edit that adds lines."""
        reducer = FileEditReducer()

        step = reducer.reduce(
            step_id="step_001",
            tool_name="fs:edit_file",
            args={
                "path": "/tmp/test.py",
                "old_content": "# old",
                "new_content": "# new\n# added\n# more",
            },
            result="OK",
            success=True,
        )

        assert "Edited test.py" in step.observation
        assert "+2 lines" in step.observation

    def test_edit_with_deletions(self):
        """Test reducing an edit that removes lines."""
        reducer = FileEditReducer()

        step = reducer.reduce(
            step_id="step_001",
            tool_name="fs:edit_file",
            args={
                "path": "/tmp/test.py",
                "old_content": "# line1\n# line2\n# line3",
                "new_content": "# single",
            },
            result="OK",
            success=True,
        )

        assert "-2 lines" in step.observation


class TestShellReducer:
    """Tests for ShellReducer."""

    def test_short_output(self):
        """Test reducing command with short output."""
        reducer = ShellReducer()

        step = reducer.reduce(
            step_id="step_001",
            tool_name="shell:run",
            args={"command": "echo hello"},
            result="hello\n",
            success=True,
        )

        assert "Ran `echo hello`" in step.observation
        assert "hello" in step.observation

    def test_long_output(self):
        """Test reducing command with long output."""
        reducer = ShellReducer()
        output = "\n".join([f"line {i}" for i in range(100)])

        step = reducer.reduce(
            step_id="step_001",
            tool_name="shell:run",
            args={"command": "find . -name '*.py'"},
            result=output,
            success=True,
        )

        assert "100 lines output" in step.observation

    def test_failed_command(self):
        """Test reducing a failed command."""
        reducer = ShellReducer()

        step = reducer.reduce(
            step_id="step_001",
            tool_name="shell:run",
            args={"command": "invalid_command", "exit_code": 127},
            result="",
            success=False,
            error="command not found",
        )

        assert step.success is False
        assert "failed" in step.observation.lower()


class TestSearchReducer:
    """Tests for SearchReducer."""

    def test_search_with_results(self):
        """Test reducing a search with matches."""
        reducer = SearchReducer()
        results = [
            {"file": "a.py", "line": 10},
            {"file": "b.py", "line": 20},
            {"file": "c.py", "line": 30},
        ]

        step = reducer.reduce(
            step_id="step_001",
            tool_name="fs:search",
            args={"query": "TODO", "path": "/project"},
            result=results,
            success=True,
        )

        assert "Search 'TODO'" in step.observation
        assert "3 matches" in step.observation

    def test_search_no_results(self):
        """Test reducing a search with no matches."""
        reducer = SearchReducer()

        step = reducer.reduce(
            step_id="step_001",
            tool_name="fs:search",
            args={"query": "FIXME", "path": "/project"},
            result=[],
            success=True,
        )

        assert "0 matches" in step.observation


class TestWebFetchReducer:
    """Tests for WebFetchReducer."""

    def test_successful_fetch(self):
        """Test reducing a successful web fetch."""
        reducer = WebFetchReducer()
        html = "<html>" + "content" * 1000 + "</html>"

        step = reducer.reduce(
            step_id="step_001",
            tool_name="web:fetch",
            args={"url": "https://docs.python.org/3/library/json.html"},
            result=html,
            success=True,
        )

        assert "docs.python.org" in step.observation
        assert "KB" in step.observation or "B" in step.observation

    def test_failed_fetch(self):
        """Test reducing a failed web fetch."""
        reducer = WebFetchReducer()

        step = reducer.reduce(
            step_id="step_001",
            tool_name="web:fetch",
            args={"url": "https://invalid.example.com"},
            result=None,
            success=False,
            error="Connection timeout",
        )

        assert step.success is False
        assert "Failed to fetch" in step.observation


class TestDefaultReducer:
    """Tests for DefaultReducer (fallback)."""

    def test_unknown_tool(self):
        """Test reducing an unknown tool."""
        reducer = DefaultReducer()

        step = reducer.reduce(
            step_id="step_001",
            tool_name="custom:my_tool",
            args={"param1": "value1", "param2": 42},
            result="some result",
            success=True,
        )

        assert "custom:my_tool" in step.observation
        assert step.success is True

    def test_large_result_truncated(self):
        """Test that large results are truncated."""
        reducer = DefaultReducer()
        large_result = "x" * 1000

        step = reducer.reduce(
            step_id="step_001",
            tool_name="custom:tool",
            args={},
            result=large_result,
            success=True,
        )

        # Should be truncated with ellipsis
        assert len(step.observation) < 300


class TestStepReducer:
    """Tests for the main StepReducer class."""

    def test_auto_dispatch(self):
        """Test automatic dispatch to correct reducer."""
        reducer = StepReducer()

        # File read
        step1 = reducer.reduce(
            tool_name="fs:read_file",
            args={"path": "/tmp/test.txt"},
            result="content",
            success=True,
        )
        assert "Read" in step1.observation

        # Shell
        step2 = reducer.reduce(
            tool_name="shell:run",
            args={"command": "ls"},
            result="output",
            success=True,
        )
        assert "Ran" in step2.observation

    def test_auto_increment_id(self):
        """Test automatic step ID generation."""
        reducer = StepReducer()

        step1 = reducer.reduce(
            tool_name="fs:read_file",
            args={"path": "/a"},
            result="",
            success=True,
        )
        step2 = reducer.reduce(
            tool_name="fs:read_file",
            args={"path": "/b"},
            result="",
            success=True,
        )

        assert step1.id == "step_001"
        assert step2.id == "step_002"

    def test_reset_counter(self):
        """Test counter reset."""
        reducer = StepReducer()

        reducer.reduce(tool_name="test", args={}, result="", success=True)
        reducer.reduce(tool_name="test", args={}, result="", success=True)
        reducer.reset_counter()

        step = reducer.reduce(tool_name="test", args={}, result="", success=True)
        assert step.id == "step_001"

    def test_custom_step_id(self):
        """Test providing custom step ID."""
        reducer = StepReducer()

        step = reducer.reduce(
            tool_name="test",
            args={},
            result="",
            success=True,
            step_id="custom_123",
        )
        assert step.id == "custom_123"

    def test_register_custom_reducer(self):
        """Test registering a custom reducer."""
        reducer = StepReducer()

        class MyReducer(DefaultReducer):
            def reduce(self, step_id, tool_name, args, result, success, error=None):
                return Step(
                    id=step_id,
                    tool_name=tool_name,
                    minimal_args={},
                    observation="CUSTOM OBSERVATION",
                    success=success,
                )

        reducer.register("my:special_tool", MyReducer())

        step = reducer.reduce(
            tool_name="my:special_tool",
            args={"x": 1},
            result="result",
            success=True,
        )

        assert step.observation == "CUSTOM OBSERVATION"

    def test_namespace_fallback(self):
        """Test fallback to short name without namespace."""
        reducer = StepReducer()

        # "custom:read_file" should fall back to "read_file" reducer
        step = reducer.reduce(
            tool_name="custom:read_file",
            args={"path": "/tmp/test.txt"},
            result="content",
            success=True,
        )

        # Should use FileReadReducer via fallback
        assert "Read" in step.observation


class TestIntegration:
    """Integration tests for Step + Reducer."""

    def test_full_workflow(self):
        """Test complete workflow: reduce → compact → serialize."""
        reducer = StepReducer()

        # Simulate a file read
        step = reducer.reduce(
            tool_name="fs:read_file",
            args={"path": "/home/user/project/main.py"},
            result="def main():\n    pass\n",
            success=True,
        )

        # Convert to compact
        compact = step.to_compact()
        assert compact.id == step.id
        assert "main.py" in compact.summary

        # Serialize and restore
        data = step.to_dict()
        restored = Step.from_dict(data)
        assert restored.observation == step.observation

    def test_multiple_steps_compaction(self):
        """Test compacting multiple steps."""
        reducer = StepReducer()
        steps = []

        # Simulate multiple operations
        steps.append(
            reducer.reduce(
                tool_name="fs:read_file",
                args={"path": "/config.json"},
                result='{"key": "value"}',
                success=True,
            )
        )
        steps.append(
            reducer.reduce(
                tool_name="shell:run",
                args={"command": "npm install"},
                result="installed 100 packages",
                success=True,
            )
        )
        steps.append(
            reducer.reduce(
                tool_name="fs:write_file",
                args={"path": "/output.txt", "content": "result"},
                result="OK",
                success=True,
            )
        )

        # Compact all
        compact_steps = [s.to_compact() for s in steps]

        # Build summary string (like would appear in prompt)
        summary = "\n".join(str(c) for c in compact_steps)
        assert "config.json" in summary
        assert "npm install" in summary
        assert "output.txt" in summary

        # Total should be much shorter than raw content
        assert len(summary) < 200
