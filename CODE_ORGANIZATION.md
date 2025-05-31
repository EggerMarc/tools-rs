# Tools-rs Code Organization

This document describes the organization of the Tools-rs codebase following Rust best practices.

## Project Structure

The project is organized as a Rust workspace with multiple crates:

```
toors/
├── Cargo.toml              # Workspace definition and main crate
├── src/                    # Main crate source (tools-rs)
│   └── lib.rs              # Re-exports and high-level API
├── tools/                  # Core implementation crate
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs          # Main library code with ToolCollection
│       ├── models/         # Core data models
│       │   └── mod.rs      # FunctionCall, Tool, ToolRegistration types
│       ├── schema/         # Function declaration schemas
│       │   └── mod.rs      # Schema generation for LLM function-calling
│       └── error/          # Error handling
│           └── mod.rs      # ToolError and related error types
├── tools_macros/           # Procedural macros
│   ├── Cargo.toml
│   └── src/
│       └── lib.rs          # #[tool] attribute macro
├── tool_schema/            # JSON Schema generation
│   ├── Cargo.toml
│   └── src/
│       └── lib.rs          # ToolSchema trait and core functionality
├── tool_schema_derive/     # Derive macros for schema generation
│   ├── Cargo.toml
│   └── src/
│       └── lib.rs          # ToolSchema derive macro
└── examples/               # Example code (separate crate)
    ├── Cargo.toml
    ├── README.md
    ├── basic/              # Basic usage examples
    │   └── main.rs         # Simple tool registration and usage
    ├── function_declarations/ # Function declaration examples
    │   └── main.rs         # LLM integration examples
    └── schema/             # Schema generation examples
        └── main.rs         # Advanced schema usage
```

## Crate Responsibilities

### tools-rs (root crate)

This is the main crate that users interact with. It:
- Re-exports the most commonly used types and functions from `tools`
- Provides a simple API via `collect_tools()` and `function_declarations()`
- Acts as a convenience layer over the core `tools` crate
- Supports optional schema generation via the `schema` feature

### tools

This is the core implementation crate that:
- Defines the `ToolCollection` for managing registered tools
- Implements async tool execution with JSON serialization
- Handles runtime registration and type-safe tool invocation
- Manages error handling and provides comprehensive error types
- Supports both manual registration and macro-based auto-registration

### tools_macros

Provides procedural macros for:
- `#[tool]` attribute macro for automatic tool registration
- Generates wrapper structs for function parameters
- Integrates with the `inventory` crate for compile-time tool collection
- Supports both schema-enabled and schema-disabled builds

### tool_schema

Lightweight JSON Schema generation crate that:
- Defines the `ToolSchema` trait for types that can generate schemas
- Provides schema generation functionality for LLM integration
- Can be disabled via feature flags for minimal builds

### tool_schema_derive

Procedural macro crate that:
- Implements `#[derive(ToolSchema)]` for automatic schema generation
- Generates JSON Schema representations of Rust types
- Supports complex nested types and collections

### examples

Separate crate containing comprehensive examples:
- **basic**: Simple tool registration and execution
- **function_declarations**: LLM integration patterns
- **schema**: Advanced schema generation usage

## Module Organization

### tools::models

Contains core data structures:
- `FunctionCall`: Represents a function call with name and JSON arguments
- `ToolCollection`: Registry and executor for tools
- `ToolRegistration`: Compile-time tool registration information
- `ToolFunc`: Type alias for async tool function signature
- `TypeSignature`: Runtime type information for tools

### tools::schema

Contains schema generation functionality:
- `FunctionDecl`: Function declaration for LLM consumption
- `schema_to_json_schema()`: Converts Rust types to JSON Schema
- Integration with the optional `tool_schema` crate

### tools::error

Comprehensive error handling:
- `ToolError`: Main error enum covering all failure modes
- `DeserializationError`: JSON deserialization failures
- `SerializationError`: JSON serialization failures
- `ParseError`: Input parsing errors

## Feature Flags

The codebase uses feature flags for optional functionality:

- **`schema`**: Enables JSON Schema generation for LLM integration
  - Pulls in `tool_schema` dependency
  - Enables schema generation in tools and examples
  - When disabled, schema functions return `null`

## Workspace Dependencies

Dependencies are centralized in the workspace root:
- **`serde`**: JSON serialization with derive support
- **`serde_json`**: JSON value handling
- **`tokio`**: Async runtime with sync primitives
- **`inventory`**: Compile-time registration collection
- **`chrono`**: Date/time handling (examples only)

## Best Practices Implemented

1. **Separation of Concerns**: Each crate has a clearly defined responsibility
2. **Feature-Gated Functionality**: Optional schema generation reduces dependencies
3. **Workspace Organization**: Shared dependencies and consistent versioning
4. **Type Safety**: Strong typing with JSON conversion at boundaries
5. **Async-First Design**: Built around tokio and async/await
6. **Comprehensive Error Handling**: Structured error types with proper conversions
7. **Macro-Driven API**: Simple `#[tool]` attribute for tool registration
8. **LLM Integration**: First-class support for function calling workflows

## Development Workflow

When adding new features:

1. **New Tools**: Use `#[tool]` attribute on async functions
2. **Core Changes**: Modify the `tools` crate for fundamental functionality
3. **Schema Changes**: Update `tool_schema` for new type support
4. **Examples**: Add to the `examples` crate for documentation
5. **Testing**: Use the comprehensive test suite in each crate

## Versioning Strategy

The workspace uses semantic versioning:
- **Major version**: Breaking API changes in public interfaces
- **Minor version**: New features maintaining backward compatibility
- **Patch version**: Bug fixes and internal improvements

## Integration Patterns

### Basic Usage
```rust
use tools_rs::tool;

#[tool]
async fn my_function(input: String) -> String {
    format!("Hello, {}!", input)
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
let mut collection = ToolCollection::new();
collection.register("name", "description", |input: String| async move {
    // tool implementation
}).unwrap();

collection.register("name2", "description2", |input: String| async move {
    // tool implementation
}).unwrap();
```
