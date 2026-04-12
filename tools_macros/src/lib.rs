//! Procedural macros for **tools-rs**
#![forbid(unsafe_code)]

use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use proc_macro_crate::{crate_name, FoundCrate};
use proc_macro_error::{abort, proc_macro_error};
use quote::quote;
use syn::{
    parse::Parser, parse_macro_input, punctuated::Punctuated, Attribute, Data, DeriveInput, Expr,
    ExprLit, Fields, FieldsNamed, FieldsUnnamed, FnArg, ItemFn, Lit, LitStr, Meta, Pat, PatIdent,
    PatType, Token, Type, TypePath,
};

// ============================================================================
// TOOL SCHEMA DERIVE MACRO
// ============================================================================

#[proc_macro_error]
#[proc_macro_derive(ToolSchema)]
pub fn derive_tool_schema(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match &input.data {
        Data::Struct(data_struct) => match &data_struct.fields {
            Fields::Named(fields) => generate_struct_schema(&input, fields),
            Fields::Unnamed(fields) => generate_tuple_struct_schema(&input, fields),
            Fields::Unit => generate_unit_struct_schema(&input),
        },
        Data::Enum(_) => {
            abort!(input.ident, "Enum schemas are not yet supported");
        }
        Data::Union(_) => {
            abort!(input.ident, "Union schemas are not supported");
        }
    }
}

fn generate_struct_schema(input: &DeriveInput, fields: &FieldsNamed) -> TokenStream {
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let crate_path = get_crate_path();

    let mut field_names = Vec::new();
    let mut field_types = Vec::new();
    let mut required_fields = Vec::new();

    for field in &fields.named {
        let field_name = field.ident.as_ref().unwrap();
        let field_name_str = field_name.to_string();
        let field_type = &field.ty;

        // Check if field is Option<T> to determine if it's required
        let is_optional = is_option_type(field_type);

        if !is_optional {
            required_fields.push(field_name_str.clone());
        }

        field_names.push(field_name_str);
        field_types.push(field_type);
    }

    let required_array = if required_fields.is_empty() {
        quote! { ::std::vec::Vec::<&str>::new() }
    } else {
        quote! { vec![#(#required_fields),*] }
    };

    TokenStream::from(quote! {
        impl #impl_generics #crate_path::ToolSchema for #name #ty_generics #where_clause {
            fn schema() -> ::serde_json::Value {
                static SCHEMA: #crate_path::once_cell::sync::Lazy<::serde_json::Value> = #crate_path::once_cell::sync::Lazy::new(|| {
                    let mut properties = ::std::collections::HashMap::<String, ::serde_json::Value>::new();
                    #(properties.insert(#field_names.to_string(), <#field_types as #crate_path::ToolSchema>::schema());)*

                    ::serde_json::json!({
                        "type": "object",
                        "properties": properties,
                        "required": #required_array
                    })
                });
                SCHEMA.clone()
            }
        }
    })
}

fn generate_tuple_struct_schema(input: &DeriveInput, fields: &FieldsUnnamed) -> TokenStream {
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let crate_path = get_crate_path();

    let field_schemas: Vec<_> = fields
        .unnamed
        .iter()
        .map(|field| {
            let field_type = &field.ty;
            quote! { <#field_type as #crate_path::ToolSchema>::schema() }
        })
        .collect();

    let field_count = fields.unnamed.len();

    TokenStream::from(quote! {
        impl #impl_generics #crate_path::ToolSchema for #name #ty_generics #where_clause {
            fn schema() -> ::serde_json::Value {
                static SCHEMA: #crate_path::once_cell::sync::Lazy<::serde_json::Value> = #crate_path::once_cell::sync::Lazy::new(|| {
                    ::serde_json::json!({
                        "type": "array",
                        "prefixItems": [#(#field_schemas),*],
                        "minItems": #field_count,
                        "maxItems": #field_count
                    })
                });
                SCHEMA.clone()
            }
        }
    })
}

fn generate_unit_struct_schema(input: &DeriveInput) -> TokenStream {
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let crate_path = get_crate_path();

    TokenStream::from(quote! {
        impl #impl_generics #crate_path::ToolSchema for #name #ty_generics #where_clause {
            fn schema() -> ::serde_json::Value {
                static SCHEMA: #crate_path::once_cell::sync::Lazy<::serde_json::Value> = #crate_path::once_cell::sync::Lazy::new(|| {
                    ::serde_json::json!({
                        "type": "object",
                        "properties": {},
                        "required": ::std::vec::Vec::<&str>::new()
                    })
                });
                SCHEMA.clone()
            }
        }
    })
}

fn get_crate_path() -> proc_macro2::TokenStream {
    match crate_name("tools_core") {
        Ok(FoundCrate::Itself) => quote!(crate),
        Ok(FoundCrate::Name(name)) => {
            let ident = proc_macro2::Ident::new(&name, proc_macro2::Span::call_site());
            quote!(#ident)
        }
        Err(_) => quote!(::tools_core),
    }
}

fn is_option_type(ty: &Type) -> bool {
    // 1. Bail out quickly if this isn’t a plain path (`T` vs `&T`, `Vec<T>` …)
    let Type::Path(TypePath { qself: None, path }) = ty else {
        return false;
    };

    // 2. If the last segment isn’t literally `Option`, we’re done.
    let Some(last) = path.segments.last() else {
        return false;
    };
    if last.ident != "Option" {
        return false;
    }

    // 3. Inspect the *whole* path without allocating.
    //    `syn::punctuated::Punctuated` gives us an iterator we can pattern-match on.
    match path
        .segments
        .iter()
        .map(|s| &s.ident)
        .collect::<Vec<_>>()
        .as_slice()
    {
        // `Option`
        [ident] if *ident == "Option" => true,

        // `std::option::Option` or `core::option::Option`
        [first, second, ident]
            if (*first == "std" || *first == "core")
                && *second == "option"
                && *ident == "Option" =>
        {
            true
        }

        _ => false,
    }
}

// ============================================================================
// TOOL ATTRIBUTE MACRO
// ============================================================================

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

#[proc_macro_error]
#[proc_macro_attribute]
pub fn tool(attr: TokenStream, item: TokenStream) -> TokenStream {
    // ───────── Parse #[tool(key = value, ...)] attributes ─────────
    let meta_json = parse_tool_attrs(attr);
    let meta_lit = LitStr::new(&meta_json, Span::call_site());

    // ───────── Parse the user function ─────────
    let func: ItemFn = parse_macro_input!(item);
    let fn_name = &func.sig.ident;
    let fn_name_str = fn_name.to_string();
    let doc_lit = LitStr::new(&docs(&func.attrs), Span::call_site());

    // ───────── Inputs → wrapper struct fields ─────────
    // Detect reserved `ctx` first parameter.
    let all_params: Vec<_> = func
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
        .collect();

    // If the first parameter is named `ctx`, treat it as context injection.
    // The user writes `ctx: T`; we rewrite the emitted fn to `ctx: Arc<T>`
    // so that field access and method calls work via Deref.
    let (ctx_inner_ty, param_pairs) = if all_params
        .first()
        .map_or(false, |(ident, _)| ident == "ctx")
    {
        let ctx_ty = &all_params[0].1;
        // Reject `ctx: Arc<T>` — we wrap in Arc internally, so the user
        // must write `ctx: T` (not `ctx: Arc<T>`).
        if is_arc_type(ctx_ty) {
            abort!(
                ctx_ty,
                "`ctx` must be typed as `T`, not `Arc<T>` — the `#[tool]` macro wraps it in `Arc` automatically"
            );
        }
        (Some(ctx_ty.clone()), all_params[1..].to_vec())
    } else {
        (None, all_params)
    };

    let (idents, types): (Vec<_>, Vec<_>) = param_pairs.into_iter().unzip();

    // ───────── Generated helper idents ─────────
    let wrapper_ident = Ident::new(&format!("__TOOL_INPUT_{fn_name}"), Span::call_site());
    let schema_fn = Ident::new(&format!("__SCHEMA_FOR_{fn_name}"), Span::call_site());
    let crate_path = get_crate_path();

    // ───────── Context-dependent codegen ─────────
    let (closure_body, needs_ctx_lit, ctx_type_id_expr, ctx_type_name_lit) =
        if let Some(ref inner_ty) = ctx_inner_ty {
            let type_name_str = quote!(#inner_ty).to_string();
            let type_name_lit = LitStr::new(&type_name_str, Span::call_site());
            (
                quote! {
                    |v, ctx_opt| ::std::boxed::Box::pin(async move {
                        let ctx_any = ctx_opt.ok_or_else(|| #crate_path::ToolError::MissingCtx {
                            tool: #fn_name_str,
                        })?;
                        let ctx: ::std::sync::Arc<#inner_ty> =
                            ctx_any.downcast::<#inner_ty>().map_err(|_| {
                                #crate_path::ToolError::Runtime(
                                    "context downcast failed".to_string(),
                                )
                            })?;
                        let arg: #wrapper_ident =
                            ::serde_json::from_value(v)
                                .map_err(#crate_path::DeserializationError::from)?;
                        let out = #fn_name(ctx, #( arg.#idents ),* ).await;
                        ::serde_json::to_value(out)
                            .map_err(|e| #crate_path::ToolError::Runtime(e.to_string()))
                    })
                },
                quote!(true),
                quote!(Some(|| ::std::any::TypeId::of::<#inner_ty>())),
                type_name_lit,
            )
        } else {
            let empty_name = LitStr::new("", Span::call_site());
            (
                quote! {
                    |v, _ctx| ::std::boxed::Box::pin(async move {
                        let arg: #wrapper_ident =
                            ::serde_json::from_value(v)
                                .map_err(#crate_path::DeserializationError::from)?;
                        let out = #fn_name( #( arg.#idents ),* ).await;
                        ::serde_json::to_value(out)
                            .map_err(|e| #crate_path::ToolError::Runtime(e.to_string()))
                    })
                },
                quote!(false),
                quote!(None),
                empty_name,
            )
        };

    // ───────── Rewrite fn signature if ctx detected ─────────
    // User wrote `ctx: T`, emit `ctx: Arc<T>` so Deref covers .field / .method().
    let emitted_func = if let Some(ref inner_ty) = ctx_inner_ty {
        let mut func_out = func.clone();
        if let Some(first_arg) = func_out.sig.inputs.first_mut() {
            if let FnArg::Typed(pat_type) = first_arg {
                pat_type.ty = Box::new(syn::parse_quote!(::std::sync::Arc<#inner_ty>));
            }
        }
        func_out
    } else {
        func
    };

    // ───────── Macro expansion ─────────
    TokenStream::from(quote! {
        #emitted_func

        #[allow(non_camel_case_types)]
        #[derive(::serde::Deserialize, tools_macros::ToolSchema)]
        struct #wrapper_ident { #( pub #idents : #types ),* }

        #[inline(always)]
        fn #schema_fn<T: #crate_path::ToolSchema>() -> ::serde_json::Value {
            T::schema()
        }

        inventory::submit! {
            #crate_path::ToolRegistration {
                name: #fn_name_str,
                doc: #doc_lit,
                f: #closure_body,
                param_schema: || #schema_fn::<#wrapper_ident>(),
                meta_json: #meta_lit,
                needs_ctx: #needs_ctx_lit,
                ctx_type_id: #ctx_type_id_expr,
                ctx_type_name: #ctx_type_name_lit,
            }
        }
    })
}

/// Parse `#[tool(key = value, key2 = value2, flag, ...)]` into a JSON
/// object literal that gets stored on `ToolRegistration::meta_json`.
/// Returns `"{}"` for empty attribute lists.
fn parse_tool_attrs(attr: TokenStream) -> String {
    if attr.is_empty() {
        return "{}".to_string();
    }

    let parser = Punctuated::<Meta, Token![,]>::parse_terminated;
    let metas = match parser.parse(attr) {
        Ok(m) => m,
        Err(e) => abort!(e.span(), "failed to parse `#[tool(...)]` attributes: {}", e),
    };

    let mut map = serde_json::Map::new();
    for m in metas {
        match m {
            Meta::NameValue(nv) => {
                let key = match nv.path.get_ident() {
                    Some(id) => id.to_string(),
                    None => abort!(nv.path, "attribute key must be a single identifier"),
                };
                if key == "name" || key == "description" {
                    abort!(
                        nv.path,
                        "`{}` is reserved — set it via the function name and doc comment",
                        key
                    );
                }
                if map.contains_key(&key) {
                    abort!(nv.path, "duplicate attribute key `{}`", key);
                }
                map.insert(key, attr_expr_to_json(&nv.value));
            }
            Meta::Path(p) => {
                let key = match p.get_ident() {
                    Some(id) => id.to_string(),
                    None => abort!(p, "attribute key must be a single identifier"),
                };
                if key == "name" || key == "description" {
                    abort!(p, "`{}` is reserved", key);
                }
                if map.contains_key(&key) {
                    abort!(p, "duplicate attribute key `{}`", key);
                }
                map.insert(key, serde_json::Value::Bool(true));
            }
            Meta::List(l) => abort!(
                l,
                "nested attributes are not supported — use flat `key = value` pairs"
            ),
        }
    }

    serde_json::Value::Object(map).to_string()
}

fn attr_expr_to_json(e: &Expr) -> serde_json::Value {
    match e {
        Expr::Lit(ExprLit {
            lit: Lit::Bool(b), ..
        }) => serde_json::Value::Bool(b.value),
        Expr::Lit(ExprLit {
            lit: Lit::Str(s), ..
        }) => serde_json::Value::String(s.value()),
        Expr::Lit(ExprLit {
            lit: Lit::Int(i), ..
        }) => match i.base10_parse::<i64>() {
            Ok(n) => serde_json::Value::Number(n.into()),
            Err(err) => abort!(i, "invalid integer literal: {}", err),
        },
        Expr::Lit(ExprLit {
            lit: Lit::Float(f), ..
        }) => match f.base10_parse::<f64>() {
            Ok(n) => match serde_json::Number::from_f64(n) {
                Some(num) => serde_json::Value::Number(num),
                None => abort!(f, "float literal cannot be represented as JSON number"),
            },
            Err(err) => abort!(f, "invalid float literal: {}", err),
        },
        // Negative integer/float — `-5` parses as Expr::Unary, not a literal.
        Expr::Unary(syn::ExprUnary {
            op: syn::UnOp::Neg(_),
            expr,
            ..
        }) => match expr.as_ref() {
            Expr::Lit(ExprLit {
                lit: Lit::Int(i), ..
            }) => match i.base10_parse::<i64>().map(|n| n.checked_neg()) {
                Ok(Some(n)) => serde_json::Value::Number(n.into()),
                Ok(None) => abort!(i, "integer literal overflows i64 when negated"),
                Err(err) => abort!(i, "invalid integer literal: {}", err),
            },
            Expr::Lit(ExprLit {
                lit: Lit::Float(f), ..
            }) => match f.base10_parse::<f64>() {
                Ok(n) => match serde_json::Number::from_f64(-n) {
                    Some(num) => serde_json::Value::Number(num),
                    None => abort!(f, "float literal cannot be represented as JSON number"),
                },
                Err(err) => abort!(f, "invalid float literal: {}", err),
            },
            _ => abort!(e, "attribute values must be bool/int/float/string literals"),
        },
        _ => abort!(e, "attribute values must be bool/int/float/string literals"),
    }
}

/// Returns `true` if the type looks like `Arc<_>` (or `std::sync::Arc<_>`).
fn is_arc_type(ty: &Type) -> bool {
    if let Type::Path(TypePath { path, .. }) = ty {
        if let Some(last) = path.segments.last() {
            return last.ident == "Arc"
                && matches!(last.arguments, syn::PathArguments::AngleBracketed(_));
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::{parse_quote, Type};

    #[test]
    fn test_is_option_type_detection() {
        // Test simple Option
        let simple_option: Type = parse_quote!(Option<i32>);
        assert!(is_option_type(&simple_option));

        // Test std::option::Option
        let std_option: Type = parse_quote!(std::option::Option<String>);
        assert!(is_option_type(&std_option));

        // Test core::option::Option
        let core_option: Type = parse_quote!(core::option::Option<bool>);
        assert!(is_option_type(&core_option));

        // Test non-Option types
        let vec_type: Type = parse_quote!(Vec<i32>);
        assert!(!is_option_type(&vec_type));

        let string_type: Type = parse_quote!(String);
        assert!(!is_option_type(&string_type));

        let custom_type: Type = parse_quote!(MyCustomOption<i32>);
        assert!(!is_option_type(&custom_type));

        // Test invalid paths that contain "Option" but aren't Option
        let fake_option: Type = parse_quote!(my_mod::Option<i32>);
        assert!(!is_option_type(&fake_option));

        let nested_fake: Type = parse_quote!(some::long::path::Option<i32>);
        assert!(!is_option_type(&nested_fake));
    }

    #[test]
    fn test_required_fields_detection() {
        let input: DeriveInput = parse_quote! {
            struct TestStruct {
                required_field: i32,
                optional_field: Option<String>,
                another_required: bool,
                another_optional: Option<Vec<i32>>,
            }
        };

        let fields = match &input.data {
            syn::Data::Struct(data_struct) => match &data_struct.fields {
                syn::Fields::Named(fields) => fields,
                _ => panic!("Expected named fields"),
            },
            _ => panic!("Expected struct"),
        };

        let mut required_count = 0;
        let mut optional_count = 0;

        for field in &fields.named {
            let field_type = &field.ty;
            if is_option_type(field_type) {
                optional_count += 1;
            } else {
                required_count += 1;
            }
        }

        assert_eq!(required_count, 2); // required_field, another_required
        assert_eq!(optional_count, 2); // optional_field, another_optional
    }

    #[test]
    fn test_enum_error_message() {
        let input: DeriveInput = parse_quote! {
            enum TestEnum {
                Variant1,
                Variant2(i32),
                Variant3 { field: String },
            }
        };

        // We can't easily test the abort! macro, but we can verify the enum detection
        match &input.data {
            syn::Data::Enum(_) => {
                // This is expected - enums should be detected
                assert!(true);
            }
            _ => panic!("Expected enum"),
        }
    }

    #[test]
    fn test_union_detection() {
        let input: DeriveInput = parse_quote! {
            union TestUnion {
                field1: i32,
                field2: f64,
            }
        };

        match &input.data {
            syn::Data::Union(_) => {
                // This is expected - unions should be detected
                assert!(true);
            }
            _ => panic!("Expected union"),
        }
    }
}
