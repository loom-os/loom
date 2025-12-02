from __future__ import annotations

import inspect
import json
from typing import Any, Callable, Optional

from pydantic import BaseModel, create_model


class Tool:
    """Represents a registered tool with its metadata and handler function.

    This replaces the deprecated Capability class to align with loom-core's
    unified Tool API.
    """

    def __init__(
        self,
        name: str,
        description: str,
        func: Callable[..., Any],
        input_model: Optional[type[BaseModel]],
        output_model: Optional[type[BaseModel]],
    ):
        self.name = name
        self.description = description
        self.func = func
        self.input_model = input_model
        self.output_model = output_model

    @property
    def parameters_schema(self) -> str:
        """Returns JSON Schema for tool parameters."""
        if self.input_model:
            return json.dumps(self.input_model.model_json_schema(), separators=(",", ":"))
        return "{}"


def _model_from_signature(
    func: Callable[..., Any],
) -> tuple[Optional[type[BaseModel]], Optional[type[BaseModel]]]:
    """Extract input/output Pydantic models from function signature."""
    sig = inspect.signature(func)
    fields = {}
    for name, param in sig.parameters.items():
        if name == "self":
            continue
        ann = (
            param.annotation
            if param.annotation is not inspect.Parameter.empty
            else (str if param.default is inspect.Parameter.empty else type(param.default))
        )
        default = ... if param.default is inspect.Parameter.empty else param.default
        fields[name] = (ann, default)
    input_model = create_model(f"{func.__name__.capitalize()}Input", **fields) if fields else None

    # Output model: from return annotation if it's a BaseModel subtype
    return_ann = sig.return_annotation
    output_model = None
    if return_ann is not inspect.Signature.empty:
        try:
            if issubclass(return_ann, BaseModel):
                output_model = return_ann
        except TypeError:
            pass
    return input_model, output_model


def tool(name: str, description: str = ""):
    """Decorator to declare a callable as a Loom tool.

    This replaces the deprecated @capability decorator to align with
    loom-core's unified Tool API.

    Args:
        name: Unique tool name (e.g., "web.search", "file.read")
        description: Human-readable description of what the tool does

    Example:
        @tool("web.search", description="Search the web for information")
        def web_search(query: str) -> dict:
            ...

        @tool("math.add", description="Add two numbers")
        async def add(a: int, b: int) -> int:
            return a + b
    """

    def wrapper(func: Callable[..., Any]):
        input_model, output_model = _model_from_signature(func)
        t = Tool(
            name=name,
            description=description or func.__doc__ or "",
            func=func,
            input_model=input_model,
            output_model=output_model,
        )
        func.__loom_tool__ = t
        return func

    return wrapper


# Backwards compatibility aliases (deprecated)
Capability = Tool
capability = tool


__all__ = ["Tool", "tool", "Capability", "capability"]
