//! error.rs – Error types for ToolCollection
use std::borrow::Cow;
use thiserror::Error;

/// Thin wrapper returned by the parsing layer.
/// Keeps the real message but avoids an extra allocation
/// when you can borrow the original `&'static str`.
#[derive(Debug, Error)]
#[error("parse error: {0}")]
pub struct ParseError(pub Cow<'static, str>);

/// Thin wrapper returned by the argument‑deserialisation layer.
#[derive(Debug, Error)]
#[error("deserialisation error: {0}")]
pub struct DeserializationError(pub Cow<'static, str>);

/// All the ways a call into a [`ToolCollection`] can fail.
#[derive(Debug, Error)]
pub enum ToolError {
    /// Tried to call a function that was never registered.
    #[error("function '{name}' not found")]
    FunctionNotFound { name: &'static str },

    /// Syntax or token‑stream error while parsing a script / command line.
    #[error(transparent)]
    Parse(#[from] ParseError),

    /// Typed‑argument deserialisation failed (JSON → `T`).
    #[error(transparent)]
    Deserialize(#[from] DeserializationError),

    /// The caller supplied a different number of arguments than expected.
    #[error("expected {expected} args, got {found}")]
    ArityMismatch { expected: usize, found: usize },

    /// Something inside the user function panicked or returned an error
    /// to bubble up as a plain string.
    #[error("runtime error: {0}")]
    Runtime(String),
}
