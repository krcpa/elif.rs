//! Middleware application macros
//!
//! Provides #[middleware] macro for applying middleware to controllers and methods.

use proc_macro::TokenStream;
use quote::quote;
use syn::{ItemFn, ItemStruct};

/// Middleware application macro
///
/// Can be applied to controllers (affects all routes) or individual methods
/// Usage: #[middleware("auth")] or #[middleware("auth", "logging")]
pub fn middleware_impl(args: TokenStream, input: TokenStream) -> TokenStream {
    // Parse middleware names - for now just acknowledge them
    let _middleware_args = if !args.is_empty() {
        // In a full implementation, would parse comma-separated string literals
        // For now, just consume the args
        Some(args.to_string())
    } else {
        None
    };

    // Convert to proc_macro2::TokenStream for cloning support
    let input_tokens: proc_macro2::TokenStream = input.into();

    // Try to parse as function first, then as struct
    if let Ok(input_fn) = syn::parse2::<ItemFn>(input_tokens.clone()) {
        // Method-level middleware
        let expanded = quote! {
            #input_fn
        };
        TokenStream::from(expanded)
    } else if let Ok(input_struct) = syn::parse2::<ItemStruct>(input_tokens.clone()) {
        // Controller-level middleware
        let expanded = quote! {
            #input_struct
        };
        TokenStream::from(expanded)
    } else {
        // Return original input if we can't parse it
        input_tokens.into()
    }
}
