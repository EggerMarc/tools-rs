//! Procedural macros for **Tools-rs Runtime**
#![forbid(unsafe_code)]

use proc_macro::TokenStream;
use proc_macro_error::{abort, proc_macro_error};
use proc_macro2::{Ident, Span};
use quote::quote;
use syn::{
    Attribute, Expr, ExprLit, FnArg, ItemFn, Lit, LitStr, Meta, Pat, PatIdent, PatType,
    parse_macro_input,
};

/// Gather `///` doc-comments into a single string, trimming the leading space after `///`.
fn docs(attrs: &[Attribute]) -> String {
    attrs
        .iter()
        .filter_map(|a| match &a.meta {
            Meta::NameValue(nv) if a.path().is_ident("doc") => {
                if let Expr::Lit(ExprLit {
                    lit: Lit::Str(s), ..
                }) = &nv.value
                {
                    Some(s.value().trim_start().to_owned())
                } else {
                    None
                }
            }
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[proc_macro_error] // better diagnostics than panic!
#[proc_macro_attribute]
pub fn tool(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // ───────── Parse the user function ─────────
    let func: ItemFn = parse_macro_input!(item);
    let fn_name = &func.sig.ident;
    let fn_name_str = fn_name.to_string();
    let doc_lit = LitStr::new(&docs(&func.attrs), Span::call_site());

    // ───────── Inputs → wrapper struct fields ─────────
    let (idents, types): (Vec<_>, Vec<_>) = func
        .sig
        .inputs
        .iter()
        .map(|arg| match arg {
            FnArg::Typed(PatType { pat, ty, .. }) => {
                let Pat::Ident(PatIdent { ident, .. }) = &**pat else {
                    abort!(pat, "`#[tool]` supports only identifier patterns");
                };
                (ident.clone(), (**ty).clone())
            }
            _ => abort!(arg, "`#[tool]` may not be used on `self` methods"),
        })
        .unzip();

    // ───────── Generated helper idents ─────────
    let wrapper_ident = Ident::new(&format!("__TOOL_INPUT_{fn_name}"), Span::call_site());
    let schema_fn = Ident::new(&format!("__SCHEMA_FOR_{fn_name}"), Span::call_site());

    // ───────── Macro expansion ─────────
    TokenStream::from(quote! {
        #func

        #[allow(non_camel_case_types)]
        #[derive(::serde::Deserialize, ::tool_schema::ToolSchema)]
        struct #wrapper_ident { #( pub #idents : #types ),* }

        #[inline(always)]
        fn #schema_fn<T: ::tool_schema::ToolSchema>() -> ::serde_json::Value {
            T::schema()
        }

        inventory::submit! {
            tools::ToolRegistration::new(
                #fn_name_str,
                #doc_lit,
                |v| ::std::boxed::Box::pin(async move {
                    let arg: #wrapper_ident =
                        ::serde_json::from_value(v)
                            .map_err(tools::error::DeserializationError::from)?;
                    let out = #fn_name( #( arg.#idents ),* ).await;
                    ::serde_json::to_value(out)
                        .map_err(|e| tools::error::ToolError::Runtime(e.to_string()))
                }),
                || #schema_fn::<#wrapper_ident>(),
            )
        }
    })
}
