//! Convenient re-exports for common usage patterns.
//!
//! This prelude module re-exports the most commonly used items from tools-rs,
//! allowing users to import everything they typically need with a single use statement:
//!
//! ```rust
//! use tools_rs::prelude::*;
//! ```

// Core functionality
pub use crate::{
    call_tool, call_tool_by_name, call_tool_with, call_tool_with_args, collect_tools,
    function_declarations, list_tool_names,
};

// Essential types
pub use crate::{FunctionCall, FunctionDecl, ToolCollection, ToolError, ToolMetadata, ToolSchema};

// Macros
pub use crate::tool;

// Commonly used external types
pub use serde_json::{Value, json};

// Re-export commonly needed traits for doc examples
pub use serde::{Deserialize, Serialize};

// Re-export async runtime for examples
pub use tokio;
