use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{ItemFn, LitStr, parse_macro_input};

/// Attribute to turn any async fn into a ToolCollection entry.
///
/// ```
/// use toors_macros::tool;
///
/// #[tool]
/// async fn greet(name: String) -> String {
///     format!("Hello, {name}!")
/// }
/// ```
#[proc_macro_attribute]
pub fn tool(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let fn_name = &input.sig.ident;
    let fn_name_s = fn_name.to_string();

    /* ---- collect `///` doc lines and turn them into a literal -------- */
    let docs_collected = input
        .attrs
        .iter()
        .filter_map(|attr| {
            if attr.path().is_ident("doc") {
                // Result -> Option -> map to the inner string
                attr.parse_args::<LitStr>().ok().map(|lit| lit.value())
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    // convert the string into a *literal* so inventory gets a &'static str
    let doc_lit = LitStr::new(&docs_collected, Span::call_site());
    /* ------------------------------------------------------------------ */

    TokenStream::from(quote! {
        #input   // keep the user’s function unchanged

        inventory::submit! {
            toors::ToolRegistration::new(
                #fn_name_s,
                #doc_lit,
                |v| Box::pin(async move {
                    // JSON → arg
                    let arg = serde_json::from_value(v).map_err(|e| {
                        toors::toors_errors::ToolError::Deserialize(
                            toors::toors_errors::DeserializationError(
                                std::borrow::Cow::Owned(e.to_string())
                            )
                        )
                    })?;
                    // run user fn
                    let out = #fn_name(arg).await;
                    // arg → JSON
                    serde_json::to_value(out)
                        .map_err(|e| toors::toors_errors::ToolError::Runtime(e.to_string()))
                })
            )
        }
    })
}
