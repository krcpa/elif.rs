//! # elif-http-derive
//! 
//! Derive macros for elif-http declarative routing and controller system.
//! 
//! This crate provides procedural macros to simplify controller development:
//! - `#[controller]`: Define controller base path and metadata
//! - `#[get]`, `#[post]`, etc.: HTTP method routing macros
//! - `#[middleware]`: Apply middleware to controllers and methods
//! - `#[param]`: Route parameter specifications
//! - `#[body]`: Request body type specifications

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn, ItemStruct, Lit, Meta, Token};

#[cfg(test)]
mod test;

/// Controller macro for defining controller base path and metadata
/// 
/// Example:
/// ```rust,ignore
/// #[controller("/users")]
/// pub struct UserController {
///     user_service: Arc<UserService>,
/// }
/// ```
#[proc_macro_attribute]
pub fn controller(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args with syn::punctuated::Punctuated::<Meta, Token![,]>::parse_terminated);
    let input = parse_macro_input!(input as ItemStruct);
    
    // Extract base path - simplified for now to just take string literals
    let base_path = args.iter()
        .find_map(|meta| match meta {
            Meta::Path(path) => path.get_ident().map(|i| i.to_string()),
            Meta::NameValue(nv) if matches!(&nv.value, syn::Expr::Lit(syn::ExprLit { lit: Lit::Str(_), .. })) => {
                if let syn::Expr::Lit(syn::ExprLit { lit: Lit::Str(s), .. }) = &nv.value {
                    Some(s.value())
                } else {
                    None
                }
            }
            _ => None,
        })
        .unwrap_or_else(|| "/".to_string());
    
    let struct_name = &input.ident;
    let struct_name_str = struct_name.to_string();
    
    // Generate a simple stub implementation for now
    // In a full implementation, this would integrate with the controller system
    let expanded = quote! {
        #input
        
        // This is a placeholder implementation
        // In practice, this would generate the ElifController trait implementation
        // and integrate with the routing system
        impl #struct_name {
            pub const BASE_PATH: &'static str = #base_path;
            pub const CONTROLLER_NAME: &'static str = #struct_name_str;
        }
    };
    
    TokenStream::from(expanded)
}

/// Generate HTTP method macros
macro_rules! http_method_macro {
    ($method:literal) => {
        |args: TokenStream, input: TokenStream| -> TokenStream {
            let _args = parse_macro_input!(args with syn::punctuated::Punctuated::<Meta, Token![,]>::parse_terminated);
            let input_fn = parse_macro_input!(input as ItemFn);
            
            // For now, just return the original function with a marker
            // In a full implementation, this would register route information
            let expanded = quote! {
                #[allow(dead_code)]
                #input_fn
            };
            
            TokenStream::from(expanded)
        }
    };
}

/// GET method routing macro
#[proc_macro_attribute]
pub fn get(args: TokenStream, input: TokenStream) -> TokenStream {
    http_method_macro!("GET")(args, input)
}

/// POST method routing macro
#[proc_macro_attribute]
pub fn post(args: TokenStream, input: TokenStream) -> TokenStream {
    http_method_macro!("POST")(args, input)
}

/// PUT method routing macro
#[proc_macro_attribute]
pub fn put(args: TokenStream, input: TokenStream) -> TokenStream {
    http_method_macro!("PUT")(args, input)
}

/// DELETE method routing macro
#[proc_macro_attribute]
pub fn delete(args: TokenStream, input: TokenStream) -> TokenStream {
    http_method_macro!("DELETE")(args, input)
}

/// PATCH method routing macro
#[proc_macro_attribute]
pub fn patch(args: TokenStream, input: TokenStream) -> TokenStream {
    http_method_macro!("PATCH")(args, input)
}

/// HEAD method routing macro
#[proc_macro_attribute]
pub fn head(args: TokenStream, input: TokenStream) -> TokenStream {
    http_method_macro!("HEAD")(args, input)
}

/// OPTIONS method routing macro
#[proc_macro_attribute]
pub fn options(args: TokenStream, input: TokenStream) -> TokenStream {
    http_method_macro!("OPTIONS")(args, input)
}

/// Middleware application macro
/// 
/// Can be applied to controllers (affects all routes) or individual methods
#[proc_macro_attribute]
pub fn middleware(args: TokenStream, input: TokenStream) -> TokenStream {
    let _args = parse_macro_input!(args with syn::punctuated::Punctuated::<Meta, Token![,]>::parse_terminated);
    
    // Try to parse as function first, then as struct
    let input_clone = input.clone();
    if let Ok(input_fn) = syn::parse::<ItemFn>(input_clone) {
        // Method-level middleware
        let expanded = quote! {
            #input_fn
        };
        TokenStream::from(expanded)
    } else if let Ok(input_struct) = syn::parse::<ItemStruct>(input.clone()) {
        // Controller-level middleware
        let expanded = quote! {
            #input_struct
        };
        TokenStream::from(expanded)
    } else {
        // Return original input if we can't parse it
        input
    }
}

/// Route parameter specification macro
#[proc_macro_attribute]
pub fn param(_args: TokenStream, input: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(input as ItemFn);
    
    let expanded = quote! {
        #input_fn
    };
    
    TokenStream::from(expanded)
}

/// Request body specification macro
#[proc_macro_attribute]
pub fn body(_args: TokenStream, input: TokenStream) -> TokenStream {
    // For now, this is just a marker - the actual body parsing happens
    // in the handler using req.json::<T>() or similar methods
    input
}