//! toors_macros ‑ Procedural macros for **Toors Runtime**
//! ================================================================
//! This crate provides the `#[tool]` attribute, a one‑liner that turns any
//! `async fn` into a **self‑registering** entry inside a
//! [`tool_collection::ToolCollection`].
//!
//! ## Why use `#[tool]`?
//! * **Zero boilerplate** – no manual `register(...)` call, no lifetime hacks.
//! * **Documentation preserved** – outer `///` comments become the runtime
//!   description string exposed to LLMs.
//! * **Inventory‑powered** – tools auto‑discoverable via
//!   [`ToolCollection::collect_tools`].
//!
//! ## Basic Example
//! ```rust
//! use toors_macros::tool;
//!
//! #[tool]                             // <‑‑ compile‑time registration
//! async fn hello(name: String) -> String {
//!     format!("Hello, {name}!")
//! }
//!
//! // later…
//! let tools = tool_collection::ToolCollection::collect_tools();
//! ```
//!
//! ## Advanced Example – Custom Error Type
//! ```rust
//! use thiserror::Error;
//! use toors_macros::tool;
//!
//! #[derive(Error, Debug)]
//! enum MathErr { #[error("division by zero")] ZeroDiv }
//!
//! #[tool]
//! async fn safe_div((a, b): (i32, i32)) -> Result<i32, MathErr> {
//!     if b == 0 { Err(MathErr::ZeroDiv) } else { Ok(a / b) }
//! }
//! ```
//! The macro will box the `Result` and propagate it as
//! `ToolError::Runtime("division by zero")`.
//!
//! --------------------------------------------------------------------
//! Implementation notes
//! --------------------------------------------------------------------
//! * All `"///"` doc comments are concatenated and embedded as a `&'static str`
//!   inside the generated `ToolRegistration`.
//! * The wrapper closure performs JSON <‑> struct conversion via `serde_json`.
//! * Inventory linkage requires users to depend on the *same version* of the
//!   `inventory` crate as this macro crate.
//!
//! --------------------------------------------------------------------
#![forbid(unsafe_code)]

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{ItemFn, LitStr, parse_macro_input};

/// Attribute macro that **registers an async function as a tool**.
///
/// This is syntactic sugar for calling
/// [`ToolCollection::register`](tool_collection::ToolCollection::register), but
/// works entirely at *compile‑time* and requires **zero runtime boilerplate**.
///
/// ### Supported function shapes
/// * `async fn foo(arg: T) -> U`               – common case.
/// * `async fn bar(arg: T) -> Result<U, E>`    – errors converted to `ToolError`.
/// * Generic functions are **not** supported (the signature must be monomorphic).
///
/// ### Behaviour in detail
/// 1. **Parse & keep** the user function unchanged in the output stream.
/// 2. **Extract doc comments** (`/// ...`) and embed them as a `&'static str`.
/// 3. **Generate** a `ToolRegistration` via `inventory::submit!` that wraps the
///    user function into the erased [`ToolFunc`] signature.
/// 4. **Error mapping** – JSON deserialization failures ⇒ `ToolError::Deserialize`;
///    any other error ⇒ `ToolError::Runtime`.
///
/// The macro is **idempotent**; applying it twice to the same function emits a
/// compile‑time error.
#[proc_macro_attribute]
pub fn tool(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // ------------------------------------------------------------------
    // 1. Parse the input tokens into a syntax tree (syn).
    // ------------------------------------------------------------------
    let input = parse_macro_input!(item as ItemFn);
    let fn_name = &input.sig.ident;
    let fn_name_s = fn_name.to_string(); // stringify for runtime key

    // ------------------------------------------------------------------
    // 2. Collect outer doc comments (`/// foo`).
    // ------------------------------------------------------------------
    let docs_collected = input
        .attrs
        .iter()
        .filter_map(|attr| {
            if attr.path().is_ident("doc") {
                attr.parse_args::<LitStr>().ok().map(|lit| lit.value())
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    // Turn the concatenated docs into a literal (&'static str).
    let doc_lit = LitStr::new(&docs_collected, Span::call_site());

    // ------------------------------------------------------------------
    // 3. Generate wrapper + inventory registration.
    // ------------------------------------------------------------------
    TokenStream::from(quote! {
        #input   // preserve the original function verbatim

        inventory::submit! {
            toors::ToolRegistration::new(
                #fn_name_s,
                #doc_lit,
                |v| Box::pin(async move {
                    // JSON → strongly‑typed arg
                    let arg = serde_json::from_value(v).map_err(|e| {
                        toors::toors_errors::ToolError::Deserialize(
                            toors::toors_errors::DeserializationError(
                                std::borrow::Cow::Owned(e.to_string())
                            )
                        )
                    })?;
                    // Execute user function
                    let out = #fn_name(arg).await;
                    // Serialize result back to JSON
                    serde_json::to_value(out)
                        .map_err(|e| toors::toors_errors::ToolError::Runtime(e.to_string()))
                })
            )
        }
    })
}
