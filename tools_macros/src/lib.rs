//! Procedural macros for **tools-rs**
#![forbid(unsafe_code)]

use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use proc_macro_crate::{crate_name, FoundCrate};
use proc_macro_error::{abort, proc_macro_error};
use quote::quote;
use syn::{
    parse_macro_input, Attribute, Data, DeriveInput, Expr, ExprLit, Fields, FieldsNamed,
    FieldsUnnamed, FnArg, ItemFn, Lit, LitStr, Meta, Pat, PatIdent, PatType, Type, TypePath,
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
    let crate_path = get_crate_path();

    // ───────── Macro expansion ─────────
    TokenStream::from(quote! {
        #func

        #[allow(non_camel_case_types)]
        #[derive(::serde::Deserialize, tools_macros::ToolSchema)]
        struct #wrapper_ident { #( pub #idents : #types ),* }

        #[inline(always)]
        fn #schema_fn<T: #crate_path::ToolSchema>() -> ::serde_json::Value {
            T::schema()
        }

        inventory::submit! {
            #crate_path::ToolRegistration::new(
                #fn_name_str,
                #doc_lit,
                |v| ::std::boxed::Box::pin(async move {
                    let arg: #wrapper_ident =
                        ::serde_json::from_value(v)
                            .map_err(#crate_path::DeserializationError::from)?;
                    let out = #fn_name( #( arg.#idents ),* ).await;
                    ::serde_json::to_value(out)
                        .map_err(|e| #crate_path::ToolError::Runtime(e.to_string()))
                }),
                || #schema_fn::<#wrapper_ident>(),
            )
        }
    })
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
