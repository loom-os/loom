"""Unit tests for loom.capability decorator."""

import json

from pydantic import BaseModel

from loom import capability
from loom.capability import Capability


def test_capability_decorator_basic() -> None:
    """Test basic capability decorator usage."""

    @capability("test.capability", version="1.0")
    def test_func(query: str) -> dict:
        return {"result": query}

    assert hasattr(test_func, "__loom_capability__")
    cap: Capability = test_func.__loom_capability__  # type: ignore[attr-defined]
    assert cap.name == "test.capability"
    assert cap.version == "1.0"


def test_capability_metadata_schema() -> None:
    """Test capability metadata includes JSON schema."""

    @capability("test.schema", version="2.0")
    def func_with_params(name: str, age: int, active: bool = True) -> dict:
        return {"name": name, "age": age, "active": active}

    cap: Capability = func_with_params.__loom_capability__  # type: ignore[attr-defined]
    metadata = cap.metadata

    # Check that input_schema exists and is valid JSON
    assert "input_schema" in metadata

    schema = json.loads(metadata["input_schema"])
    assert "properties" in schema
    assert "name" in schema["properties"]
    assert "age" in schema["properties"]
    assert "active" in schema["properties"]


def test_capability_invocation() -> None:
    """Test calling a capability function."""

    @capability("test.invoke", version="1.0")
    def add_numbers(a: int, b: int) -> int:
        return a + b

    result = add_numbers(5, 3)
    assert result == 8


def test_capability_with_pydantic_model() -> None:
    """Test capability with Pydantic model input."""

    class PersonInput(BaseModel):
        name: str
        age: int

    @capability("test.pydantic")
    def process_person(person: PersonInput) -> dict:
        return {"name": person.name, "age": person.age, "processed": True}

    cap: Capability = process_person.__loom_capability__  # type: ignore[attr-defined]
    metadata = cap.metadata

    assert "input_schema" in metadata
    # Pydantic models should be directly used as schema
    assert metadata["input_schema"] is not None


def test_capability_validation() -> None:
    """Test that capability validates input."""

    @capability("test.validate")
    def strict_func(count: int) -> str:
        return f"Count: {count}"

    # Should work with correct type
    result = strict_func(42)
    assert result == "Count: 42"

    # Type checking happens at runtime in Python, but capability
    # metadata should enable validation on the receiving side


def test_multiple_capabilities() -> None:
    """Test multiple capabilities can be defined."""

    @capability("math.add")
    def add(a: int, b: int) -> int:
        return a + b

    @capability("math.multiply")
    def multiply(a: int, b: int) -> int:
        return a * b

    add_cap: Capability = add.__loom_capability__  # type: ignore[attr-defined]
    mult_cap: Capability = multiply.__loom_capability__  # type: ignore[attr-defined]

    assert add_cap.name == "math.add"
    assert mult_cap.name == "math.multiply"
    assert add_cap.name != mult_cap.name
