//! HTTP method macros implementation
//! 
//! Provides #[get], #[post], #[put], #[delete], #[patch], #[head], #[options] macros.

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn};

use crate::utils::{validate_route_path, extract_path_parameters, extract_function_parameters};

/// Generate HTTP method macros with parameter extraction
pub fn http_method_macro_impl(method: &str, args: TokenStream, input: TokenStream) -> TokenStream {
    // Parse route path - can be empty for root routes
    let route_path = if args.is_empty() {
        "".to_string()
    } else {
        let path_lit = match syn::parse::<syn::LitStr>(args) {
            Ok(lit) => lit,
            Err(_) => {
                return syn::Error::new(
                    proc_macro2::Span::call_site(),
                    format!("Invalid path argument for {} macro. Hint: Use a string literal like #[{}(\"/users/{{id}}\")]", method, method.to_lowercase())
                )
                .to_compile_error()
                .into();
            }
        };
        let path = path_lit.value();
        
        // Validate path format
        if let Err(msg) = validate_route_path(&path) {
            return syn::Error::new_spanned(
                &path_lit,
                format!("Invalid route path '{}': {}. Hint: Use format like '/users/{{id}}' with proper parameter syntax.", path, msg)
            )
            .to_compile_error()
            .into();
        }
        
        path
    };
    let input_fn = parse_macro_input!(input as ItemFn);
    
    // Extract path parameters from the route and function signature
    let path_params = extract_path_parameters(&route_path);
    let _fn_params = extract_function_parameters(&input_fn.sig);
    
    // Generate parameter validation comments
    let param_docs = if !path_params.is_empty() {
        let param_list = path_params.join(", ");
        format!("Route parameters: {}", param_list)
    } else {
        "No route parameters".to_string()
    };
    
    // Return the function with enhanced documentation
    let expanded = quote! {
        #[doc = #param_docs]
        #[allow(dead_code)]
        #input_fn
    };
    
    TokenStream::from(expanded)
}

/// GET method routing macro
pub fn get_impl(args: TokenStream, input: TokenStream) -> TokenStream {
    http_method_macro_impl("GET", args, input)
}

/// POST method routing macro
pub fn post_impl(args: TokenStream, input: TokenStream) -> TokenStream {
    http_method_macro_impl("POST", args, input)
}

/// PUT method routing macro
pub fn put_impl(args: TokenStream, input: TokenStream) -> TokenStream {
    http_method_macro_impl("PUT", args, input)
}

/// DELETE method routing macro
pub fn delete_impl(args: TokenStream, input: TokenStream) -> TokenStream {
    http_method_macro_impl("DELETE", args, input)
}

/// PATCH method routing macro
pub fn patch_impl(args: TokenStream, input: TokenStream) -> TokenStream {
    http_method_macro_impl("PATCH", args, input)
}

/// HEAD method routing macro
pub fn head_impl(args: TokenStream, input: TokenStream) -> TokenStream {
    http_method_macro_impl("HEAD", args, input)
}

/// OPTIONS method routing macro
pub fn options_impl(args: TokenStream, input: TokenStream) -> TokenStream {
    http_method_macro_impl("OPTIONS", args, input)
}