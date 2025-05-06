//! error.rs – Error types for ToolCollection

use serde_json::Error as JsonError;
use std::borrow::Cow;
use thiserror::Error;

/*───────────────────────────────────────────────────────────────────────────*/

/// Thin wrapper returned by the parsing layer.
/// Keeps the real message but avoids an extra allocation
/// when you can borrow the original `&'static str`.
#[derive(Debug, Error)]
#[error("Parse error: {0}")]
pub struct ParseError(pub Cow<'static, str>);

/// Typed‑argument deserialisation failed (JSON → `T`).
///
/// Carries the full `serde_json::Error` so callers can down‑cast or
/// inspect line/column information.
#[derive(Debug, Error)]
#[error(transparent)]
pub struct DeserializationError(#[from] pub JsonError);

/*───────────────────────────────────────────────────────────────────────────*/

/// All the ways a call into a [`ToolCollection`] can fail.
#[non_exhaustive]
#[derive(Debug, Error)]
pub enum ToolError {
    /// Tried to call a function that was never registered.
    #[error("Function '{name}' not found")]
    FunctionNotFound { name: Cow<'static, str> },

    /// Attempted to register a function under a name that is already taken.
    #[error("Tool '{name}' is already registered")]
    AlreadyRegistered { name: &'static str },

    /// Syntax or token‑stream error while parsing a script / command line.
    #[error(transparent)]
    Parse(#[from] ParseError),

    /// Typed‑argument deserialisation failed (JSON → `T`).
    #[error(transparent)]
    Deserialize(#[from] DeserializationError),

    /// The caller supplied a different number of arguments than expected.
    #[error("Expected {expected} args, got {found}")]
    ArityMismatch { expected: usize, found: usize },

    /// Something inside the user function panicked or bubbled up an error.
    #[error("Runtime error: {0}")]
    Runtime(String),
}
