extern crate proc_macro;
use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use std::collections::HashMap;
use syn::{parse_macro_input, FnArg, ImplItem, ItemImpl, Pat, ReturnType};
use toors::ToolMetadata;

#[proc_macro_attribute]
pub fn tools(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut impl_block = parse_macro_input!(item as ItemImpl);
    let struct_name = &impl_block.self_ty;
    let struct_name_str = struct_name.to_token_stream().to_string();
    let mut description_vec = String::new();
    let mut signature_parts = Vec::new();
    let collection: HashMap<String, ToolMetadata> = HashMap::new();

    for attr in &impl_block.attrs {
        if attr.path().is_ident("doc") {
            let tokens = attr.to_token_stream().to_string();
            if let Some(content) = tokens.split('"').nth(1) {
                if !description_vec.is_empty() {
                    description_vec.push('\n');
                }
                description_vec.push_str(content.trim());
            }
        }
    }
    // Find the call method and extract metadata
    for item in impl_block.items.iter_mut() {
        if let ImplItem::Fn(method) = item {
            let name = method.sig.ident.clone().to_string();
            let description = method
                .attrs
                .iter()
                .filter(|attr| attr.path().is_ident("doc"))
                .filter_map(|attr| {
                    attr.parse_args::<syn::LitStr>()
                        .ok()
                        .map(|lit_str| lit_str.value().trim().to_string())
                })
                .collect::<Vec<_>>()
                .join("\n");

            let params = method
                .sig
                .inputs
                .iter()
                .map(|input| {
                    if let FnArg::Typed(pat_type) = input {
                        let name = match &*pat_type.pat {
                            Pat::Ident(ident) => ident.ident.to_string(),
                            _ => "*".to_string(),
                        };
                        let ty = pat_type.ty.to_token_stream().to_string();
                        format!("{}: {}", name, ty)
                    } else {
                        String::new()
                    }
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
            signature_parts.push(signature.clone());

            let meta = ToolMetadata::from(description, signature.to_owned());
        }
    }

    let description = description_vec.trim().to_string();
    let signature = signature_parts.join("\n");
    let expanded = quote! {
        #impl_block

        impl toors::Tool for #struct_name {

            fn description() -> &'static str {
                #description
            }

            fn signature() -> &'static str {
                #signature
            }
        }
    };
    TokenStream::from(expanded)
}


               

