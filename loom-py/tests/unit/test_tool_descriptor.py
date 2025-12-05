"""Tests for tool descriptor and few-shot examples."""

from loom.context import (
    FewShotExample,
    FewShotLibrary,
    ToolDescriptor,
    ToolParameter,
    ToolRegistry,
    create_default_registry,
    get_default_library,
)


class TestToolParameter:
    """Test ToolParameter functionality."""

    def test_required_parameter(self):
        """Test required parameter formatting."""
        param = ToolParameter(
            name="path",
            type="string",
            description="File path",
            required=True,
        )

        result = param.to_string()
        assert "path: string (required)" in result
        assert "File path" in result

    def test_optional_with_default(self):
        """Test optional parameter with default value."""
        param = ToolParameter(
            name="timeout",
            type="number",
            description="Timeout in seconds",
            default=30,
        )

        result = param.to_string()
        assert "timeout: number" in result
        assert "(required)" not in result
        assert "default: 30" in result

    def test_with_examples(self):
        """Test parameter with examples."""
        param = ToolParameter(
            name="query",
            type="string",
            description="Search query",
            required=True,
            examples=["TODO", "FIXME", "BUG"],
        )

        result = param.to_string()
        assert "e.g. TODO, FIXME" in result  # Only first 2 examples


class TestToolDescriptor:
    """Test ToolDescriptor functionality."""

    def test_simple_signature(self):
        """Test tool signature without parameters."""
        desc = ToolDescriptor(
            name="web:search",
            description="Search the web",
        )

        assert desc.get_signature() == "web:search()"

    def test_signature_with_params(self):
        """Test signature with required and optional params."""
        desc = ToolDescriptor(
            name="fs:read_file",
            description="Read file",
            parameters=[
                ToolParameter("path", "string", "File path", required=True),
                ToolParameter("start", "number", "Start line", required=False),
                ToolParameter("end", "number", "End line", required=False),
            ],
        )

        sig = desc.get_signature()
        assert sig == "fs:read_file(path, start?, end?)"

    def test_compact_string(self):
        """Test compact one-line format."""
        desc = ToolDescriptor(
            name="shell:run",
            description="Execute shell command",
        )

        result = desc.to_compact_string()
        assert result == "shell:run() - Execute shell command"

    def test_detailed_string(self):
        """Test detailed multi-line format."""
        desc = ToolDescriptor(
            name="fs:write_file",
            description="Write to file",
            parameters=[
                ToolParameter("path", "string", "File path", required=True),
                ToolParameter("content", "string", "Content", required=True),
            ],
            examples=[
                '{"tool": "fs:write_file", "args": {"path": "test.txt", "content": "hello"}}'
            ],
        )

        result = desc.to_detailed_string()
        assert "Tool: fs:write_file" in result
        assert "Description: Write to file" in result
        assert "Parameters:" in result
        assert "path: string (required)" in result
        assert "Examples:" in result

    def test_from_simple_name(self):
        """Test creating minimal descriptor."""
        desc = ToolDescriptor.from_simple_name("custom:tool", "Do something")

        assert desc.name == "custom:tool"
        assert desc.description == "Do something"
        assert len(desc.parameters) == 0


class TestToolRegistry:
    """Test ToolRegistry functionality."""

    def test_register_and_get(self):
        """Test basic registration and retrieval."""
        registry = ToolRegistry()
        desc = ToolDescriptor.from_simple_name("test:tool", "Test tool")

        registry.register(desc)
        retrieved = registry.get("test:tool")

        assert retrieved is not None
        assert retrieved.name == "test:tool"

    def test_register_simple(self):
        """Test simple registration."""
        registry = ToolRegistry()
        registry.register_simple("my:tool", "My tool description")

        desc = registry.get("my:tool")
        assert desc is not None
        assert desc.name == "my:tool"
        assert desc.description == "My tool description"

    def test_get_all(self):
        """Test getting all tools."""
        registry = ToolRegistry()
        registry.register_simple("tool1", "First")
        registry.register_simple("tool2", "Second")

        all_tools = registry.get_all()
        assert len(all_tools) == 2
        assert {t.name for t in all_tools} == {"tool1", "tool2"}

    def test_category_grouping(self):
        """Test categorization."""
        registry = ToolRegistry()
        registry.register(ToolDescriptor("fs:read", "Read", category="filesystem"))
        registry.register(ToolDescriptor("fs:write", "Write", category="filesystem"))
        registry.register(ToolDescriptor("shell:run", "Run", category="shell"))

        fs_tools = registry.get_by_category("filesystem")
        assert len(fs_tools) == 2
        assert all(t.category == "filesystem" for t in fs_tools)

    def test_format_compact(self):
        """Test compact formatting for prompt."""
        registry = ToolRegistry()
        registry.register_simple("tool1", "First tool")
        registry.register_simple("tool2", "Second tool")

        formatted = registry.format_for_prompt(detailed=False)
        assert "• tool1() - First tool" in formatted
        assert "• tool2() - Second tool" in formatted

    def test_format_with_categories(self):
        """Test formatting with category grouping."""
        registry = ToolRegistry()
        registry.register(ToolDescriptor("fs:read", "Read", category="filesystem"))
        registry.register(ToolDescriptor("shell:run", "Run", category="shell"))

        formatted = registry.format_for_prompt(group_by_category=True)
        assert "FILESYSTEM:" in formatted
        assert "SHELL:" in formatted

    def test_format_specific_tools(self):
        """Test formatting only specific tools."""
        registry = ToolRegistry()
        registry.register_simple("tool1", "First")
        registry.register_simple("tool2", "Second")
        registry.register_simple("tool3", "Third")

        formatted = registry.format_for_prompt(tool_names=["tool1", "tool3"])
        assert "tool1" in formatted
        assert "tool3" in formatted
        assert "tool2" not in formatted

    def test_default_registry(self):
        """Test default registry has common tools."""
        registry = create_default_registry()

        # Should have filesystem tools
        assert registry.get("fs:read_file") is not None
        assert registry.get("fs:write_file") is not None
        assert registry.get("fs:search") is not None

        # Should have shell tools
        assert registry.get("shell:run") is not None

        # Should have web tools
        assert registry.get("web:fetch") is not None
        assert registry.get("web:search") is not None


class TestFewShotExample:
    """Test FewShotExample functionality."""

    def test_format_for_prompt(self):
        """Test ReAct-style formatting."""
        example = FewShotExample(
            goal="Find files",
            thought="I need to search",
            action='{"tool": "fs:search"}',
            observation="Found 5 files",
        )

        result = example.format_for_prompt()
        assert "Thought: I need to search" in result
        assert 'Action: {"tool": "fs:search"}' in result
        assert "Observation: Found 5 files" in result


class TestFewShotLibrary:
    """Test FewShotLibrary functionality."""

    def test_add_and_get_by_category(self):
        """Test adding and retrieving by category."""
        library = FewShotLibrary()

        example1 = FewShotExample("goal1", "thought1", "action1", "obs1", category="filesystem")
        example2 = FewShotExample("goal2", "thought2", "action2", "obs2", category="filesystem")
        example3 = FewShotExample("goal3", "thought3", "action3", "obs3", category="shell")

        library.add(example1)
        library.add(example2)
        library.add(example3)

        fs_examples = library.get_by_category("filesystem")
        assert len(fs_examples) == 2

    def test_get_relevant_by_keywords(self):
        """Test keyword-based relevance matching."""
        library = FewShotLibrary()

        library.add(
            FewShotExample(
                "Find Python files",
                "Search for .py files",
                "search",
                "found files",
            )
        )
        library.add(
            FewShotExample(
                "Read config",
                "Read JSON config",
                "read",
                "config data",
            )
        )
        library.add(
            FewShotExample(
                "Search Python code",
                "Search in Python files",
                "search",
                "found matches",
            )
        )

        # Should find examples with "python" keyword
        relevant = library.get_relevant("Find all Python TODO comments", max_examples=2)
        assert len(relevant) <= 2
        assert any("Python" in ex.goal or "Python" in ex.thought for ex in relevant)

    def test_format_examples(self):
        """Test formatting multiple examples."""
        library = FewShotLibrary()

        example1 = FewShotExample("goal1", "thought1", "action1", "obs1")
        example2 = FewShotExample("goal2", "thought2", "action2", "obs2")

        library.add(example1)
        library.add(example2)

        formatted = library.format_examples([example1, example2])
        assert "Example 1 (Goal: goal1):" in formatted
        assert "Example 2 (Goal: goal2):" in formatted
        assert "Thought: thought1" in formatted
        assert "Thought: thought2" in formatted

    def test_default_library(self):
        """Test default library has examples."""
        library = get_default_library()

        # Should have examples in various categories
        fs_examples = library.get_by_category("filesystem")
        assert len(fs_examples) > 0

        shell_examples = library.get_by_category("shell")
        assert len(shell_examples) > 0

        research_examples = library.get_by_category("research")
        assert len(research_examples) > 0

    def test_singleton_library(self):
        """Test that get_default_library returns same instance."""
        lib1 = get_default_library()
        lib2 = get_default_library()
        assert lib1 is lib2  # Same object
