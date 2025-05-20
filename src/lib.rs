//! toors_core – Lightweight façade for the **Toors** runtime.
//!
//! * Re-exports the primary APIs from the `toors` crate so downstream users
//!   only depend on a single public crate.
//! * Adds a couple of thin helpers (`collect_tools`, `function_declarations`).
//!
//! When compiled **with** the Cargo-feature `schema`, every `FunctionDecl`
//! contains a full JSON-Schema (courtesy of *schemars*).  Without the feature
//! those fields are `null`, but the public surface stays identical.

pub use tools::{
    schema::{schema_to_json_schema, FunctionDecl},
    FunctionCall, ToolCollection, ToolError, ToolRegistration, TypeSignature,
};

pub use tools_macros::tool;

/// Collect every tool that was *registered at compile-time* through the
/// `#[tool]` macro and return a live [`ToolCollection`].
#[inline]
pub fn collect_tools() -> ToolCollection {
    ToolCollection::collect_tools()
}

/// Convenience wrapper around [`collect_tools`], returning the JSON array that
/// can be pasted straight into the `tools` / `functionDeclarations` field of
/// an OpenAI / Gemini chat-completion request.
#[inline]
pub fn function_declarations() -> Result<serde_json::Value, ToolError> {
    // ◀ now Result
    collect_tools().json() // ◀ bubbles `?` inside
}
// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tool]
    /// Adds two numbers.
    async fn add(a: i32, b: i32) -> i32 {
        a + b
    }

    #[tokio::test]
    async fn tool_macro_and_collection() {
        let hub = collect_tools();

        let result = hub
            .call(FunctionCall {
                name: "add".into(),
                arguments: json!({ "a": 5, "b": 7 }),
            })
            .await
            .unwrap();

        assert_eq!(result, json!(12));
    }
}
