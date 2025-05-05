//! lib/toors.rs
//! ============================================================
//! **Toors Runtime** – A minimal, fully‑typed, JSON‑driven
//! runtime for Large‑Language‑Model (LLM) *function‑calling* in Rust.
//!
//! ```text
//! Crate      : toors
//! Version    : 0.1.x (API‑stable)
//! License    : ???
//! Last update: 2025‑05‑05
//! ```
//!
//! ## 1  Why this crate exists
//! LLMs can emit a *function‑call* intent instead of free‑form text.  The host
//! application must then **deserialize**, **dispatch**, and **serialize** the
//! result **safely**.  `toors` provides exactly that glue while
//! retaining Rust’s *zero‑cost abstractions* and type system.
//!
//! ### Design pillars
//! 1. **Simplicity** – Just JSON in, JSON out.
//! 2. **Type safety** – Input/Output generics checked at compile‑time; run‑time
//!    reflection via `TypeId`.
//! 3. **Async‑first** – All tools are executed as `Future`s; no blocking.
//! 4. **Extensibility** – Proc‑macro auto‑registration, pluggable error model.
//!
//! ---------------------------------------------------------------------------
//! The remainder of this file contains the *implementation* with rich inline
//! documentation.
//! ---------------------------------------------------------------------------

pub mod toors_errors;

use std::{any::TypeId, borrow::Cow, collections::HashMap, sync::Arc};

use futures::{future::BoxFuture, FutureExt};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::Value;

use toors_errors::{DeserializationError, ToolError};

/* ───────────────────────────── PUBLIC TYPES ────────────────────────── */

/// An envelope emitted by an LLM (or any router) when it *intends* to invoke a
/// Rust tool.
///
/// ## JSON shape
/// ```json
/// {
///   "name": "add",
///   "arguments": { "a": 1, "b": 2 }
/// }
/// ```
#[derive(Debug, Deserialize)]
pub struct FunctionCall {
    /// Identifier of the tool to run.  Must match one of the keys registered
    /// in [`ToolCollection`].
    pub name: String,

    /// Raw JSON arguments exactly as provided by the LLM.  The registry will
    /// attempt to deserialize this into the input type `I` of the tool.
    pub arguments: Value,
}

/// Type‑erased async function pointer stored inside the registry.
///
/// * **Input** : a `serde_json::Value` payload (raw JSON).
/// * **Output** : `Result<Value, ToolError>` returned in a boxed `Future`.
///
/// Consumers generally interact through the safe [`ToolCollection`] API and
/// never manipulate this alias directly.
pub type ToolFunc = dyn Fn(Value) -> BoxFuture<'static, Result<Value, ToolError>> + Send + Sync;

/// Central registry mapping *tool names* to async callables plus documentation
/// and signature metadata.
///
/// ### Cloning & Sharing
/// The struct contains only `Arc`‑wrapped callables and immutable maps – it is
/// **cheap to clone** (`O(1)` pointer bump) and is `Send + Sync`, so it can be
/// stored inside a web‑server state container.
#[derive(Default)]
pub struct ToolCollection {
    funcs: HashMap<&'static str, Arc<ToolFunc>>,
    descriptions: HashMap<&'static str, &'static str>,
    signatures: HashMap<&'static str, (TypeId, TypeId)>,
}

/* ───────────────────────────── IMPLEMENTATION ──────────────────────── */

impl ToolCollection {
    /// Construct an empty registry.
    ///
    /// # Example
    /// ```rust
    /// use tool_collection::ToolCollection;
    /// let tools = ToolCollection::new();
    /// assert_eq!(tools.descriptions().count(), 0);
    /// ```
    pub fn new() -> Self {
        Self::default()
    }

    /*──────────── fluent `register` – returns &mut Self ───────────────*/
    /// Register a **single** Rust function as an LLM‑callable *tool*.
    ///
    /// # Type Parameters
    /// * `I` – Input type expected from the JSON payload (must implement
    ///   [`DeserializeOwned`]).
    /// * `O` – Output type returned by the function (must implement
    ///   [`Serialize`]).
    /// * `F` – The Rust function/closure.
    /// * `Fut` – The future returned by `F` (auto‑deduced).
    ///
    /// # Errors
    /// This method never fails; errors surface only when the tool is *called*.
    ///
    /// # Panics
    /// Panics if another tool is already registered under the same `name`.
    ///
    /// # Example
    /// ```rust
    /// use tool_collection::ToolCollection;
    /// let mut hub = ToolCollection::new();
    /// hub.register("echo", "Returns its input", |s: String| async move { s });
    /// ```
    pub fn register<I, O, F, Fut>(
        &mut self,
        name: &'static str,
        desc: &'static str,
        func: F,
    ) -> &mut Self
    where
        I: 'static + DeserializeOwned + Send,
        O: 'static + Serialize + Send,
        F: Fn(I) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = O> + Send + 'static,
    {
        /* 1. store human‑readable metadata */
        if self.funcs.contains_key(name) {
            panic!("tool '{name}' already registered");
        }
        self.descriptions.insert(name, desc);
        self.signatures
            .insert(name, (TypeId::of::<I>(), TypeId::of::<O>()));

        /* 2. wrap the user‑supplied function into an erased async thunk */
        let func_arc: Arc<F> = Arc::new(func);
        self.funcs.insert(
            name,
            Arc::new(
                move |raw: Value| -> BoxFuture<'static, Result<Value, ToolError>> {
                    let func_clone = func_arc.clone();
                    async move {
                        /* 2a. JSON → I */
                        let input: I = serde_json::from_value(raw).map_err(|e| {
                            ToolError::Deserialize(DeserializationError(Cow::Owned(e.to_string())))
                        })?;

                        /* 2b. execute user code */
                        let output: O = (func_clone)(input).await;

                        /* 2c. I/O → JSON */
                        serde_json::to_value(output).map_err(|e| ToolError::Runtime(e.to_string()))
                    }
                    .boxed()
                },
            ),
        );

        self
    }

    /// Dispatch a [`FunctionCall`] produced by an LLM and return the tool’s
    /// serialized output.
    ///
    /// # Errors
    /// * [`ToolError::FunctionNotFound`] if `call.name` is not registered.
    /// * [`ToolError::Deserialize`] if JSON payload fails to parse as `I`.
    /// * [`ToolError::Runtime`] for anything thrown inside user code.
    ///
    /// # Example
    /// ```rust
    /// use tool_collection::{ToolCollection, FunctionCall};
    /// use serde_json::json;
    /// let mut hub = ToolCollection::new();
    /// hub.register("inc", "Increment", |x: u8| async move { x + 1 });
    /// let out = hub.call(FunctionCall { name: "inc".into(), arguments: json!(41) }).await.unwrap();
    /// assert_eq!(out, json!(42));
    /// ```
    pub async fn call(&self, call: FunctionCall) -> Result<Value, ToolError> {
        let async_func = self.funcs.get(call.name.as_str()).ok_or_else(|| {
            // Leak the string so we can return a 'static ref in the error
            let leaked: &'static str = Box::leak(call.name.into_boxed_str());
            ToolError::FunctionNotFound { name: leaked }
        })?;

        async_func(call.arguments).await
    }

    /*──────────────────── helpers / iterators ───────────────────────*/

    /// Iterate over `(name, description)` pairs for *all* registered tools.
    pub fn descriptions(&self) -> impl Iterator<Item = (&'static str, &'static str)> + '_ {
        self.descriptions.iter().map(|(k, v)| (*k, *v))
    }

    /// Iterate over `(name, (input_type_id, output_type_id))` tuples.
    pub fn signatures(&self) -> impl Iterator<Item = (&'static str, (TypeId, TypeId))> + '_ {
        self.signatures.iter().map(|(k, v)| (*k, *v))
    }

    /*──────────── auto‑populate every #[tool] function ───────────────*/

    /// Construct a registry by **collecting all functions** annotated with the
    /// `#[tool]` proc‑macro (requires the companion `tool_collection_macros` and
    /// the [`inventory`](https://docs.rs/inventory) crate).
    ///
    /// # Example
    /// ```rust
    /// # use tool_collection::ToolCollection;
    /// let hub = ToolCollection::collect_tools();
    /// assert!(hub.descriptions().count() > 0);
    /// ```
    pub fn collect_tools() -> Self {
        let mut hub = Self::new();
        for reg in inventory::iter::<ToolRegistration> {
            hub.descriptions.insert(reg.name, reg.doc);
            hub.funcs.insert(reg.name, Arc::new(reg.f));
        }
        hub
    }
}

/// One compile‑time registration record, emitted by the `#[tool]` proc‑macro
/// in the *macro* crate and gathered by [`inventory`].  End users rarely
/// interact with this struct directly.
pub struct ToolRegistration {
    /// Name under which the tool is exposed.
    pub name: &'static str,
    /// Rust docstring extracted from the function body.
    pub doc: &'static str,
    /// Erased async callable.
    pub f: fn(Value) -> BoxFuture<'static, Result<Value, ToolError>>,
}

impl ToolRegistration {
    /// Create a new registration record – mostly used by generated code.
    pub const fn new(
        name: &'static str,
        doc: &'static str,
        f: fn(Value) -> BoxFuture<'static, Result<Value, ToolError>>,
    ) -> Self {
        Self { name, doc, f }
    }
}

// Tell `inventory` to collect every `ToolRegistration` emitted across the
// dependency graph into a single linked list available at runtime.
inventory::collect!(ToolRegistration);
