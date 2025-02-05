extern crate proc_macro;
use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::{
    parse_macro_input, Attribute, Data, DeriveInput, FnArg, ImplItem, ItemImpl, Pat, ReturnType,
};

#[proc_macro_derive(Tool, attributes(doc))]
pub fn derive_tool(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_name = &input.ident;

    // Extract struct-level documentation using the same technique as the `tools` macro
    let struct_docs = input
        .attrs
        .iter()
        .filter_map(|attr| {
            Some(
                attr.to_token_stream()
                    .to_string()
                    .strip_prefix("#[doc = \"")?
                    .strip_suffix("\"]")?
                    .to_string(),
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    // Ensure the macro is applied to a struct
    let field_descriptions = match &input.data {
        Data::Struct(s) => {
            s.fields
                .iter()
                .filter_map(|f| {
                    let name = f.ident.as_ref()?.to_string(); // Convert Ident to String

                    // Extract field documentation
                    let field_docs = f
                        .attrs
                        .iter()
                        .filter_map(|attr| {
                            Some(
                                attr.to_token_stream()
                                    .to_string()
                                    .strip_prefix("#[doc = \"")?
                                    .strip_suffix("\"]")?
                                    .to_string(),
                            )
                        })
                        .collect::<Vec<_>>()
                        .join("\n");

                    Some(format!("{}: {}", name, field_docs))
                })
                .collect::<Vec<_>>()
                .join("\n")
        }
        _ => {
            return syn::Error::new_spanned(
                struct_name,
                "Error: #[derive(Tool)] can only be used on structs.",
            )
            .to_compile_error()
            .into();
        }
    };

    // Generate the trait implementation
    let expanded = quote! {
        impl ::toors::Tool for #struct_name {
            fn description() -> &'static str {
                #struct_docs
            }

            fn signature() -> &'static str {
                #field_descriptions
            }
        }
    };

    TokenStream::from(expanded)
}

#[proc_macro_attribute]
pub fn tools(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let impl_block = parse_macro_input!(item as ItemImpl);
    let struct_name = &impl_block.self_ty;
    let struct_name_str = struct_name.to_token_stream().to_string();

    let meta_tokens: Vec<_> = impl_block
        .items
        .iter()
        .filter_map(|item| match item {
            ImplItem::Fn(method) => {
                let name = method.sig.ident.to_string();
                // Extract doc comments
                let description = method
                    .attrs
                    .iter()
                    .filter_map(|attr| {
                        Some(
                            attr.meta
                                .to_token_stream()
                                .to_string()
                                .strip_prefix("doc = \"")?
                                .strip_suffix('"')?
                                .to_string(),
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n");

                let params = method
                    .sig
                    .inputs
                    .iter()
                    .filter_map(|input| match input {
                        FnArg::Typed(pat_type) => {
                            let name = match &*pat_type.pat {
                                Pat::Ident(ident) => ident.ident.to_string(),
                                _ => "_".to_string(),
                            };
                            let ty = pat_type.ty.to_token_stream().to_string();
                            Some(format!("{}: {}", name, ty))
                        }
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join(", ");

                let return_type = match &method.sig.output {
                    ReturnType::Type(_, ty) => ty.to_token_stream().to_string(),
                    _ => "()".to_string(),
                };

                let signature = format!(
                    "{}::{}({}) -> {}",
                    struct_name_str, name, params, return_type
                );

                Some(quote! {
                    map.insert(#name.to_string(), toors::ToolMetadata {
                        description: #description.to_string(),
                        signature: #signature.to_string(),
                    });
                })
            }
            _ => None,
        })
        .collect();

    let expanded = quote! {
        #impl_block

        impl #struct_name {
            pub fn tools() -> std::collections::HashMap<String, toors::ToolMetadata> {
                let mut map = std::collections::HashMap::new();
                #(#meta_tokens)*
                map
            }
        }
    };

    TokenStream::from(expanded)
}

fn get_docs(attrs: &[Attribute]) -> String {
    attrs
        .iter()
        .filter(|attr| attr.path().is_ident("doc"))
        .filter_map(|attr| {
            attr.parse_args::<syn::LitStr>()
                .ok()
                .map(|lit| lit.value())
                .map(|s| s.trim().to_string())
        })
        .collect::<Vec<_>>()
        .join("\n")
}


