//! tools_macros – Procedural macros for **Tools-rs Runtime**
//!
//! This version flattens `"properties"` **and** strips `"title"` from the
//! emitted JSON-Schema so the root of `"parameters"` / `"returns"` contains
//! only the relevant field definitions.

#![forbid(unsafe_code)]

use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::quote;
use syn::{
    Attribute, Expr, ExprLit, FnArg, ItemFn, Lit, LitStr, Meta, Pat, PatIdent, PatType, ReturnType,
    Type, parse_macro_input,
};

fn collect_docs(attrs: &[Attribute]) -> String {
    attrs
        .iter()
        .filter(|a| a.path().is_ident("doc"))
        .filter_map(|a| match &a.meta {
            Meta::NameValue(nv) => {
                if let Expr::Lit(ExprLit {
                    lit: Lit::Str(s), ..
                }) = &nv.value
                {
                    Some(s.value())
                } else {
                    None
                }
            }
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[proc_macro_attribute]
pub fn tool(_attr: TokenStream, item: TokenStream) -> TokenStream {
    /* parse user fn */
    let func: ItemFn = parse_macro_input!(item);
    let fn_name = &func.sig.ident;
    let fn_name_str = fn_name.to_string();

    let doc_lit = LitStr::new(&collect_docs(&func.attrs), Span::call_site());

    /* parameters → wrapper struct */
    let (idents, types): (Vec<_>, Vec<_>) = func
        .sig
        .inputs
        .iter()
        .map(|arg| match arg {
            FnArg::Typed(PatType { pat, ty, .. }) => {
                let Pat::Ident(PatIdent { ident, .. }) = &**pat else {
                    panic!("`#[tool]` supports only identifier patterns");
                };
                (ident.clone(), (**ty).clone())
            }
            _ => panic!("`#[tool]` does not support `self` receivers"),
        })
        .unzip();

    /* return type */
    let output_ty: Type = match &func.sig.output {
        ReturnType::Type(_, ty) => (**ty).clone(),
        ReturnType::Default => syn::parse_quote!(()),
    };

    /* generated names */
    let wrapper_ident = Ident::new(&format!("__ToolInput_{}", fn_name), Span::call_site());
    let flatten_fn = Ident::new(&format!("__flatten_schema_{}", fn_name), Span::call_site());

    /* expansion */
    let expanded = quote! {
        #func

        #[allow(non_camel_case_types)]
        #[derive(::serde::Deserialize)]
        #[cfg_attr(feature = "schema", derive(::schemars::JsonSchema))]
        struct #wrapper_ident {
            #( pub #idents : #types ),*
        }

        // unique helper: remove `title` and hoist `properties`
        #[cfg(feature = "schema")]
        fn #flatten_fn(mut v: ::serde_json::Value) -> ::serde_json::Value {
            if let ::serde_json::Value::Object(ref mut root) = v {
                root.remove("title"); // ─── drop the `title` key
                if let Some(::serde_json::Value::Object(props)) =
                    root.remove("properties")
                {
                    for (k, schema) in props { root.insert(k, schema); }
                }
            }
            v
        }

        inventory::submit! {
            tools::ToolRegistration::new(
                #fn_name_str,
                #doc_lit,

                /* async-call wrapper */
                |v| ::std::boxed::Box::pin(async move {
                    let arg: #wrapper_ident =
                        ::serde_json::from_value(v)
                            .map_err(tools::error::DeserializationError::from)?;
                    let out = #fn_name( #( arg.#idents ),* ).await;
                    ::serde_json::to_value(out)
                        .map_err(|e| tools::error::ToolError::Runtime(e.to_string()))
                }),

                /* parameter-schema */
                || {
                    #[cfg(feature = "schema")]
                    {
                        let raw = tools::schema::schema_to_json_schema::<#wrapper_ident>();
                        #flatten_fn(::serde_json::to_value(raw).unwrap())
                    }
                    #[cfg(not(feature = "schema"))]
                    { ::serde_json::Value::Null }
                },

                /* return-schema */
                || {
                    #[cfg(feature = "schema")]
                    {
                        let raw = tools::schema::schema_to_json_schema::<#output_ty>();
                        #flatten_fn(::serde_json::to_value(raw).unwrap())
                    }
                    #[cfg(not(feature = "schema"))]
                    { ::serde_json::Value::Null }
                }
            )
        }
    };

    TokenStream::from(expanded)
}
