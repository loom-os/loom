"""Unit tests for loom.tool decorator."""

import json

from pydantic import BaseModel

from loom import Tool, tool


def test_tool_decorator_basic() -> None:
    """Test basic tool decorator usage."""

    @tool("test.tool", description="A test tool")
    def test_func(query: str) -> dict:
        return {"result": query}

    assert hasattr(test_func, "__loom_tool__")
    t: Tool = test_func.__loom_tool__  # type: ignore[attr-defined]
    assert t.name == "test.tool"
    assert t.description == "A test tool"


def test_tool_parameters_schema() -> None:
    """Test tool parameters schema includes JSON schema."""

    @tool("test.schema", description="Test schema generation")
    def func_with_params(name: str, age: int, active: bool = True) -> dict:
        return {"name": name, "age": age, "active": active}

    t: Tool = func_with_params.__loom_tool__  # type: ignore[attr-defined]
    params_schema = t.parameters_schema

    # Check that parameters_schema is valid JSON
    schema = json.loads(params_schema)
    assert "properties" in schema
    assert "name" in schema["properties"]
    assert "age" in schema["properties"]
    assert "active" in schema["properties"]


def test_tool_invocation() -> None:
    """Test calling a tool function."""

    @tool("test.invoke", description="Add two numbers")
    def add_numbers(a: int, b: int) -> int:
        return a + b

    result = add_numbers(5, 3)
    assert result == 8


def test_tool_with_pydantic_model() -> None:
    """Test tool with Pydantic model input."""

    class PersonInput(BaseModel):
        name: str
        age: int

    @tool("test.pydantic", description="Process a person")
    def process_person(person: PersonInput) -> dict:
        return {"name": person.name, "age": person.age, "processed": True}

    t: Tool = process_person.__loom_tool__  # type: ignore[attr-defined]
    params_schema = t.parameters_schema

    # Pydantic models should be used for schema
    assert params_schema is not None
    assert len(params_schema) > 2  # More than just "{}"


def test_tool_validation() -> None:
    """Test that tool validates input."""

    @tool("test.validate", description="Strict function")
    def strict_func(count: int) -> str:
        return f"Count: {count}"

    # Should work with correct type
    result = strict_func(42)
    assert result == "Count: 42"

    # Type checking happens at runtime in Python, but tool
    # parameters_schema should enable validation on the receiving side


def test_multiple_tools() -> None:
    """Test multiple tools can be defined."""

    @tool("math.add", description="Add two numbers")
    def add(a: int, b: int) -> int:
        return a + b

    @tool("math.multiply", description="Multiply two numbers")
    def multiply(a: int, b: int) -> int:
        return a * b

    add_tool: Tool = add.__loom_tool__  # type: ignore[attr-defined]
    mult_tool: Tool = multiply.__loom_tool__  # type: ignore[attr-defined]

    assert add_tool.name == "math.add"
    assert mult_tool.name == "math.multiply"
    assert add_tool.name != mult_tool.name


def test_tool_with_docstring_as_description() -> None:
    """Test that docstring is used as description when not provided."""

    @tool("test.docstring")
    def documented_func(x: int) -> int:
        """This is the function documentation."""
        return x * 2

    t: Tool = documented_func.__loom_tool__  # type: ignore[attr-defined]
    assert t.description == "This is the function documentation."


def test_backwards_compatibility_capability_alias() -> None:
    """Test that capability decorator still works for backwards compatibility."""
    from loom import Capability, capability

    @capability("legacy.capability", description="Legacy test")
    def legacy_func(x: int) -> int:
        return x + 1

    # Should have both attributes (tool is the primary)
    assert hasattr(legacy_func, "__loom_tool__")

    # capability/Capability are aliases
    assert capability == tool
    assert Capability == Tool
