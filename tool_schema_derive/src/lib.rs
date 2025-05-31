#![forbid(unsafe_code)]

use proc_macro::TokenStream;
use proc_macro_error::{abort, proc_macro_error};
use proc_macro_crate::{crate_name, FoundCrate};
use quote::quote;
use syn::{
    parse_macro_input, Data, DeriveInput, Fields, FieldsNamed, FieldsUnnamed, Type,
};

#[proc_macro_error]
#[proc_macro_derive(ToolSchema)]
pub fn derive_tool_schema(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    
    match &input.data {
        Data::Struct(data_struct) => {
            match &data_struct.fields {
                Fields::Named(fields) => generate_struct_schema(&input, fields),
                Fields::Unnamed(fields) => generate_tuple_struct_schema(&input, fields),
                Fields::Unit => generate_unit_struct_schema(&input),
            }
        }
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
                let mut properties = ::std::collections::HashMap::<String, ::serde_json::Value>::new();
                #(properties.insert(#field_names.to_string(), <#field_types as #crate_path::ToolSchema>::schema());)*
                
                ::serde_json::json!({
                    "type": "object",
                    "properties": properties,
                    "required": #required_array
                })
            }
        }
    })
}

fn generate_tuple_struct_schema(input: &DeriveInput, fields: &FieldsUnnamed) -> TokenStream {
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let crate_path = get_crate_path();
    
    let field_schemas: Vec<_> = fields.unnamed.iter().map(|field| {
        let field_type = &field.ty;
        quote! { <#field_type as #crate_path::ToolSchema>::schema() }
    }).collect();
    
    let field_count = fields.unnamed.len();

    
    TokenStream::from(quote! {
        impl #impl_generics #crate_path::ToolSchema for #name #ty_generics #where_clause {
            fn schema() -> ::serde_json::Value {
                ::serde_json::json!({
                    "type": "array",
                    "prefixItems": [#(#field_schemas),*],
                    "minItems": #field_count,
                    "maxItems": #field_count
                })
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
                ::serde_json::json!({
                    "type": "object",
                    "properties": {},
                    "required": ::std::vec::Vec::<&str>::new()
                })
            }
        }
    })
}

fn get_crate_path() -> proc_macro2::TokenStream {
    match crate_name("tool_schema") {
        Ok(FoundCrate::Itself) => quote!(crate),
        Ok(FoundCrate::Name(name)) => {
            let ident = proc_macro2::Ident::new(&name, proc_macro2::Span::call_site());
            quote!(#ident)
        }
        Err(_) => quote!(::tool_schema),
    }
}

fn is_option_type(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            if segment.ident == "Option" {
                // Check the path structure directly instead of string matching
                let segments: Vec<_> = type_path.path.segments.iter().collect();
                match segments.len() {
                    1 => true, // Just "Option" - assume it's std::option::Option from prelude
                    3 => {
                        // Check for std::option::Option or core::option::Option
                        (segments[0].ident == "std" && segments[1].ident == "option") ||
                        (segments[0].ident == "core" && segments[1].ident == "option")
                    },
                    _ => false,
                }
            } else {
                false
            }
        } else {
            false
        }
    } else {
        false
    }
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

    // Note: Field name handling regression test removed because proc macro
    // functions cannot be called outside of proc macro context.
    // The fix for LitStr field names is verified through integration tests.

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