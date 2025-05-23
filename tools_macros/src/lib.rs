//! Procedural macros for **Tools-rs Runtime**
#![forbid(unsafe_code)]

use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use proc_macro_error::{abort, proc_macro_error};
use quote::quote;
use syn::{
    parse_macro_input, Attribute, Expr, ExprLit, FnArg, ItemFn, Lit, LitStr, Meta, Pat, PatIdent,
    PatType, ReturnType, Type,
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

    // ───────── Return type ─────────
    let output_ty: Type = match &func.sig.output {
        ReturnType::Type(_, ty) => (**ty).clone(),
        ReturnType::Default => syn::parse_quote!(()),
    };

    // ───────── Generated helper idents ─────────
    let wrapper_ident = Ident::new(&format!("__TOOL_INPUT_{fn_name}"), Span::call_site());
    let flatten_fn = Ident::new(&format!("__FLATTEN_SCHEMA_{fn_name}"), Span::call_site());
    let schema_fn = Ident::new(&format!("__SCHEMA_FOR_{fn_name}"), Span::call_site());

    // ───────── Macro expansion ─────────
    TokenStream::from(quote! {
        #func

        #[allow(non_camel_case_types)]
        #[derive(::serde::Deserialize)]
        #[cfg_attr(feature = "schema", derive(::schemars::JsonSchema))]
        struct #wrapper_ident { #( pub #idents : #types ),* }

        #[cfg(feature = "schema")]
        fn #flatten_fn(mut v: ::serde_json::Value) -> ::serde_json::Value {
            use ::serde_json::Value::*;
            if let Object(ref mut root) = v {
                root.remove("title");
                if let Some(Object(props)) = root.remove("properties") {
                    root.extend(props);
                }
            }
            v
        }

        #[cfg(feature = "schema")]
        #[inline(always)]
        fn #schema_fn<T: ::schemars::JsonSchema>() -> ::serde_json::Value {
            #flatten_fn(::serde_json::to_value(tools::schema::schema_to_json_schema::<T>()).unwrap())
        }
        #[cfg(not(feature = "schema"))]
        #[inline(always)]
        fn #schema_fn<T>() -> ::serde_json::Value {
            ::serde_json::Value::Null
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
                || #schema_fn::<#output_ty>(),
            )
        }
    })
}
