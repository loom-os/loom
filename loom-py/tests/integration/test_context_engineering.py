"""Integration tests for Context Engineering with CognitiveAgent."""


class TestPromptBuilding:
    """Test prompt building with compaction."""

    def test_prompt_uses_compaction(self):
        """Test that build_react_prompt uses compactor."""
        from loom.cognitive.loop import build_react_prompt
        from loom.cognitive.types import Observation, ThoughtStep, ToolCall
        from loom.context import Step, StepCompactor

        compactor = StepCompactor()

        # Create steps with reduced_step
        steps = []
        for i in range(8):
            reduced = Step(
                id=f"step_{i:03d}",
                tool_name="fs:read_file",
                minimal_args={"path": f"/file{i}.txt"},
                observation=f"Read file{i}.txt (100 lines, 5KB)",
                success=True,
            )

            thought_step = ThoughtStep(
                step=i + 1,
                reasoning=f"Reading file {i}",
                tool_call=ToolCall(name="fs:read_file", arguments={"path": f"/file{i}.txt"}),
                observation=Observation(
                    tool_name="fs:read_file",
                    success=True,
                    output="[large file content...]",
                ),
                reduced_step=reduced,
            )
            steps.append(thought_step)

        # Build prompt with compaction
        prompt = build_react_prompt(
            goal="Process files",
            steps=steps,
            compactor=compactor,
            use_compaction=True,
        )

        # Should contain compacted summary
        assert "Previous actions (summarized)" in prompt or "Recent steps" in prompt
        # Should mention compaction
        assert "file operations" in prompt.lower() or "omitted" in prompt.lower()

    def test_prompt_without_compaction(self):
        """Test traditional prompt format without compaction."""
        from loom.cognitive.loop import build_react_prompt
        from loom.cognitive.types import Observation, ThoughtStep, ToolCall

        steps = [
            ThoughtStep(
                step=1,
                reasoning="First step",
                tool_call=ToolCall(name="test_tool", arguments={}),
                observation=Observation(
                    tool_name="test_tool",
                    success=True,
                    output="result",
                ),
            )
        ]

        # Build without compaction
        prompt = build_react_prompt(
            goal="Test task",
            steps=steps,
            compactor=None,
            use_compaction=False,
        )

        # Should use traditional format
        assert "Previous steps:" in prompt
        assert "Thought 1:" in prompt


class TestCLIDisplay:
    """Test CLI display functions with context engineering."""

    def test_display_offloaded_step(self, capsys):
        """Test that CLI correctly displays offloaded data references."""
        from loom.cli.chat import print_stream_step_complete
        from loom.cognitive.types import Observation, ThoughtStep, ToolCall
        from loom.context import Step

        # Create a step with offloaded data
        reduced_step = Step(
            id="step_001",
            tool_name="web:search",
            minimal_args={"query": "test"},
            observation="Search completed with 5 results",
            success=True,
            outcome_ref=".loom/cache/search/websearch_123.json",
        )

        step = ThoughtStep(
            step=1,
            reasoning="Searching for information",
            tool_call=ToolCall(name="web:search", arguments={"query": "test", "limit": 5}),
            observation=Observation(
                tool_name="web:search",
                success=True,
                output='{"count": 5, "results": [...]}',
                reduced_step=reduced_step,
            ),
            reduced_step=reduced_step,
        )

        # Should not raise AttributeError
        print_stream_step_complete(step)
        captured = capsys.readouterr()

        # Verify output contains offload reference
        assert ".loom/cache/search/websearch_123.json" in captured.out
        assert "Offloaded to:" in captured.out
        assert "Summary:" in captured.out or "Summary" in captured.out
        assert "Search completed" in captured.out
        # Check for "View with:" and "cat" separately due to ANSI codes
        assert "View with:" in captured.out
        assert "cat " in captured.out

    def test_display_non_offloaded_step(self, capsys):
        """Test CLI display for normal (non-offloaded) tool output."""
        from loom.cli.chat import print_stream_step_complete
        from loom.cognitive.types import Observation, ThoughtStep, ToolCall

        step = ThoughtStep(
            step=1,
            reasoning="Getting weather",
            tool_call=ToolCall(name="weather:get", arguments={"location": "Tokyo"}),
            observation=Observation(
                tool_name="weather:get",
                success=True,
                output="Temperature: 15°C\nConditions: Sunny",
            ),
        )

        print_stream_step_complete(step)
        captured = capsys.readouterr()

        # Should show normal output
        assert "Temperature: 15°C" in captured.out
        assert "Conditions: Sunny" in captured.out
        assert "Data offloaded" not in captured.out

    def test_display_error_step(self, capsys):
        """Test CLI display for failed tool execution."""
        from loom.cli.chat import print_stream_step_complete
        from loom.cognitive.types import Observation, ThoughtStep, ToolCall
        from loom.context import Step

        reduced_step = Step(
            id="step_001",
            tool_name="fs:read_file",
            minimal_args={"path": "/nonexistent.txt"},
            observation="",
            success=False,
            error="File not found",
        )

        step = ThoughtStep(
            step=1,
            reasoning="Reading file",
            tool_call=ToolCall(name="fs:read_file", arguments={"path": "/nonexistent.txt"}),
            observation=Observation(
                tool_name="fs:read_file",
                success=False,
                output="",
                error="File not found",
                reduced_step=reduced_step,
            ),
            reduced_step=reduced_step,
        )

        print_stream_step_complete(step)
        captured = capsys.readouterr()

        assert "Error:" in captured.out
        assert "File not found" in captured.out


class TestEndToEndContextEngineering:
    """End-to-end tests for context engineering pipeline."""

    def test_full_pipeline_with_offloading(self, tmp_path):
        """Test complete flow: reducer -> offloader -> compactor -> prompt."""
        from loom.cognitive.loop import build_react_prompt
        from loom.cognitive.types import Observation, ThoughtStep, ToolCall
        from loom.context import DataOffloader, OffloadConfig, StepCompactor, StepReducer

        # Setup components
        reducer = StepReducer()
        offloader = DataOffloader(
            tmp_path,
            OffloadConfig(enabled=True, size_threshold=100, line_threshold=5),
        )
        compactor = StepCompactor()

        # Simulate tool execution with large output
        large_output = "\n".join([f"Line {i}: some data" for i in range(100)])

        # Step 1: Offload data
        offload_result = offloader.offload(
            content=large_output,
            category="test_output",
            identifier="test_001",
        )

        assert offload_result.offloaded
        assert offload_result.file_path is not None

        # Step 2: Reduce to Step
        reduced_step = reducer.reduce(
            tool_name="test:generate",
            args={"count": 100},
            result=offload_result.content,  # Preview
            success=True,
        )
        reduced_step.outcome_ref = offload_result.file_path

        # Step 3: Create ThoughtStep
        thought_step = ThoughtStep(
            step=1,
            reasoning="Generating test data",
            tool_call=ToolCall(name="test:generate", arguments={"count": 100}),
            observation=Observation(
                tool_name="test:generate",
                success=True,
                output=offload_result.content,
                reduced_step=reduced_step,
            ),
            reduced_step=reduced_step,
        )

        # Step 4: Build prompt
        steps = [thought_step]
        prompt = build_react_prompt(
            goal="Generate and process data",
            steps=steps,
            compactor=compactor,
            use_compaction=False,  # Only 1 step, no compaction needed
        )

        # With only 1 step, it uses traditional format
        # Verify observation is shown (not full output)
        assert "test:generate" in prompt
        # Should show preview (offloader already truncated it)
        assert "Line 0:" in prompt
        # Original size reduced by offloading
        assert len(prompt) < len(large_output) + 200

    def test_compaction_after_multiple_steps(self):
        """Test that compaction kicks in after threshold."""
        from loom.cognitive.loop import build_react_prompt
        from loom.cognitive.types import Observation, ThoughtStep, ToolCall
        from loom.context import Step, StepCompactor

        compactor = StepCompactor()
        steps = []

        # Create 10 steps (threshold is 5)
        for i in range(10):
            reduced = Step(
                id=f"step_{i:03d}",
                tool_name="test:action",
                minimal_args={"index": i},
                observation=f"Completed action {i}",
                success=True,
            )

            thought_step = ThoughtStep(
                step=i + 1,
                reasoning=f"Performing action {i}",
                tool_call=ToolCall(name="test:action", arguments={"index": i}),
                observation=Observation(
                    tool_name="test:action",
                    success=True,
                    output=f"Result {i}",
                    reduced_step=reduced,
                ),
                reduced_step=reduced,
            )
            steps.append(thought_step)

        # Build prompt with compaction
        prompt = build_react_prompt(
            goal="Perform multiple actions",
            steps=steps,
            compactor=compactor,
            use_compaction=True,
        )

        # Should show compaction markers
        assert "summarized" in prompt.lower() or "omitted" in prompt.lower()
        # Should have recent steps
        assert "Recent steps" in prompt
        # Last step should be visible
        assert "action 9" in prompt.lower()
