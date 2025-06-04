# Toors Code Organization

This document describes the organization of the Toors codebase following Rust best practices.

## Project Structure

The project is organized as a Rust workspace with two main crates:

```
toors/
├── Cargo.toml              # Workspace definition and main crate
├── src/                    # Main crate source (tools-rs)
│   └── lib.rs              # Re-exports and high-level API
├── tools_core/             # Core implementation crate
│   ├── Cargo.toml
│   └── src/
│       └── lib.rs          # Runtime functionality and schema trait
├── tools_macros/           # Procedural macros crate
│   ├── Cargo.toml
│   └── src/
│       └── lib.rs          # All procedural macros
├── examples/               # Example code (separate crate)
│   ├── Cargo.toml
│   ├── README.md
│   ├── basic/              # Basic usage examples
│   │   └── main.rs         # Simple tool registration and usage
│   ├── function_declarations/ # Function declaration examples
│   │   └── main.rs         # LLM integration examples
│   └── schema/             # Schema generation examples
│       └── main.rs         # Advanced schema usage
└── tests/                  # Integration tests
    └── no_features.rs      # Test schema generation without features
```

## Crate Responsibilities

### tools-rs (root crate)

This is the main crate that users interact with. It:
- Re-exports the most commonly used types and functions from `tools_core`
- Re-exports both macros from `tools_macros` (`tool` and `ToolSchema`)
- Provides a simple API via `collect_tools()` and `function_declarations()`
- Acts as a convenience layer over the core `tools_core` crate

### tools_core

This is the core implementation crate that contains:
- **Tool Runtime**: `ToolCollection` for managing and executing registered tools
- **Schema Generation**: `ToolSchema` trait and implementations for all primitive types
- **Error Handling**: Comprehensive error types (`ToolError`, `DeserializationError`)
- **Core Models**: `FunctionCall`, `ToolRegistration`, `FunctionDecl`, etc.
- **Async Execution**: Type-safe tool invocation with JSON serialization
- **Inventory Integration**: Runtime collection of tools registered via macros

### tools_macros

This is the procedural macro crate that provides:
- **`#[derive(ToolSchema)]`**: Automatic JSON Schema generation for structs
  - Supports named structs, tuple structs, and unit structs
  - Handles nested types and collections
  - Detects optional fields (`Option<T>`) for schema generation
- **`#[tool]`**: Attribute macro for automatic tool registration
  - Generates wrapper structs for function parameters
  - Integrates with the `inventory` crate for compile-time tool collection
  - Supports async functions with automatic JSON conversion

## Design Rationale

The 2-crate structure follows Rust best practices:

### Separation of Concerns
- **Runtime vs. Compile-time**: `tools_core` handles runtime functionality while `tools_macros` provides compile-time code generation
- **Proc-macro Isolation**: Procedural macros require `proc-macro = true` and have different compilation requirements, so they belong in a separate crate

### Dependency Management
- **Minimal Dependencies**: `tools_core` only includes runtime dependencies (serde, tokio, etc.)
- **Proc-macro Dependencies**: `tools_macros` includes proc-macro specific dependencies (syn, quote, proc-macro2)
- **No Circular Dependencies**: The macros reference the core crate, but not vice versa

### User Experience
- **Single Entry Point**: Users import from `tools-rs` and get everything they need
- **Flexible Usage**: Advanced users can depend directly on `tools_core` or `tools_macros` if needed
- **Clear API**: The macro and trait names are consistent and intuitive

## Module Organization

### tools_core modules
- **Root**: Core trait definitions (`ToolSchema`) and implementations
- **Error handling**: `ToolError`, `DeserializationError` with proper error chaining
- **Models**: Data structures for function calls, registrations, and metadata
- **Schema generation**: `FunctionDecl` for LLM consumption
- **Tool collection**: `ToolCollection` with registration and execution logic

### tools_macros modules
- **Derive macro**: `ToolSchema` implementation generation
- **Attribute macro**: `#[tool]` for automatic registration
- **Utilities**: Helper functions for path resolution and type analysis

## Rust Best Practices Implemented

1. **Clear Ownership**: Each crate has a single, well-defined responsibility
2. **Minimal API Surface**: Users only need to import from one crate
3. **Type Safety**: Strong typing with JSON conversion at boundaries
4. **Error Handling**: Comprehensive error types with proper context
5. **Async-First Design**: Built around tokio and async/await patterns
6. **Macro Hygiene**: Proper path resolution and name collision avoidance
7. **Workspace Management**: Shared dependencies and consistent versioning
8. **Documentation**: Clear examples and comprehensive tests

## Development Workflow

### Adding New Features

1. **New Tools**: Use `#[tool]` attribute on async functions
2. **Core Changes**: Modify `tools_core` for fundamental functionality
3. **Schema Changes**: Update `tools_core` for new type support
4. **Macro Changes**: Update `tools_macros` for new derive capabilities
5. **Examples**: Add to the `examples` crate for documentation

### Testing Strategy

- **Unit Tests**: Each crate has its own test suite
- **Integration Tests**: Workspace-level tests verify end-to-end functionality
- **Example Tests**: Examples serve as both documentation and integration tests

## Migration from Previous Structure

The reorganization consolidated 4 crates into 2:

**Before:**
- `tool_schema` → **Merged into `tools_core`**
- `tool_schema_derive` → **Merged into `tools_macros`**
- `tools` → **Became `tools_core`**
- `tools_macros` → **Merged into `tools_macros`**

**Benefits:**
- Reduced complexity from 4 interdependent crates to 2 focused crates
- Eliminated confusion about which crate provides which functionality
- Simplified dependency management and circular dependency issues
- Better alignment with Rust ecosystem conventions

## Usage Patterns

### Basic Usage
```rust
use tools_rs::{tool, ToolSchema};

#[derive(serde::Serialize, serde::Deserialize, ToolSchema)]
struct MyInput {
    value: String,
}

#[tool]
async fn my_function(input: MyInput) -> String {
    format!("Hello, {}!", input.value)
}
```

### LLM Integration
```rust
let tools = tools_rs::collect_tools();
let declarations = tools_rs::function_declarations()?;
// Send declarations to LLM, receive function calls, execute with tools.call()
```

### Manual Registration
```rust
use tools_core::ToolCollection;

let mut collection = ToolCollection::new();
collection.register("name", "description", |input: String| async move {
    // tool implementation
}).unwrap();
```

## Versioning Strategy

The workspace uses semantic versioning:
- **Major version**: Breaking API changes in public interfaces
- **Minor version**: New features maintaining backward compatibility  
- **Patch version**: Bug fixes and internal improvements

Both `tools_core` and `tools_macros` follow the same version as the main `tools-rs` crate to ensure compatibility.