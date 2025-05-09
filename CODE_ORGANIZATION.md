# Toors Code Organization

This document describes the organization of the Toors codebase following Rust best practices.

## Project Structure

The project is organized as a Rust workspace with multiple crates:

```
toors/
├── Cargo.toml           # Workspace definition and main crate
├── src/                 # Main crate source (toors_core)
│   ├── lib.rs           # Re-exports and high-level API
│   └── main.rs          # Example binary
├── toors/               # Core implementation crate
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs       # Main library code
│       ├── models/      # Core data models
│       │   └── mod.rs   # Type definitions
│       ├── schema/      # Function declaration schemas
│       │   └── mod.rs   # Schema definitions for LLM function-calling
│       ├── error/       # Error handling
│       │   └── mod.rs   # Error types and implementations
│       └── db/          # Database utilities
│           └── mod.rs   # Database-related code
├── toors_macros/        # Procedural macros
│   ├── Cargo.toml
│   └── src/
│       └── lib.rs       # Macro definitions
├── toors_derive/        # Derive macros (optional)
│   ├── Cargo.toml
│   └── src/
│       └── lib.rs       # Derive macro implementations
└── examples/            # Example code
    ├── Cargo.toml
    ├── basic/           # Basic usage examples
    │   └── main.rs      # Simple tool examples
    └── README.md        # Examples documentation
```

## Crate Responsibilities

### toors_core

This is the main crate that users interact with. It:
- Re-exports the most commonly used types and functions
- Provides a simple API for using the tool collection system
- Acts as a convenience layer over the toors crate

### toors

This is the core implementation crate that:
- Defines the core data structures
- Implements the tool collection functionality
- Handles runtime registration and execution of tools
- Manages error handling and type safety

### toors_macros

Provides procedural macros for:
- Tool registration (`#[tool]` attribute)
- Auto-registration with inventory

### toors_derive (optional)

Provides derive macros for:
- Custom trait implementations
- Additional tool-related functionality

## Module Organization

### models

Contains core data structures:
- `FunctionCall`: Represents a function call with name and arguments
- `ToolCollection`: Maintains the registry of tools
- `ToolRegistration`: Information for registering a tool
- `TypeSignature`: Type information for tools

### schema

Contains types for function declaration schemas:
- `FunctionDecl`: Represents a function declaration for LLM function-calling
- `TypeName`: Type representations, either as Rust type names or JSON Schema
- Functions for converting Rust types to their schema representation

### error

Contains error types and handling:
- `ToolError`: Enum of all possible errors
- `DeserializationError`: JSON deserialization failures
- `ParseError`: Input parsing errors

### db (optional)

Database utilities for persistence.

## Best Practices Implemented

1. **Separation of Concerns**: Each crate has a clearly defined responsibility
2. **Module Organization**: Code is organized into logical modules
3. **Public API Design**: Clear distinction between public and internal APIs
4. **Error Handling**: Structured error types with proper error conversion
5. **Type Safety**: Strong typing throughout with JSON conversion at the boundaries
6. **Documentation**: All public items are documented
7. **Examples**: Comprehensive examples of library usage
8. **Feature Flags**: Optional functionality behind feature flags (e.g., `schema`)

## Development Workflow

When adding new features:
1. Determine the appropriate crate for the implementation
2. Add tests in the same module
3. Update documentation
4. Add examples if appropriate

## Versioning

The crates follow semantic versioning:
- Major version: Breaking API changes
- Minor version: New features without breaking changes
- Patch version: Bug fixes and minor improvements

## Further Reading

- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [Rust Book - Package and Crate Structure](https://doc.rust-lang.org/book/ch07-01-packages-and-crates.html)