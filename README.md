# Tools-rs - Tool Collection and Execution Framework
*It's pronounced tools-r-us!!*

[![Crates.io](https://img.shields.io/crates/v/tools-rs.svg)](https://crates.io/crates/tools-rs)
[![Documentation](https://docs.rs/tools-rs/badge.svg)](https://docs.rs/tools-rs)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

Tools-rs is a framework for building, registering, and executing tools with automatic JSON schema generation for Large Language Model (LLM) integration.

## Features

- **Automatic Registration** - Use `#[tool]` to automatically register functions with compile-time discovery
- **JSON Schema Generation** - Automatic schema generation for LLM integration with full type information
- **Type Safety** - Full type safety with JSON serialization at boundaries, compile-time parameter validation
- **Async Support** - Built for async/await from the ground up with `tokio` integration
- **Error Handling** - Comprehensive error types with context and proper error chaining
- **LLM Integration** - Export function declarations for LLM function calling APIs (OpenAI, Anthropic, etc.)
- **Manual Registration** - Programmatic tool registration for dynamic scenarios
- **Inventory System** - Link-time tool collection using the `inventory` crate for zero-runtime-cost discovery
- **Typed Metadata** - Attach `#[tool(key = value)]` attributes to tools and read them through a user-defined `M` type on `ToolCollection<M>` (see [Tool Metadata](#tool-metadata))
- **Shared Context** - Inject shared resources (`Arc<T>`) into tools via `ctx` first parameter and `ToolCollection::builder().with_context(...)` (see [Shared Context](#shared-context))

## Quick Start

```rust
use serde_json::json;
use tools_rs::{collect_tools, FunctionCall, tool};

#[tool]
/// Adds two numbers.
async fn add(pair: (i32, i32)) -> i32 {
    pair.0 + pair.1
}

#[tool]
/// Greets a person.
async fn greet(name: String) -> String {
    format!("Hello, {name}!")
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let tools = collect_tools();

    let sum = tools
        .call(FunctionCall::new(
            "add".into(),
            json!({ "pair": [3, 4] }),
        ))
        .await?.result;
    println!("add → {sum}");  // Outputs: "add → 7"

    let hi = tools
        .call(FunctionCall::new(
            "greet".into(),
            json!({ "name": "Alice" }),
        ))
        .await?.result;
    println!("greet → {hi}");  // Outputs: "greet → \"Hello, Alice!\""

    // Export function declarations for LLM APIs
    let declarations = tools.json()?;
    println!("Function declarations: {}", serde_json::to_string_pretty(&declarations)?);

    Ok(())
}
```

## Installation

Add the following to your `Cargo.toml`:

```toml
[dependencies]
tools-rs = "0.3.1"
tokio = { version = "1.45", features = ["macros", "rt-multi-thread"] }
serde_json = "1.0"
```

## Crate Structure

The tools-rs system is organized as a Rust workspace with three main crates:

- **tools-rs**: Main entry point, re-exports the most commonly used items
- **tools_core**: Core runtime implementation including:
  - Tool collection and execution (`ToolCollection`)
  - JSON schema generation (`ToolSchema` trait)
  - Error handling (`ToolError`, `DeserializationError`)
  - Core data structures (`FunctionCall`, `ToolRegistration`, etc.)
- **tools_macros**: Procedural macros for tool registration:
  - `#[tool]` attribute macro for automatic registration
  - `#[derive(ToolSchema)]` for automatic schema generation
- **examples**: Comprehensive examples demonstrating different use cases

For more details about the codebase organization, see [CODE_ORGANIZATION.md](CODE_ORGANIZATION.md).

## Compatibility

### Rust Version Support

Tools-rs requires **Rust 1.85** or later and supports:
- Automatically generate JSON schemas for LLM consumption
- Execute tools safely with full type checking
- Handle errors gracefully with detailed context

## Function Declarations for LLMs

Tools-rs can automatically generate function declarations suitable for LLM APIs:

```rust
use tools_rs::{function_declarations, tool};

#[tool]
/// Return the current date in ISO-8601 format.
async fn today() -> String {
    chrono::Utc::now().date_naive().to_string()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Generate function declarations for an LLM
    let declarations = function_declarations()?;

    // Use in API request
    let llm_request = serde_json::json!({
        "model": "gpt-4o",
        "messages": [/* ... */],
        "tools": declarations
    });

    Ok(())
}
```

The generated declarations follow proper JSON Schema format:

```json
[
  {
    "description": "Return the current date in ISO-8601 format.",
    "name": "today",
    "parameters": {
      "properties": {},
      "required": [],
      "type": "object"
    }
  }
]
```

## Manual Registration

While the `#[tool]` macro provides the most convenient way to register tools, you can also register tools manually for more dynamic scenarios:

```rust
use tools_rs::ToolCollection;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut tools: ToolCollection = ToolCollection::new();

    // Register a simple tool manually. Pass `()` as the metadata when the
    // collection uses the default `NoMeta`; pass a real `M` value when the
    // collection is typed (e.g. `ToolCollection::<MyPolicy>::new()`).
    tools.register(
        "multiply",
        "Multiplies two numbers",
        |pair: (i64, i64)| async move { pair.0 * pair.1 },
        (),
    )?;

    // Call the manually registered tool
    let result = tools.call(tools_rs::FunctionCall {
        id: None, // Refers to the call ID
        name: "multiply".to_string(),
        arguments: json!({"a": 6, "b": 7}),
    }).await?;

    println!("6 * 7 = {}", result);
    Ok(())
}
```

### Advanced Manual Registration

For complex scenarios with custom types:

```rust
use tools_rs::{ToolCollection, ToolSchema};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, ToolSchema)]
struct Calculator {
    operation: String,
    operands: Vec<f64>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut tools: ToolCollection = ToolCollection::new();

    tools.register(
        "calculate",
        "Performs arithmetic operations on a list of numbers",
        |input: Calculator| async move {
            match input.operation.as_str() {
                "sum" => input.operands.iter().sum::<f64>(),
                "product" => input.operands.iter().product::<f64>(),
                "mean" => input.operands.iter().sum::<f64>() / input.operands.len() as f64,
                _ => f64::NAN,
            }
        },
        (),
    )?;

    Ok(())
}
```

## Tool Metadata

`#[tool(...)]` accepts flat `key = value` attributes that get stored on each
tool and read back through a user-defined metadata type. This lets a single
tool declaration feed multiple collections, each typed to the schema *that*
collection cares about — useful for HITL approval gates, cost tiering, and
similar policy concerns that don't belong in the function body.

```rust
use serde::Deserialize;
use tools_rs::{tool, ToolCollection};

#[derive(Debug, Default, Deserialize, Clone, Copy)]
#[serde(default)]
struct Policy {
    requires_approval: bool,
    cost_tier: u8,
}

#[tool(requires_approval = true, cost_tier = 3)]
/// Deletes a file.
async fn delete_file(path: String) -> String { format!("deleted {path}") }

#[tool]
/// Reads a file (no metadata declared — fields default).
async fn read_file(path: String) -> String { format!("read {path}") }

fn main() -> Result<(), tools_rs::ToolError> {
    let tools = ToolCollection::<Policy>::collect_tools()?;
    let entry = tools.get("delete_file").unwrap();
    if entry.meta.requires_approval {
        // gate the call behind your approval flow
    }
    Ok(())
}
```

### Default behavior

`ToolCollection` defaults to `ToolCollection<NoMeta>`. `NoMeta` is an empty
struct that swallows any attributes a tool declared, so existing
`collect_tools()` callers see no behavioral change. Opt into typed metadata
by writing `ToolCollection::<MyMeta>::collect_tools()` instead.

### Validation

`ToolCollection::<M>::collect_tools()` fails fast on the first tool whose
attributes don't match `M`. For CI, two helpers accumulate every failure
across the inventory before returning:

```rust
use tools_core::{validate_tool_attrs, validate_tool_attrs_for};
# #[derive(serde::Deserialize)] struct Policy;

#[test]
fn every_tool_conforms_to_policy() {
    validate_tool_attrs::<Policy>().unwrap();
}

#[test]
fn destructive_tools_have_approval_metadata() {
    // Subset gating: only check the named tools, error if any name doesn't
    // match a registered tool (typos in the test list are caught too).
    validate_tool_attrs_for::<Policy>(&["delete_file", "drop_table"]).unwrap();
}
```

### Attribute syntax

- `#[tool(key = "value")]` — string
- `#[tool(key = 42)]` — integer (negative literals are accepted: `-3`)
- `#[tool(key = 1.5)]` — float
- `#[tool(key = true)]` — boolean
- `#[tool(flag)]` — bare flag, equivalent to `flag = true`
- Multiple attributes are comma-separated: `#[tool(a = 1, b = "x", flag)]`

Attributes are flat-only — nested structures (`#[tool(policy = { ... })]`)
are not supported. Use richer types in runtime metadata, not at the
attribute site. The keys `name` and `description` are reserved (the
function name and doc comment supply them).

### Programmatic registration with metadata

`ToolCollection::register` takes a metadata argument. For untyped
collections, pass `()`; passing `()` to a typed collection is a compile
error.

```rust
# use tools_rs::ToolCollection;
# #[derive(Default)] struct Policy { requires_approval: bool }
let mut tools: ToolCollection<Policy> = ToolCollection::new();
tools.register(
    "noop",
    "does nothing",
    |_: ()| async {},
    Policy { requires_approval: false },
)?;
# Ok::<(), tools_rs::ToolError>(())
```

## Shared Context

Tools often need access to shared resources — database connections, HTTP
clients, caches. If a tool's **first parameter is named `ctx`**, the macro
treats it as a shared context that the collection injects at call time.
Callers only pass the "real" arguments; `ctx` never appears in the JSON
schema.

```rust
use std::sync::Arc;
use tools_rs::{tool, ToolCollection, FunctionCall};
use tools_core::NoMeta;

struct Db {
    url: String,
}

#[tool]
/// Look up a user by name.
async fn find_user(ctx: Db, name: String) -> String {
    format!("[{}] SELECT * FROM users WHERE name = '{name}'", ctx.url)
}

#[tool]
/// A plain tool — no context needed.
async fn ping() -> String { "pong".into() }

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db = Arc::new(Db { url: "postgres://localhost/app".into() });

    let tools = ToolCollection::<NoMeta>::builder()
        .with_context(db)
        .collect()?;

    // Caller never mentions ctx — only the "real" args:
    let resp = tools.call(FunctionCall::new(
        "find_user".into(),
        serde_json::json!({ "name": "Alice" }),
    )).await?;
    println!("{}", resp.result);
    Ok(())
}
```

### How it works

1. **Macro detection** — if the first parameter is named `ctx`, it is
   excluded from the JSON schema wrapper struct. The macro rewrites the
   emitted function signature from `ctx: T` to `ctx: Arc<T>` so that field
   access and method calls work via `Deref`.
2. **Builder** — `ToolCollection::builder().with_context(arc).collect()`
   stores the `Arc<T>` and validates at startup that every context-requiring
   tool expects the same type (via `TypeId` comparison). A mismatch produces
   a clear `CtxTypeMismatch` error.
3. **Call-time injection** — `collection.call(...)` passes the stored
   context into the closure. Non-context tools ignore it.

### Important: write `ctx: T`, not `ctx: Arc<T>`

The macro wraps the type in `Arc` automatically. Writing `ctx: Arc<T>`
produces a compile error to prevent accidental double-wrapping.

### Interior mutability

`Arc<T>` is immutable, but you can wrap an interior-mutable type:

```rust
use std::sync::Mutex;
# use tools_rs::tool;

struct Cache { entries: Vec<String> }

#[tool]
/// Record a cache hit.
async fn record(ctx: Mutex<Cache>, key: String) -> String {
    ctx.lock().unwrap().entries.push(key.clone());
    format!("recorded {key}")
}
```

The builder receives `Arc::new(Mutex::new(cache))`. The tool sees
`Arc<Mutex<Cache>>` via Deref, so `ctx.lock()` works directly.

### Context + metadata

Context and metadata are orthogonal. Combine them freely:

```rust
# use std::sync::Arc;
# use serde::Deserialize;
# use tools_rs::{tool, ToolCollection};
# struct Db { url: String }
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct Policy { requires_approval: bool }

#[tool(requires_approval = true)]
/// Drop a table — requires approval.
async fn drop_table(ctx: Db, table: String) -> String {
    format!("[{}] DROP TABLE {table}", ctx.url)
}

# fn example() -> Result<(), tools_rs::ToolError> {
let tools = ToolCollection::<Policy>::builder()
    .with_context(Arc::new(Db { url: "pg://localhost".into() }))
    .collect()?;

let meta = tools.meta("drop_table").unwrap();
assert!(meta.requires_approval);
# Ok(())
# }
```

### Without context

`collect_tools()` and `ToolCollection::collect_tools()` still work for
collections that don't need context. If any tool in the inventory requires
context and you call `collect_tools()` without the builder, it fails with
`ToolError::MissingCtx` at startup.

## ToolsBuilder (Typestate Builder)

`ToolsBuilder` is a typestate-based builder for `ToolCollection` that
enforces invariants at compile time. It is the recommended way to
construct collections when you need context or typed metadata.

```rust
use std::sync::Arc;
use serde::Deserialize;
use tools_rs::{ToolsBuilder, ToolCollection, FunctionCall};

struct AppState { db_url: String }

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct Policy { requires_approval: bool }

# fn example() -> Result<(), tools_rs::ToolError> {
// Simple — equivalent to collect_tools():
let tools = ToolsBuilder::new().collect()?;

// With context and typed metadata:
let state = Arc::new(AppState { db_url: "pg://localhost".into() });
let tools = ToolsBuilder::new()
    .with_context(state)
    .with_meta::<Policy>()
    .collect()?;
# Ok(())
# }
```

### Typestate enforcement

The builder uses typestate to prevent invalid combinations at compile
time. Once `with_context` is called, the builder transitions to the
`Native` state where `with_context` can no longer be called:

```compile_fail
use tools_rs::ToolsBuilder;
use std::sync::Arc;

// ERROR: with_context cannot be called twice
let b = ToolsBuilder::new()
    .with_context(Arc::new(42_u32))
    .with_context(Arc::new(42_u32));
```

### Raw tool registration

`register_raw` lets you register tools from a pre-built JSON schema
and a raw async closure, bypassing `ToolSchema` derivation. This is
useful for dynamic tool registration or tools coming from external
sources.

```rust
use tools_rs::{ToolsBuilder, ToolCollection, FunctionCall};
use serde_json::json;

# async fn example() -> Result<(), Box<dyn std::error::Error>> {
let mut tools: ToolCollection = ToolsBuilder::new().collect()?;

tools.register_raw(
    "echo",
    "Echoes the input message back",
    json!({
        "type": "object",
        "properties": {
            "msg": { "type": "string", "description": "Message to echo" }
        },
        "required": ["msg"]
    }),
    |args| Box::pin(async move {
        let msg = args.get("msg").and_then(|m| m.as_str()).unwrap_or("");
        Ok(serde_json::Value::String(msg.to_string()))
    }),
    (),
)?;

let resp = tools.call(FunctionCall::new(
    "echo".into(),
    json!({ "msg": "hello" }),
)).await?;
assert_eq!(resp.result, json!("hello"));
# Ok(())
# }
```

## Examples

Check out the [examples directory](examples/) for comprehensive sample code:

```bash
# Run the basic example - simple tool registration and calling
cargo run --example basic

# Run the function declarations example - LLM integration demo
cargo run --example function_declarations

# Run the schema example - complex type schemas and validation
cargo run --example schema

# Run the newtype demo - custom type wrapping examples
cargo run --example newtype_demo

# Run the HITL example - human-in-the-loop approval gating with metadata
cargo run --example hitl

# Run the context example - shared context injection
cargo run --example context
```

Each example demonstrates different aspects of the framework:

- **basic**: Simple tool registration with `#[tool]` and basic function calls
- **function_declarations**: Complete LLM integration workflow with JSON schema generation
- **schema**: Advanced schema generation for complex nested types and collections
- **newtype_demo**: Working with custom wrapper types and serialization patterns
- **hitl**: Human-in-the-loop approval gating using typed metadata
- **context**: Shared context injection via `ToolCollection::builder().with_context(...)`

## API Reference

### Core Functions

- `collect_tools()` - Discover all tools registered via `#[tool]` macro
- `function_declarations()` - Generate JSON schema declarations for LLMs
- `call_tool(name, args)` - Execute a tool by name with JSON arguments
- `call_tool_with(name, typed_args)` - Execute a tool with typed arguments
- `call_tool_by_name(collection, name, args)` - Execute tool on specific collection
- `list_tool_names(collection)` - List all available tool names

### Core Types

- `ToolCollection<M>` - Container for registered tools, generic over metadata type `M` (defaults to `NoMeta`)
- `ToolsBuilder<S, M>` - Typestate builder for constructing collections with compile-time enforcement
- `CollectionBuilder<M>` - Builder for constructing collections with shared context
- `ToolEntry<M>` - A single tool entry with its function, declaration, and metadata
- `FunctionCall` - Represents a tool invocation with id, name, and arguments
- `FunctionResponse` - Represents the response of a tool invocation with matching id to call, name, and result
- `ToolError` - Comprehensive error type for tool operations
- `NoMeta` - Default metadata type that ignores all `#[tool(...)]` attributes
- `ToolSchema` - Trait for automatic JSON schema generation
- `ToolRegistration` - Internal representation of registered tools
- `FunctionDecl` - LLM-compatible function declaration structure

### Macros

- `#[tool]` - Attribute macro for automatic tool registration. Accepts flat `key = value` attributes for metadata. Detects `ctx` as a reserved first parameter for shared context injection.
- `#[derive(ToolSchema)]` - Derive macro for automatic schema generation

## Error Handling

Tools-rs provides comprehensive error handling with detailed context:

```rust
use tools_rs::{ToolError, collect_tools, FunctionCall};
use serde_json::json;

#[tokio::main]
async fn main() {
    let tools = collect_tools();

    match tools.call(FunctionCall::new(
        "nonexistent".to_string(),
        json!({}),
    )).await {
        Ok(response) => println!("Result: {}", response.result),
        Err(ToolError::FunctionNotFound { name }) => {
            println!("Tool '{}' not found", name);
        },
        Err(ToolError::Deserialize(err)) => {
            println!("Deserialization error: {}", err.source);
        },
        Err(e) => println!("Other error: {}", e),
    }
}
```

## Performance Considerations

### Schema Caching
- JSON schemas are generated once and cached.
- Schema generation has minimal runtime overhead after first access
- Primitive types use pre-computed static schemas for optimal performance

### Tool Discovery
- Tool registration happens at compile-time via the `inventory` crate
- Runtime tool collection (`collect_tools()`) is a zero-cost operation
- Tools are stored in efficient hash maps for O(1) lookup by name

### Execution Performance
- Tool calls have minimal overhead beyond JSON serialization/deserialization
- Async execution allows for concurrent tool invocation
- Error handling uses `Result` types to avoid exceptions and maintain performance

### Memory Usage
- Tool metadata is stored statically with minimal heap allocation
- JSON schemas are shared across all instances of the same type
- Function declarations are generated on-demand and can be cached by the application

### Optimization Tips

```rust
// Reuse ToolCollection instances to avoid repeated discovery
let tools = collect_tools(); // Call once, reuse multiple times

// Generate function declarations once for LLM integration
let declarations = function_declarations()?;
// Cache and reuse declarations across multiple LLM requests

// Use typed parameters to avoid repeated JSON parsing
use tools_rs::call_tool_with;
let result = call_tool_with("my_tool", &my_typed_args).await?.result;
```

## Troubleshooting

### Common Issues

**Tool not found at runtime**
- Ensure the `#[tool]` macro is applied to your function
- Verify the function is in a module that gets compiled (not behind unused feature flags)
- Check that `inventory` is properly collecting tools with `collect_tools()`

**Schema generation errors**
- Ensure all parameter and return types implement `ToolSchema`
- For custom types, add `#[derive(ToolSchema)]` to struct definitions
- Complex generic types may need manual `ToolSchema` implementations

**Deserialization failures**
- Verify JSON arguments match the expected parameter structure
- Check that argument names match function parameter names exactly
- Use `serde` attributes like `#[serde(rename = "...")]` for custom field names

**Async execution issues**
- All tool functions must be `async fn` when using `#[tool]`
- Ensure you're using `tokio` runtime for async execution
- Tool execution is inherently async - use `.await` when calling tools

### Debugging Tips

```rust
// Enable debug logging to see tool registration and execution
use tools_rs::{collect_tools, list_tool_names};

let tools = collect_tools();
println!("Registered tools: {:?}", list_tool_names(&tools));

// Inspect generated schemas
let declarations = tools.json()?;
println!("Function declarations: {}", serde_json::to_string_pretty(&declarations)?);
```

## Scripting Language Tools (FFI)

Tools-rs supports registering tools written in scripting languages alongside
native Rust tools. End users can drop a script into a config folder and have
it available as an LLM tool — no Rust knowledge required.

### Design

- **One language per builder** — no collection gymnastics. Want Python and Lua?
  Build two collections.
- **Package manager agnostic** — we don't run `pip install` or `npm install`.
  Users set up their own environment (venv, node_modules, etc). The adapter
  detects and uses whatever exists.
- **TypeScript = JavaScript** — we accept `.js` files. Users transpile their
  own TypeScript.

```rust
use tools_rs::ToolsBuilder;

// Python tools (requires `python` feature)
let py_tools = ToolsBuilder::new()
    .with_language(Language::Python)
    .from_path("~/.config/myapp/tools/python/")
    .collect()?;

// Lua tools (requires `lua` feature)
let lua_tools = ToolsBuilder::new()
    .with_language(Language::Lua)
    .from_path("~/.config/myapp/tools/lua/")
    .collect()?;
```

### Supported Languages

| Language | Feature | Interpreter | Convention |
|---|---|---|---|
| Python | `python` | pyo3 | `@tool` decorator, type hints, docstrings |
| Lua | `lua` | mlua | `Tool` table with `description`, `params`, `call` |
| JavaScript | `js` | boa_engine | `Tool` object export |

### Python example

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

### Lua example

```lua
Tool = {
    description = "Summarizes text to a maximum length",
    meta = { requires_approval = false },
    params = {
        text = "The text to summarize",
        max_length = "(number) Maximum output length",
    },
}

function Tool.call(args)
    return args.text:sub(1, args.max_length) .. "..."
end
```

### Workspace structure

```
tools-rs/
├── tools_core/             # Language enum, RawToolDef, builder
├── tools_macros/           # #[tool], ToolSchema
├── ffi/
│   ├── tools_python/       # pyo3 adapter
│   ├── tools_lua/          # mlua adapter
│   └── tools_js/           # boa_engine adapter
├── src/                    # tools-rs facade crate
└── examples/
```

## Roadmap

- [x] Core framework — `#[tool]` macro, `ToolCollection`, JSON schema generation
- [x] Tool metadata — `#[tool(key = value)]` with typed `ToolCollection<M>`
- [x] Shared context — `ctx` parameter injection via `ToolCollection::builder()`
- [x] Typestate builder — `ToolsBuilder` with `Blank`/`Native`/`Scripted` states
- [x] Raw registration — `register_raw()` for pre-built schemas
- [ ] FFI foundation — `Language` enum, `RawToolDef`, `from_path` on builder
- [ ] Python adapter — `ffi/tools_python/` with `@tool` decorator
- [ ] Lua adapter — `ffi/tools_lua/` with `Tool` table convention
- [ ] JavaScript adapter — `ffi/tools_js/` with `Tool` object export

## Contributing

We welcome contributions!

### Development Setup

```bash
# Clone the repository
git clone https://github.com/EggerMarc/tools-rs
cd tools-rs 

# Run tests
cargo test

# Run examples
cargo run --example basic
```

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
