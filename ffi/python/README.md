# tools-rs

Python decorator for defining [tools-rs](https://github.com/EggerMarc/tools-rs) LLM tool functions.

## What is this?

[tools-rs](https://crates.io/crates/tools-rs) is a Rust framework for building, registering, and executing tools with automatic JSON schema generation for LLM integration. Its Python FFI adapter lets end users write tools in Python that get loaded into a Rust application.

This pip package provides the `@tool` decorator so your IDE can resolve imports, provide autocomplete, and type-check your tool definitions. **At runtime, the Rust adapter auto-injects the same decorator** — this package is optional and only needed for IDE support.

## Usage

```bash
pip install tools-rs
```

Then write your tools:

```python
from tools_rs import tool

@tool(requires_approval=False, cost_tier=1)
def weather(city: str, units: str = "celsius") -> str:
    """Get current weather for a city.

    Args:
        city: City name to look up
        units: Temperature unit (celsius or fahrenheit)
    """
    import requests
    resp = requests.get(f"https://wttr.in/{city}?format=j1")
    return resp.json()["current_condition"][0]["temp_C"]
```

The `@tool` decorator extracts:
- **Name** from the function name
- **Description** from the docstring (first paragraph)
- **Parameter schema** from type hints (`str` → `"string"`, `int` → `"integer"`, `Literal["a", "b"]` → `enum`, etc.)
- **Parameter descriptions** from Google-style `Args:` docstring blocks
- **Required vs optional** from default values
- **Metadata** from decorator keyword arguments

## Supported type hints

| Python type | JSON Schema |
|---|---|
| `str` | `{"type": "string"}` |
| `int` | `{"type": "integer"}` |
| `float` | `{"type": "number"}` |
| `bool` | `{"type": "boolean"}` |
| `list[T]` | `{"type": "array", "items": ...}` |
| `Literal["a", "b"]` | `{"type": "string", "enum": ["a", "b"]}` |
| `Optional[T]` | `{"anyOf": [..., {"type": "null"}]}` |

## License

MIT
