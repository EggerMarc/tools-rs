extern crate proc_macro;
use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::{parse_macro_input, FnArg, ImplItem, ItemImpl, Pat, ReturnType, Type};

#[proc_macro_attribute]
pub fn tool(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut impl_block = parse_macro_input!(item as ItemImpl);
    let struct_name = &impl_block.self_ty;
    let struct_name_str = struct_name.to_token_stream().to_string();
    let mut description = String::new();
    let mut signature_parts = Vec::new();

    for attr in &impl_block.attrs {
        if attr.path().is_ident("doc") {
            let tokens = attr.to_token_stream().to_string();
            if let Some(content) = tokens.split('"').nth(1) {
                if !description.is_empty() {
                    description.push('\n');
                }
                description.push_str(content.trim());
            }
        }
    }
    // Find the call method and extract metadata
    for item in impl_block.items.iter_mut() {
        if let ImplItem::Fn(method) = item {
            if method.sig.ident == "call" {
                // Also extract doc comments from the call method
                method.attrs.retain(|attr| {
                    if attr.path().is_ident("doc") {
                        if let Ok(syn::Lit::Str(lit_str)) = attr.parse_args() {
                            let line = lit_str.value().trim().to_string();
                            if !description.is_empty() {
                                description.push('\n');
                            }
                            description.push_str(&line);
                        }
                        false
                    } else {
                        true
                    }
                });

                // Build signature (rest remains the same)
                let mut params = Vec::new();
                for input in &method.sig.inputs {
                    if let FnArg::Typed(pat_type) = input {
                        let name = match &*pat_type.pat {
                            Pat::Ident(ident) => ident.ident.to_string(),
                            _ => "*".to_string(),
                        };
                        let ty = pat_type.ty.to_token_stream().to_string();
                        params.push(format!("{}: {}", name, ty));
                    }
                }
                let return_type = match &method.sig.output {
                    ReturnType::Type(_, ty) => ty.to_token_stream().to_string(),
                    _ => "()".to_string(),
                };
                signature_parts.push(format!(
                    "{}::call({}) -> {}",
                    struct_name_str,
                    params.join(", "),
                    return_type
                ));
            }
        }
    }

    let description = description.trim().to_string();
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
