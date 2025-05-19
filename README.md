# Toors - Tool Runtime System for Rust

[![Crates.io](https://img.shields.io/crates/v/toors_core.svg)](https://crates.io/crates/toors_core)
[![Documentation](https://docs.rs/toors_core/badge.svg)](https://docs.rs/toors_core)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

Toors is a minimal, fully-typed, JSON-driven runtime for Large Language Model (LLM) function-calling in Rust.

> **Note**: This codebase has been reorganized following Rust best practices. See [CODE_ORGANIZATION.md](CODE_ORGANIZATION.md) for details.

## Features

- **Simplicity** - Just JSON in, JSON out
- **Type safety** - Input/Output generics checked at compile-time; run-time reflection via `TypeId`
- **Async-first** - All tools are executed as `Future`s; no blocking
- **Extensibility** - Proc-macro auto-registration, pluggable error model
- **LLM Integration** - Export function declarations for LLM function calling APIs

## Quick Start

```rust
use serde_json::json;
use toors_core::{collect_tools, FunctionCall, tool};

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
    let hub = collect_tools();

    let sum = hub
        .call(FunctionCall {
            name: "add".into(),
            arguments: json!([3, 4]),
        })
        .await?;
    println!("add → {sum}");  // Outputs: "add → 7"

    let hi = hub
        .call(FunctionCall {
            name: "greet".into(),
            arguments: json!("Alice"),
        })
        .await?;
    println!("greet → {hi}");  // Outputs: "greet → Hello, Alice!"

    // Export function declarations for LLM APIs
    let declarations = hub.json();
    println!("Function declarations: {}", serde_json::to_string_pretty(&declarations)?);

    Ok(())
}
```

## Crate Structure

The Toors system is organized into several crates following Rust best practices:

- **toors_core**: Main entry point, re-exports the most commonly used items
- **toors**: Core runtime implementation with modular organization:
  - `models`: Core data structures
  - `error`: Error types and handling
  - `schema`: Function declaration schemas
  - `db`: Database utilities (optional)
- **toors_macros**: Procedural macros for tool registration

For more details about the codebase organization, see [CODE_ORGANIZATION.md](CODE_ORGANIZATION.md).

## Installation

Add the following to your `Cargo.toml`:

```toml
[dependencies]
toors_core = "0.1.0"
```

## Why This Crate Exists

LLMs can emit a *function-call* intent instead of free-form text. The host application must then **deserialize**, **dispatch**, and **serialize** the result **safely**. `toors` provides exactly that glue while retaining Rust's *zero-cost abstractions* and type system.

### Function Declarations for LLMs

Toors can automatically generate function declarations suitable for LLM APIs:

```rust
use toors_core::{function_declarations, tool};

#[tool]
/// Return the current date in ISO-8601 format.
async fn today(_: ()) -> String {
    chrono::Utc::now().date_naive().to_string()
}

#[tokio::main]
async fn main() {
    // Generate function declarations for an LLM
    let declarations = function_declarations();
    
    // Use in API request
    let llm_request = serde_json::json!({
        "model": "gpt-4-turbo",
        "messages": [/* ... */],
        "functionDeclarations": declarations
    });
}
```

The generated declarations will include function name, description, parameters and return type:

```json
[
  {
    "name": "today",
    "description": "Return the current date in ISO-8601 format.",
    "parameters": [{ "rust": "()" }],
    "returns": { "rust": "alloc::string::String" }
  }
]
```

With the `schema` feature enabled, this becomes full JSON Schema:

```toml
[dependencies]
toors_core = { version = "0.1.0", features = ["schema"] }
```

For a complete example of using JSON Schema with complex types, see the [schema example](examples/schema/main.rs).

## Examples

Check out the [examples directory](examples/) for sample code showing how to use the Toors library:

```bash
# Run the basic example
cargo run --example basic

# Run the function declarations example
cargo run --example function_declarations

# Run the schema example (requires the schema feature)
cargo run --example schema
```

## Code Organization

The codebase follows modern Rust best practices with:

- Clear separation of concerns through modular design
- Proper error handling with typed errors
- Well-documented public APIs
- Comprehensive test coverage
- Example-driven development
- Feature flags for optional functionality

See [CODE_ORGANIZATION.md](CODE_ORGANIZATION.md) for a detailed explanation of the code structure.

## License

This project is licensed under the MIT License - see the LICENSE file for details.
