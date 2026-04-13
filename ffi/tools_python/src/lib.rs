//! Python language adapter for tools-rs FFI.
//!
//! Loads Python scripts from a directory, discovers functions decorated
//! with `@tool`, and returns [`PyToolDef`]s ready for conversion to
//! `RawToolDef` by `tools_core`.

mod decorator;

use std::{future::Future, path::Path, pin::Pin};

use serde_json::Value;

/// Boxed future type alias (avoids pulling in `futures` crate).
pub type BoxFuture<T> = Pin<Box<dyn Future<Output = T> + Send + 'static>>;

/// A tool definition extracted from a Python script.
///
/// This mirrors `tools_core::RawToolDef` but lives in this crate to
/// avoid a cyclic dependency. `tools_core` converts these into
/// `RawToolDef` when the `python` feature is enabled.
pub struct PyToolDef {
    pub name: String,
    pub description: String,
    pub parameters: Value,
    pub meta: Value,
    pub func: Box<dyn Fn(Value) -> BoxFuture<Result<Value, String>> + Send + Sync>,
}

/// Load all `@tool`-decorated functions from `*.py` files in `dir`.
pub fn load(dir: &Path) -> Result<Vec<PyToolDef>, String> {
    let _ = dir;
    Err("Python adapter: load() not yet implemented".into())
}
