extern crate proc_macro;
use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::{
    parse_macro_input, parse_quote, Attribute, Data, DeriveInput, FnArg, GenericParam, ImplItem,
    ItemImpl, Pat, ReturnType,
};
use proc_macro2::Span;

#[proc_macro_derive(Tool, attributes(doc))]
pub fn derive_tool(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_name = input.ident;

    // Modify generics: add a 'static bound for each type parameter.
    let mut generics = input.generics;
    for param in &mut generics.params {
        if let GenericParam::Type(ref mut type_param) = param {
            type_param.bounds.push(parse_quote!('static));
        }
    }
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    // Collect the struct-level documentation.
    let struct_docs = input
        .attrs
        .iter()
        .filter_map(|attr| {
            attr.to_token_stream()
                .to_string()
                .strip_prefix("#[doc = \"")
                .and_then(|s| s.strip_suffix("\"]"))
                .map(|s| s.to_string())
        })
        .collect::<Vec<_>>()
        .join("\n");

    // Process fields: build two things:
    //   1. A description of each field (using its doc comment, if any).
    //   2. A vector of expressions that, at runtime, evaluate the field's type.
    let (description_fields, field_sig_exprs) = match &input.data {
        Data::Struct(s) => {
            let mut desc_lines = Vec::new();
            let mut sig_exprs = Vec::new();
            for f in s.fields.iter() {
                // Only process named fields.
                if let Some(ident) = &f.ident {
                    let field_name = ident.to_string();
                    // Extract field documentation (if any).
                    let field_docs: String = f
                        .attrs
                        .iter()
                        .filter_map(|attr| {
                            attr.to_token_stream()
                                .to_string()
                                .strip_prefix("#[doc = \"")
                                .and_then(|s| s.strip_suffix("\"]"))
                                .map(|s| s.to_string())
                        })
                        .collect::<Vec<_>>()
                        .join(" ");
                    desc_lines.push(format!(" - {}: {}", field_name, field_docs));
                    
                    // Generate an expression that will compute the field's runtime type.
                    let field_name_lit = syn::LitStr::new(&field_name, Span::call_site());
                    let field_ty = &f.ty;
                    sig_exprs.push(quote! {
                        format!("{}: {}", #field_name_lit, std::any::type_name::<#field_ty>())
                    });
                }
            }
            (desc_lines.join("\n"), sig_exprs)
        }
        _ => {
            return syn::Error::new_spanned(
                struct_name,
                "#[derive(Tool)] can only be used on structs",
            )
            .to_compile_error()
            .into();
        }
    };

    // Build a full description string from the struct name, its docs, and each field description.
    let full_description = format!("{}: {}\n{}", struct_name, struct_docs, description_fields);
    let full_description_lit = syn::LitStr::new(&full_description, Span::call_site());

    let expanded = quote! {
        impl #impl_generics ::toors::Tool for #struct_name #ty_generics #where_clause {
            fn description(&self) -> &'static str {
                #full_description_lit
            }
            fn signature(&self) -> ::toors::ToolMetadata {
                // Build the signature at runtime by collecting each field’s name and type.
                let field_sigs: Vec<String> = vec![ #(#field_sig_exprs),* ];
                let signature = field_sigs.join(", ");
                ::toors::ToolMetadata {
                    name: stringify!(#struct_name).to_string(),
                    description: #full_description_lit.to_string(),
                    signature,
                }
            }
        }
    };

    TokenStream::from(expanded)
}


/// The attribute macro remains largely the same.
/// It collects method docs and creates a helper method `tools()`.
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
                // Extract doc comments for the method.
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
                    map.insert(#name.to_string(), ::toors::ToolMetadata {
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
            pub fn tools() -> std::collections::HashMap<String, ::toors::ToolMetadata> {
                let mut map = std::collections::HashMap::new();
                #(#meta_tokens)*
                map
            }
        }
    };

    TokenStream::from(expanded)
}

