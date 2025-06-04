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
        .call(FunctionCall {
            name: "add".into(),
            arguments: json!({ "pair": [3, 4] }),
        })
        .await?;
    println!("add → {sum}");  // Outputs: "add → 7"

    let hi = tools
        .call(FunctionCall {
            name: "greet".into(),
            arguments: json!({ "name": "Alice" }),
        })
        .await?;
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
tools-rs = "0.1.0"
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

Tools-rs requires **Rust 1.70** or later and supports:
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
    let mut tools = ToolCollection::new();

    // Register a simple tool manually
    tools.register(
        "multiply",
        "Multiplies two numbers",
        |args: serde_json::Value| async move {
            let a = args["a"].as_i64().unwrap_or(0);
            let b = args["b"].as_i64().unwrap_or(0);
            Ok(json!(a * b))
        }
    )?;

    // Call the manually registered tool
    let result = tools.call(tools_rs::FunctionCall {
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
    let mut tools = ToolCollection::new();

    tools.register(
        "calculate",
        "Performs arithmetic operations on a list of numbers",
        |input: Calculator| async move {
            match input.operation.as_str() {
                "sum" => Ok(input.operands.iter().sum::<f64>()),
                "product" => Ok(input.operands.iter().product::<f64>()),
                "mean" => Ok(input.operands.iter().sum::<f64>() / input.operands.len() as f64),
                _ => Err(format!("Unknown operation: {}", input.operation)),
            }
        }
    )?;

    Ok(())
}
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
```

Each example demonstrates different aspects of the framework:

- **basic**: Simple tool registration with `#[tool]` and basic function calls
- **function_declarations**: Complete LLM integration workflow with JSON schema generation
- **schema**: Advanced schema generation for complex nested types and collections
- **newtype_demo**: Working with custom wrapper types and serialization patterns

## API Reference

### Core Functions

- `collect_tools()` - Discover all tools registered via `#[tool]` macro
- `function_declarations()` - Generate JSON schema declarations for LLMs
- `call_tool(name, args)` - Execute a tool by name with JSON arguments
- `call_tool_with(name, typed_args)` - Execute a tool with typed arguments
- `call_tool_by_name(collection, name, args)` - Execute tool on specific collection
- `list_tool_names(collection)` - List all available tool names

### Core Types

- `ToolCollection` - Container for registered tools with execution capabilities
- `FunctionCall` - Represents a tool invocation with name and arguments
- `ToolError` - Comprehensive error type for tool operations
- `ToolSchema` - Trait for automatic JSON schema generation
- `ToolRegistration` - Internal representation of registered tools
- `FunctionDecl` - LLM-compatible function declaration structure

### Macros

- `#[tool]` - Attribute macro for automatic tool registration
- `#[derive(ToolSchema)]` - Derive macro for automatic schema generation

## Error Handling

Tools-rs provides comprehensive error handling with detailed context:

```rust
use tools_rs::{ToolError, collect_tools, FunctionCall};
use serde_json::json;

#[tokio::main]
async fn main() {
    let tools = collect_tools();

    match tools.call(FunctionCall {
        name: "nonexistent".to_string(),
        arguments: json!({}),
    }).await {
        Ok(result) => println!("Result: {}", result),
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
let result = call_tool_with("my_tool", &my_typed_args).await?;
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

## Contributing

We welcome contributions!

### Development Setup

```bash
# Clone the repository
git clone https://github.com/EggerMarc/toors.git
cd toors

# Run tests
cargo test

# Run examples
cargo run --example basic
```

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
