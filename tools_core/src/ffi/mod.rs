//! FFI adapter types for scripting-language tool definitions.

#[cfg(feature = "python")]
pub(crate) mod python;

use std::path::Path;

use futures::future::BoxFuture;
use serde_json::Value;

use crate::ToolError;

// ============================================================================
// LANGUAGE ENUM
// ============================================================================

/// Scripting language for FFI tool adapters.
///
/// Each variant is gated behind a cargo feature. The enum is
/// `#[non_exhaustive]` so new languages can be added in minor releases.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Language {
    #[cfg(feature = "python")]
    Python,
    #[cfg(feature = "lua")]
    Lua,
    #[cfg(feature = "js")]
    JavaScript,
}

impl Language {
    /// Human-readable name, used in error messages.
    pub fn name(self) -> &'static str {
        match self {
            #[cfg(feature = "python")]
            Self::Python => "Python",
            #[cfg(feature = "lua")]
            Self::Lua => "Lua",
            #[cfg(feature = "js")]
            Self::JavaScript => "JavaScript",
        }
    }
}

// ============================================================================
// RAW TOOL DEFINITION
// ============================================================================

/// A tool definition produced by an FFI adapter's `load()` function.
///
/// `name` and `description` are owned [`String`]s here; they are leaked
/// to `&'static str` at registration time so that
/// [`register_raw`][crate::ToolCollection::register_raw]'s signature
/// stays unchanged.
pub struct RawToolDef {
    pub name: String,
    pub description: String,
    pub parameters: Value,
    pub meta: Value,
    pub func: Box<
        dyn Fn(Value) -> BoxFuture<'static, Result<Value, ToolError>> + Send + Sync,
    >,
}

// ============================================================================
// LOAD DISPATCH
// ============================================================================

/// Load tool definitions from a path using the given language adapter.
///
/// `Language::Python` loads via the `python` submodule. Lua and
/// JavaScript are not yet implemented. Errors are propagated as
/// [`ToolError::Runtime`].
pub(crate) fn load_language(
    lang: Language,
    #[cfg_attr(
        not(any(feature = "python", feature = "lua", feature = "js")),
        allow(unused_variables)
    )]
    path: &Path,
) -> Result<Vec<RawToolDef>, ToolError> {
    match lang {
        #[cfg(feature = "python")]
        Language::Python => python::load(path),
        #[cfg(feature = "lua")]
        Language::Lua => Err(ToolError::Runtime(format!(
            "Lua language support not yet implemented (path: {})",
            path.display(),
        ))),
        #[cfg(feature = "js")]
        Language::JavaScript => Err(ToolError::Runtime(format!(
            "JavaScript language support not yet implemented (path: {})",
            path.display(),
        ))),
    }
}

// ============================================================================
// HELPERS
// ============================================================================

/// Leak a [`String`] to `&'static str`. Used at the FFI boundary so
/// that tool names and descriptions from scripts become static
/// references compatible with the rest of the API.
pub(crate) fn leak_string(s: String) -> &'static str {
    Box::leak(s.into_boxed_str())
}
