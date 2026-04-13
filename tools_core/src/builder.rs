//! Typestate-based builder for [`ToolCollection`].
//!
//! ```ignore
//! use tools_core::ToolsBuilder;
//! use std::sync::Arc;
//!
//! // Simple — same as collect_tools():
//! let tools = ToolsBuilder::new().collect()?;
//!
//! // With context:
//! let tools = ToolsBuilder::new()
//!     .with_context(Arc::new(my_state))
//!     .with_meta::<MyPolicy>()
//!     .collect()?;
//! ```

use std::{
    any::{Any, TypeId},
    marker::PhantomData,
    path::PathBuf,
    sync::Arc,
};

use serde::de::DeserializeOwned;

use crate::{NoMeta, ToolCollection, ToolError, collect_inventory_inner};
use crate::ffi::{Language, leak_string, load_language};

// ============================================================================
// TYPESTATE MARKERS
// ============================================================================

mod sealed {
    pub trait Sealed {}
}

/// Marker trait for [`ToolsBuilder`] states. Sealed — cannot be implemented
/// outside this crate.
pub trait BuilderState: sealed::Sealed {}

/// Initial state: no context, no FFI adapters configured.
pub struct Blank;

/// Context has been provided. FFI adapter methods are unavailable.
pub struct Native;

/// At least one FFI adapter source has been added. `with_context()` is
/// unavailable. (Transition methods added behind feature flags.)
pub struct Scripted;

impl sealed::Sealed for Blank {}
impl sealed::Sealed for Native {}
impl sealed::Sealed for Scripted {}

impl BuilderState for Blank {}
impl BuilderState for Native {}
impl BuilderState for Scripted {}

// ============================================================================
// BUILDER INTERNALS
// ============================================================================

struct BuilderInner {
    ctx: Option<Arc<dyn Any + Send + Sync>>,
    ctx_type_id: Option<TypeId>,
    ctx_type_name: &'static str,
    language: Option<Language>,
    script_paths: Vec<PathBuf>,
}

impl BuilderInner {
    fn empty() -> Self {
        Self {
            ctx: None,
            ctx_type_id: None,
            ctx_type_name: "",
            language: None,
            script_paths: Vec::new(),
        }
    }
}

// ============================================================================
// TOOLS BUILDER
// ============================================================================

/// Typestate builder for [`ToolCollection`].
///
/// The type parameter `S` tracks the builder state:
///
/// - [`Blank`] — initial. Can call [`with_context`][Self::with_context] or
///   (in the future) FFI adapter methods.
/// - [`Native`] — context was set. FFI methods unavailable.
/// - [`Scripted`] — FFI adapters added. `with_context` unavailable.
///
/// `M` is the metadata type, defaulting to [`NoMeta`]. Change it via
/// [`with_meta`][ToolsBuilder::with_meta].
///
/// # Typestate enforcement
///
/// Calling `with_context` after it has already been called does not compile:
///
/// ```compile_fail
/// use tools_core::builder::{ToolsBuilder, Native};
/// use std::sync::Arc;
///
/// // ERROR: ToolsBuilder<Native, _> has no method `with_context`
/// let b = ToolsBuilder::new()
///     .with_context(Arc::new(42_u32))
///     .with_context(Arc::new(42_u32));
/// ```
pub struct ToolsBuilder<S: BuilderState = Blank, M = NoMeta> {
    inner: BuilderInner,
    _marker: PhantomData<fn() -> (S, M)>,
}

// ── Blank ──────────────────────────────────────────────────────────────

impl ToolsBuilder<Blank, NoMeta> {
    /// Create a new builder in the [`Blank`] state with [`NoMeta`].
    /// Use [`with_meta`][ToolsBuilder::with_meta] to change the metadata
    /// type.
    pub fn new() -> Self {
        Self {
            inner: BuilderInner::empty(),
            _marker: PhantomData,
        }
    }
}

impl Default for ToolsBuilder<Blank, NoMeta> {
    fn default() -> Self {
        Self::new()
    }
}

impl<M> ToolsBuilder<Blank, M> {
    /// Set a shared context that will be injected into every tool whose
    /// first parameter is named `ctx`. Transitions to the [`Native`]
    /// state, locking out FFI adapter methods.
    ///
    /// ```compile_fail
    /// # use tools_core::builder::ToolsBuilder;
    /// # use std::sync::Arc;
    /// // ERROR: with_language not available after with_context
    /// let b = ToolsBuilder::new()
    ///     .with_context(Arc::new(42_u32))
    ///     .with_language(tools_core::Language::Python);
    /// ```
    pub fn with_context<T: Send + Sync + 'static>(
        self,
        ctx: Arc<T>,
    ) -> ToolsBuilder<Native, M> {
        ToolsBuilder {
            inner: BuilderInner {
                ctx: Some(ctx),
                ctx_type_id: Some(TypeId::of::<T>()),
                ctx_type_name: std::any::type_name::<T>(),
                language: None,
                script_paths: Vec::new(),
            },
            _marker: PhantomData,
        }
    }

    /// Set the scripting language for FFI tool loading. Transitions to
    /// the [`Scripted`] state, locking out [`with_context`][Self::with_context].
    ///
    /// Use [`from_path`][ToolsBuilder::<Scripted, M>::from_path] to add
    /// script directories after calling this.
    ///
    /// ```compile_fail
    /// # use tools_core::builder::ToolsBuilder;
    /// # use std::sync::Arc;
    /// // ERROR: with_context not available after with_language
    /// let b = ToolsBuilder::new()
    ///     .with_language(tools_core::Language::Python)
    ///     .with_context(Arc::new(42_u32));
    /// ```
    pub fn with_language(self, lang: Language) -> ToolsBuilder<Scripted, M> {
        ToolsBuilder {
            inner: BuilderInner {
                language: Some(lang),
                script_paths: Vec::new(),
                ctx: None,
                ctx_type_id: None,
                ctx_type_name: "",
            },
            _marker: PhantomData,
        }
    }
}

// ── Any state: with_meta ───────────────────────────────────────────────

impl<S: BuilderState, M> ToolsBuilder<S, M> {
    /// Change the metadata type. This is a phantom-only transition — no
    /// data is stored. `M2` is used at [`collect`] time to deserialize
    /// each tool's `#[tool(...)]` attributes.
    pub fn with_meta<M2>(self) -> ToolsBuilder<S, M2> {
        ToolsBuilder {
            inner: self.inner,
            _marker: PhantomData,
        }
    }
}

// ── collect() per state ────────────────────────────────────────────────

impl<M: DeserializeOwned> ToolsBuilder<Blank, M> {
    /// Build the collection from the global tool inventory (no context).
    /// Tools that require context will produce a [`ToolError::MissingCtx`]
    /// error.
    pub fn collect(self) -> Result<ToolCollection<M>, ToolError> {
        collect_inventory_inner(None, None, "")
    }
}

impl<M: DeserializeOwned> ToolsBuilder<Native, M> {
    /// Build the collection from the global tool inventory, injecting the
    /// stored context into tools that require it. Validates that every
    /// context-requiring tool expects the same type.
    pub fn collect(self) -> Result<ToolCollection<M>, ToolError> {
        collect_inventory_inner(
            self.inner.ctx,
            self.inner.ctx_type_id,
            self.inner.ctx_type_name,
        )
    }
}

// ── Scripted: from_path + collect ───────────────────────────────────

impl<M> ToolsBuilder<Scripted, M> {
    /// Add a script path to load tools from. Can be called multiple
    /// times to load from several paths.
    pub fn from_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.inner.script_paths.push(path.into());
        self
    }
}

impl<M: DeserializeOwned> ToolsBuilder<Scripted, M> {
    /// Build the collection. Collects `#[tool]` inventory tools (no
    /// context), then loads scripted tools from configured paths via
    /// the selected language adapter.
    ///
    /// No paths configured = inventory tools only.
    pub fn collect(self) -> Result<ToolCollection<M>, ToolError> {
        let lang = self
            .inner
            .language
            .expect("Scripted state must have a language set");

        let mut collection: ToolCollection<M> = collect_inventory_inner(None, None, "")?;

        for path in &self.inner.script_paths {
            let defs = load_language(lang, path)?;
            for def in defs {
                let name = leak_string(def.name);
                let desc = leak_string(def.description);
                let meta: M =
                    serde_json::from_value(def.meta).map_err(|e| ToolError::BadMeta {
                        tool: name,
                        error: e.to_string(),
                    })?;
                let func = def.func;
                collection.register_raw(name, desc, def.parameters, move |v| func(v), meta)?;
            }
        }

        Ok(collection)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;
    use serde_json::json;

    #[test]
    fn blank_collect_returns_collection() {
        let tools: ToolCollection = ToolsBuilder::new().collect().unwrap();
        // No panic, collection created. Tool count depends on what's
        // registered via #[tool] in the test binary — just check it works.
        let _ = tools.json().unwrap();
    }

    #[test]
    fn with_meta_changes_type() {
        #[derive(Debug, Default, Deserialize)]
        #[serde(default)]
        struct Policy {
            _flag: bool,
        }

        let tools = ToolsBuilder::new()
            .with_meta::<Policy>()
            .collect()
            .unwrap();

        let _ = tools.json().unwrap();
    }

    #[test]
    fn with_context_then_collect() {
        let ctx = Arc::new(42_u32);
        // No ctx-requiring tools in this test binary, but the builder
        // should still work — context is stored and unused tools are fine.
        let tools: ToolCollection = ToolsBuilder::new()
            .with_context(ctx)
            .collect()
            .unwrap();

        let _ = tools.json().unwrap();
    }

    #[test]
    fn register_raw_works() {
        let mut tools: ToolCollection = ToolsBuilder::new().collect().unwrap();

        tools
            .register_raw(
                "echo",
                "Echoes input back",
                json!({
                    "type": "object",
                    "properties": {
                        "msg": { "type": "string" }
                    },
                    "required": ["msg"]
                }),
                |v| {
                    Box::pin(async move {
                        let msg = v.get("msg").and_then(|m| m.as_str()).unwrap_or("");
                        Ok(serde_json::Value::String(msg.to_string()))
                    })
                },
                (),
            )
            .unwrap();

        let decls = tools.json().unwrap();
        let arr = decls.as_array().unwrap();
        assert!(arr.iter().any(|d| d["name"] == "echo"));
    }

    #[tokio::test]
    async fn register_raw_callable() {
        let mut tools: ToolCollection = ToolsBuilder::new().collect().unwrap();

        tools
            .register_raw(
                "double",
                "Doubles a number",
                json!({
                    "type": "object",
                    "properties": { "n": { "type": "integer" } },
                    "required": ["n"]
                }),
                |v| {
                    Box::pin(async move {
                        let n = v.get("n").and_then(|n| n.as_i64()).unwrap_or(0);
                        Ok(serde_json::Value::Number((n * 2).into()))
                    })
                },
                (),
            )
            .unwrap();

        let resp = tools
            .call(crate::FunctionCall::new(
                "double".to_string(),
                json!({ "n": 21 }),
            ))
            .await
            .unwrap();

        assert_eq!(resp.result, json!(42));
    }

    #[cfg(feature = "python")]
    #[test]
    fn scripted_no_paths_collects_inventory() {
        let tools: ToolCollection = ToolsBuilder::new()
            .with_language(crate::Language::Python)
            .collect()
            .unwrap();

        let _ = tools.json().unwrap();
    }

    #[cfg(feature = "python")]
    #[test]
    fn scripted_with_path_errors_not_implemented() {
        let err = ToolsBuilder::new()
            .with_language(crate::Language::Python)
            .from_path("/some/script.py")
            .collect()
            .err()
            .expect("should error");

        assert!(
            err.to_string().contains("not yet implemented"),
            "expected 'not yet implemented', got: {err}"
        );
    }

    #[cfg(feature = "python")]
    #[test]
    fn scripted_from_path_chainable() {
        let err = ToolsBuilder::new()
            .with_language(crate::Language::Python)
            .from_path("/first.py")
            .from_path("/second.py")
            .collect()
            .err()
            .expect("should error");

        assert!(err.to_string().contains("not yet implemented"));
    }
}
