pub use tools::{FunctionCall, ToolCollection, ToolError};
pub use tools_macros::tool;

#[inline]
pub fn collect_tools() -> ToolCollection {
    ToolCollection::collect_tools()
}

#[inline]
pub fn function_declarations() -> Result<serde_json::Value, ToolError> {
    collect_tools().json()
}
