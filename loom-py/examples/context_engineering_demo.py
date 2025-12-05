"""Demo: Context Engineering in Action

This demonstrates how the context engineering system reduces token usage
while maintaining full capability for the cognitive agent.
"""

from loom.cognitive.loop import build_react_prompt
from loom.cognitive.types import Observation, ThoughtStep, ToolCall
from loom.context import StepCompactor, StepReducer


def demo_traditional_vs_compacted():
    """Compare traditional vs compacted prompt sizes."""
    print("=" * 80)
    print("DEMO: Traditional vs Context-Engineered Prompts")
    print("=" * 80)

    # Simulate a cognitive loop with many steps
    reducer = StepReducer()
    compactor = StepCompactor()

    # Create 15 steps of various operations
    operations = [
        ("fs:read_file", {"path": "/config.json"}, '{"api_key": "...", "settings": {...}}' * 50),
        ("fs:read_file", {"path": "/data.csv"}, "col1,col2,col3\n" + "data,row,values\n" * 100),
        ("shell:run", {"command": "ls -la"}, "file1.txt\nfile2.py\n" * 20),
        ("fs:search", {"query": "TODO"}, "Found 45 matches across 12 files..."),
        ("web:fetch", {"url": "https://api.example.com"}, '{"results": [...]}' * 30),
    ] * 3  # Repeat to get 15 steps

    thought_steps = []
    for i, (tool, args, output) in enumerate(operations[:15]):
        # Reduce the step
        reduced = reducer.reduce(
            tool_name=tool,
            args=args,
            result=output,
            success=True,
        )

        # Create ThoughtStep
        thought_step = ThoughtStep(
            step=i + 1,
            reasoning=f"Executing {tool} to gather information",
            tool_call=ToolCall(name=tool, arguments=args),
            observation=Observation(
                tool_name=tool,
                success=True,
                output=output,
            ),
            reduced_step=reduced,
        )
        thought_steps.append(thought_step)

    goal = "Analyze the project structure and dependencies"

    # Build traditional prompt (no compaction)
    traditional_prompt = build_react_prompt(
        goal=goal,
        steps=thought_steps,
        compactor=None,
        use_compaction=False,
    )

    # Build compacted prompt
    compacted_prompt = build_react_prompt(
        goal=goal,
        steps=thought_steps,
        compactor=compactor,
        use_compaction=True,
    )

    # Show results
    print("\n📊 Statistics:")
    print(f"  Total steps: {len(thought_steps)}")
    print(f"  Traditional prompt: {len(traditional_prompt):,} chars")
    print(f"  Compacted prompt:   {len(compacted_prompt):,} chars")
    reduction = (1 - len(compacted_prompt) / len(traditional_prompt)) * 100
    print(f"  Reduction:          {reduction:.1f}%")
    print(f"  Estimated tokens saved: ~{(len(traditional_prompt) - len(compacted_prompt)) // 4:,}")

    print("\n📝 Traditional Prompt Preview (first 500 chars):")
    print("-" * 80)
    print(traditional_prompt[:500])
    print("...")

    print("\n✨ Compacted Prompt Preview (first 800 chars):")
    print("-" * 80)
    print(compacted_prompt[:800])
    print("...")

    print("\n" + "=" * 80)
    print("Key Features Demonstrated:")
    print("  ✓ Large outputs offloaded to files")
    print("  ✓ Similar operations grouped together")
    print("  ✓ Recent steps kept in full detail")
    print("  ✓ Old steps summarized compactly")
    print("  ✓ Significant token savings (typically 50-80%)")
    print("=" * 80)


def demo_step_reduction():
    """Demo how individual steps are reduced."""
    print("\n" + "=" * 80)
    print("DEMO: Step Reduction - Before & After")
    print("=" * 80)

    reducer = StepReducer()

    examples = [
        (
            "fs:read_file",
            {"path": "/home/user/project/README.md"},
            "# Project\n\n" + "Content line\n" * 200,
        ),
        ("shell:run", {"command": "npm install"}, "added 1523 packages\n" * 100),
        (
            "web:fetch",
            {"url": "https://docs.python.org/3/library/json.html"},
            "<html>" + "x" * 5000 + "</html>",
        ),
    ]

    for tool, args, output in examples:
        print(f"\n🔧 Tool: {tool}")
        print(f"   Raw output size: {len(output):,} chars")

        step = reducer.reduce(
            tool_name=tool,
            args=args,
            result=output,
            success=True,
        )

        print(f"   Reduced to: {step.observation}")
        print(f"   Minimal args: {step.minimal_args}")
        print(f"   Savings: {len(output) - len(step.observation):,} chars")


if __name__ == "__main__":
    demo_traditional_vs_compacted()
    demo_step_reduction()
