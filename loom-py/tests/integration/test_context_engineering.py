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
