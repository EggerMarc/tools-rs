//! The `@tool` decorator Python source, auto-injected at runtime.

/// Python source for the `@tool` decorator.
///
/// This is prepended to every Python module before execution. It
/// attaches a `__tool__` dict to decorated functions containing:
/// - `name`: function name
/// - `description`: first paragraph of docstring
/// - `meta`: decorator keyword arguments
/// - `parameters`: JSON-Schema-compatible dict from type hints
pub const DECORATOR_SOURCE: &str = r#"
import inspect
import typing
import json

def _python_type_to_json(t):
    """Convert a Python type hint to a JSON Schema type string."""
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
        schema = {"enum": values}
        types = set()
        for v in values:
            if isinstance(v, str): types.add("string")
            elif isinstance(v, bool): types.add("boolean")
            elif isinstance(v, int): types.add("integer")
            elif isinstance(v, float): types.add("number")
        if len(types) == 1:
            schema["type"] = types.pop()
        return schema
    if origin is typing.Union:
        args = [a for a in getattr(t, "__args__", ()) if a is not type(None)]
        null = any(a is type(None) for a in getattr(t, "__args__", ()))
        parts = [_python_type_to_json(a) for a in args]
        if null:
            parts.append({"type": "null"})
        if len(parts) == 1:
            return parts[0]
        return {"anyOf": parts}
    return {"type": "string"}

def _parse_args_block(doc):
    """Extract param descriptions from Google-style Args block."""
    descs = {}
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
    """Decorator that marks a function as a tools-rs tool."""
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
"#;
