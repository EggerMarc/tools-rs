//! toors_macros – Procedural macros for **Toors Runtime**

#![forbid(unsafe_code)]

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{ItemFn, LitStr, parse_macro_input};

/// `#[tool]` – register an async function as a Toors tool.
///
/// See crate‑level docs for details.
#[proc_macro_attribute]
pub fn tool(_attr: TokenStream, item: TokenStream) -> TokenStream {
    /*───────────────────────────────────────────────────────────────*/
    /* 1. Parse the user function                                   */
    /*───────────────────────────────────────────────────────────────*/
    let input = parse_macro_input!(item as ItemFn);
    let fn_name = &input.sig.ident;
    let fn_name_s = fn_name.to_string();

    /*───────────────────────────────────────────────────────────────*/
    /* 2. Collect doc comments (/// …)                              */
    /*───────────────────────────────────────────────────────────────*/
    let docs_combined = input
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
    let doc_lit = LitStr::new(&docs_combined, Span::call_site());

    /*───────────────────────────────────────────────────────────────*/
    /* 3. Generate wrapper + inventory registration                 */
    /*───────────────────────────────────────────────────────────────*/
    TokenStream::from(quote! {
        #input     /* keep the original function unchanged */

        inventory::submit! {
            toors::ToolRegistration::new(
                #fn_name_s,
                #doc_lit,
                |v| Box::pin(async move {
                    /* JSON → strongly‑typed arg */
                    let arg = serde_json::from_value(v)
                        .map_err(toors::error::DeserializationError::from)?;

                    /* call the user function */
                    let out = #fn_name(arg).await;

                    /* back to JSON */
                    serde_json::to_value(out)
                        .map_err(|e| toors::error::ToolError::Runtime(e.to_string()))
                })
            )
        }
    })
}
