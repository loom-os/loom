from __future__ import annotations
from typing import Any, Callable, Optional
from pydantic import BaseModel, create_model
import json
import inspect

class Capability:
    def __init__(self, name: str, version: str, func: Callable[..., Any], input_model: Optional[type[BaseModel]], output_model: Optional[type[BaseModel]]):
        self.name = name
        self.version = version
        self.func = func
        self.input_model = input_model
        self.output_model = output_model

    @property
    def metadata(self) -> dict:
        meta: dict[str, str] = {}
        if self.input_model:
            # Use model_json_schema() to get dict, then dump for portability
            meta["input_schema"] = json.dumps(self.input_model.model_json_schema(), separators=(",", ":"))
        if self.output_model:
            meta["output_schema"] = json.dumps(self.output_model.model_json_schema(), separators=(",", ":"))
        return meta


def _model_from_signature(func: Callable[..., Any]) -> tuple[Optional[type[BaseModel]], Optional[type[BaseModel]]]:
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
    # Output model: from return annotation if it's a BaseModel subtype, else generic
    return_ann = sig.return_annotation
    output_model = None
    if return_ann is not inspect.Signature.empty:
        try:
            if issubclass(return_ann, BaseModel):
                output_model = return_ann
        except TypeError:
            pass
    return input_model, output_model


def capability(name: str, version: str = "1.0"):
    """Decorator to declare a callable as a Loom capability.

    Example:
        @capability("web.search", version="1.0")
        def web_search(query: str) -> dict: ...
    """
    def wrapper(func: Callable[..., Any]):
        input_model, output_model = _model_from_signature(func)
        cap = Capability(name=name, version=version, func=func, input_model=input_model, output_model=output_model)
        setattr(func, "__loom_capability__", cap)
        return func
    return wrapper

__all__ = ["Capability", "capability"]
