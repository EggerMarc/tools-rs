//! toors_core: Top-level crate for the Toors tool collection system
//!
//! This crate provides a convenient entry point for using the Toors tool collection system.
//! It re-exports the main components from the `toors` crate and provides additional
//! convenience functionality.

pub use toors::{
    schema::{type_to_decl, FunctionDecl, TypeName},
    FunctionCall, ToolCollection, ToolRegistration, TypeSignature,
};
pub use toors_macros::tool;

/// Collects all registered tools and returns a tool collection.
///
/// This is a convenience function that calls `ToolCollection::collect_tools()`.
pub fn collect_tools() -> ToolCollection {
    ToolCollection::collect_tools()
}

/// Export all registered tools as a JSON array ready for "functionDeclarations".
///
/// This is a convenience function that calls `collect_tools().json()`.
pub fn function_declarations() -> serde_json::Value {
    collect_tools().json()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tool]
    /// Test function that adds two numbers.
    async fn add(pair: (i32, i32)) -> i32 {
        pair.0 + pair.1
    }

    #[tokio::test]
    async fn test_tool_macro_and_collection() {
        let hub = collect_tools();

        let result = hub
            .call(FunctionCall {
                name: "add".into(),
                arguments: json!((5, 7)),
            })
            .await
            .unwrap();

        assert_eq!(result.to_string(), "12");
    }
}

