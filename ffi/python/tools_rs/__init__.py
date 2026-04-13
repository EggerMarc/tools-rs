"""tools-rs: Python decorator for defining LLM tool functions.

This module provides the ``@tool`` decorator used to mark Python functions
as tools-rs tools. The Rust FFI adapter auto-injects this same code at
runtime, so installing this package is optional — it exists for IDE
autocomplete and type-checking support.
"""

from __future__ import annotations

import inspect
import typing

__all__ = ["tool"]


def _python_type_to_json(t: type) -> dict:
    """Convert a Python type hint to a JSON Schema dict."""
    if t is str:
        return {"type": "string"}
    if t is int:
        return {"type": "integer"}
    if t is float:
        return {"type": "number"}
    if t is bool:
        return {"type": "boolean"}

    origin = getattr(t, "__origin__", None)

    if origin is list:
        args = getattr(t, "__args__", (str,))
        return {"type": "array", "items": _python_type_to_json(args[0])}

    if origin is typing.Literal:
        values = list(getattr(t, "__args__", ()))
        schema: dict = {"enum": values}
        types = set()
        for v in values:
            if isinstance(v, str):
                types.add("string")
            elif isinstance(v, bool):
                types.add("boolean")
            elif isinstance(v, int):
                types.add("integer")
            elif isinstance(v, float):
                types.add("number")
        if len(types) == 1:
            schema["type"] = types.pop()
        return schema

    if origin is typing.Union:
        all_args = getattr(t, "__args__", ())
        args = [a for a in all_args if a is not type(None)]
        has_none = any(a is type(None) for a in all_args)
        parts = [_python_type_to_json(a) for a in args]
        if has_none:
            parts.append({"type": "null"})
        if len(parts) == 1:
            return parts[0]
        return {"anyOf": parts}

    return {"type": "string"}


def _parse_args_block(doc: str) -> dict[str, str]:
    """Extract parameter descriptions from a Google-style Args block."""
    descs: dict[str, str] = {}
    if not doc:
        return descs
    in_args = False
    for line in doc.split("\n"):
        stripped = line.strip()
        if stripped.startswith("Args:"):
            in_args = True
            continue
        if in_args:
            if not stripped or (not line.startswith(" ") and not line.startswith("\t")):
                break
            if ":" in stripped:
                param, desc = stripped.split(":", 1)
                descs[param.strip()] = desc.strip()
    return descs


def tool(**meta):
    """Decorator that marks a function as a tools-rs tool.

    Keyword arguments become tool metadata (e.g. ``requires_approval=True``).

    The decorated function must have type-annotated parameters. The
    decorator attaches a ``__tool__`` dict containing the tool's name,
    description, JSON schema, and metadata.

    Example::

        @tool(requires_approval=False)
        def greet(name: str, times: int = 1) -> str:
            \"\"\"Greet someone.

            Args:
                name: Person to greet
                times: How many times
            \"\"\"
            return ("Hello, " + name + "! ") * times
    """

    def decorator(fn):
        hints = typing.get_type_hints(fn)
        hints.pop("return", None)
        sig = inspect.signature(fn)
        doc = inspect.getdoc(fn) or ""
        param_descs = _parse_args_block(doc)

        properties = {}
        required = []
        for name, p in sig.parameters.items():
            hint = hints.get(name, str)
            schema = _python_type_to_json(hint)
            if name in param_descs:
                schema["description"] = param_descs[name]
            properties[name] = schema
            if p.default is inspect.Parameter.empty:
                required.append(name)

        fn.__tool__ = {
            "name": fn.__name__,
            "description": doc.split("\n\n")[0].strip() if doc else "",
            "meta": meta,
            "parameters": {
                "type": "object",
                "properties": properties,
                "required": required,
            },
        }
        return fn

    return decorator
