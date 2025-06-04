//! # Tools-rs: A Tool Collection and Execution Framework
//!
//! Tools-rs provides a framework for building, registering, and executing tools
//! with automatic JSON schema generation for LLM integration.
//!
//! ## Quick Start
//!
//! ```rust
//! use tools_rs::{tool, collect_tools, call_tool_with_args};
//!
//! #[tool]
//! /// Adds two numbers together
//! async fn add(a: i32, b: i32) -> i32 {
//!     a + b
//! }
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let tools = collect_tools();
//!     let result = call_tool_with_args(&tools, "add", &[1, 2]).await?;
//!     println!("Result: {}", result);
//!     Ok(())
//! }
//! ```
//!
//! ## Features
//!
//! - **Automatic Registration**: Use `#[tool]` to automatically register functions
//! - **JSON Schema Generation**: Automatic schema generation for LLM integration
//! - **Type Safety**: Full type safety with JSON serialization at boundaries
//! - **Async Support**: Built for async/await from the ground up
//! - **Error Handling**: Comprehensive error types with context
//!
//! ## Manual Registration
//!
//! ```rust
//! use tools_rs::ToolCollection;
//!
//! # fn example() -> Result<(), tools_rs::ToolError> {
//! let mut tools = ToolCollection::new();
//! tools.register("greet", "Greets a person", |name: String| async move {
//!     format!("Hello, {}!", name)
//! })?;
//! # Ok(())
//! # }
//! ```

// Re-export core functionality
pub use tools_core::{
    DeserializationError, FunctionCall, FunctionDecl, ToolCollection, ToolError, ToolMetadata,
    ToolRegistration, TypeSignature,
};

// Re-export schema functionality (trait from tools_core)
pub use tools_core::ToolSchema;

// Re-export macros (both tool attribute and ToolSchema derive)
pub use tools_macros::{ToolSchema, tool};

/// Convenient imports for common usage patterns.
///
/// Import everything you typically need with:
/// ```rust
/// use tools_rs::prelude::*;
/// ```
pub mod prelude;

/// Collect all tools registered via the `#[tool]` macro.
///
/// This function discovers all tools that were registered at compile time
/// using the `#[tool]` attribute macro.
///
/// # Example
///
/// ```rust
/// use tools_rs::{collect_tools, list_tool_names};
///
/// let tools = collect_tools();
/// println!("Available tools: {:?}", list_tool_names(&tools));
/// ```
#[inline]
pub fn collect_tools() -> ToolCollection {
    ToolCollection::collect_tools()
}

/// Generate function declarations in JSON format for LLM consumption.
///
/// This is equivalent to `collect_tools().json()` but provides a more
/// convenient API for the common use case of generating LLM-compatible
/// function declarations.
///
/// # Example
///
/// ```rust
/// use tools_rs::function_declarations;
///
/// let declarations = function_declarations()?;
/// // Send to LLM for function calling
/// # Ok::<(), tools_rs::ToolError>(())
/// ```
#[inline]
pub fn function_declarations() -> Result<serde_json::Value, ToolError> {
    collect_tools().json()
}

/// Call a tool by name with JSON arguments.
///
/// This is a convenience function that combines tool collection and execution
/// in a single call. Useful for simple scenarios where you don't need to
/// manage the tool collection yourself.
///
/// # Arguments
///
/// * `name` - The name of the tool to call
/// * `arguments` - JSON value containing the arguments
///
/// # Example
///
/// ```rust
/// use tools_rs::call_tool;
/// use serde_json::json;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let result = call_tool("add", json!({"a": 1, "b": 2})).await?;
/// # Ok(())
/// # }
/// ```
pub async fn call_tool(
    name: &str,
    arguments: serde_json::Value,
) -> Result<serde_json::Value, ToolError> {
    let tools = collect_tools();
    let call = FunctionCall {
        name: name.to_string(),
        arguments,
    };
    tools.call(call).await
}

/// Call a tool by name with typed arguments.
///
/// This function provides a more ergonomic API for calling tools when you
/// have typed arguments that can be serialized to JSON.
///
/// # Arguments
///
/// * `name` - The name of the tool to call
/// * `args` - Arguments that implement `serde::Serialize`
///
/// # Example
///
/// ```rust
/// use tools_rs::call_tool_with;
/// use serde::Serialize;
///
/// #[derive(Serialize)]
/// struct AddArgs { a: i32, b: i32 }
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let result = call_tool_with("add", &AddArgs { a: 1, b: 2 }).await?;
/// # Ok(())
/// # }
/// ```
pub async fn call_tool_with<T: serde::Serialize>(
    name: &str,
    args: &T,
) -> Result<serde_json::Value, ToolError> {
    let arguments = serde_json::to_value(args)
        .map_err(|e| ToolError::Runtime(format!("Failed to serialize arguments: {}", e)))?;
    call_tool(name, arguments).await
}

/// Call a tool by name with JSON arguments on a given collection.
///
/// # Example
///
/// ```rust
/// use tools_rs::{collect_tools, call_tool_by_name};
/// use serde_json::json;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let tools = collect_tools();
/// let result = call_tool_by_name(&tools, "add", json!([1, 2])).await?;
/// # Ok(())
/// # }
/// ```
pub async fn call_tool_by_name(
    collection: &ToolCollection,
    name: &str,
    arguments: serde_json::Value,
) -> Result<serde_json::Value, ToolError> {
    let call = FunctionCall {
        name: name.to_string(),
        arguments,
    };
    collection.call(call).await
}

/// Call a tool by name with typed arguments on a given collection.
///
/// # Example
///
/// ```rust
/// use tools_rs::{collect_tools, call_tool_with_args};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let tools = collect_tools();
/// let result = call_tool_with_args(&tools, "add", &[1, 2]).await?;
/// # Ok(())
/// # }
/// ```
pub async fn call_tool_with_args<T: serde::Serialize>(
    collection: &ToolCollection,
    name: &str,
    args: &T,
) -> Result<serde_json::Value, ToolError> {
    let arguments = serde_json::to_value(args)
        .map_err(|e| ToolError::Runtime(format!("Failed to serialize arguments: {}", e)))?;
    call_tool_by_name(collection, name, arguments).await
}

/// List all available tool names in a collection.
///
/// # Example
///
/// ```rust
/// use tools_rs::{collect_tools, list_tool_names};
///
/// let tools = collect_tools();
/// let names = list_tool_names(&tools);
/// println!("Available tools: {:?}", names);
/// ```
pub fn list_tool_names(collection: &ToolCollection) -> Vec<&'static str> {
    collection.descriptions().map(|(name, _)| name).collect()
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_prelude_exports() {
        // Ensure prelude exports don't cause compilation errors
        use crate::prelude::*;
        let _tools = collect_tools();
    }
}
