//! Route registration and resource macros
//! 
//! Provides #[routes] and #[resource] macros for route organization.

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemImpl, ItemFn, ImplItem, LitStr};

use crate::utils::{extract_http_method_info, extract_resource_info};

/// Route registration macro for impl blocks
/// 
/// Generates route registration code from an impl block containing route methods.
/// This macro scans all methods in the impl block and generates a router setup function.
/// 
/// Example:
/// ```rust,ignore
/// #[routes]
/// impl ApiRoutes {
///     #[get("/health")]
///     fn health() -> HttpResult<ElifResponse> {
///         Ok(response().json(json!({"status": "ok"})))
///     }
///     
///     #[resource("/users")]
///     fn users() -> UserController { UserController::new() }
/// }
/// ```
pub fn routes_impl(_args: TokenStream, input: TokenStream) -> TokenStream {
    let input_impl = parse_macro_input!(input as ItemImpl);
    let impl_name = if let syn::Type::Path(type_path) = &*input_impl.self_ty {
        if let Some(segment) = type_path.path.segments.last() {
            &segment.ident
        } else {
            return syn::Error::new_spanned(
                &input_impl.self_ty,
                "Cannot get identifier from an empty type path. Hint: Use a proper struct name like `impl MyRoutes`.",
            )
            .to_compile_error()
            .into();
        }
    } else {
        return syn::Error::new_spanned(
            &input_impl.self_ty, 
            "Expected a simple type for the impl block. Hint: Apply #[routes] to `impl MyStruct { ... }` not complex types."
        )
        .to_compile_error()
        .into();
    };
    
    let mut route_registrations = Vec::new();
    let mut resource_registrations = Vec::new();
    
    // Process each method in the impl block
    for item in &input_impl.items {
        if let ImplItem::Fn(method) = item {
            let method_name = &method.sig.ident;
            
            // Check for HTTP method attributes
            if let Some((http_method, path)) = extract_http_method_info(&method.attrs) {
                let path = if path.is_empty() { "/" } else { &path };
                route_registrations.push(quote! {
                    router = router.#http_method(#path, Self::#method_name);
                });
            }
            
            // Check for resource attribute
            if let Some(resource_path) = extract_resource_info(&method.attrs) {
                resource_registrations.push(quote! {
                    router = router.resource(#resource_path, Self::#method_name());
                });
            }
        }
    }
    
    // Count routes and resources
    let route_count = route_registrations.len();
    let resource_count = resource_registrations.len();
    
    // Generate the router build function
    let expanded = quote! {
        #input_impl
        
        impl #impl_name {
            /// Generated router setup function  
            pub fn build_router() -> String {
                // In production, this would return a proper router instance
                // For now, return a string for testing purposes
                format!("Router with {} routes and {} resources", #route_count, #resource_count)
            }
        }
    };
    
    TokenStream::from(expanded)
}

/// Resource macro for automatic RESTful resource registration
/// 
/// Registers a controller as a RESTful resource with standard routes.
/// 
/// Example:
/// ```rust,ignore
/// #[resource("/users")]
/// fn user_routes() -> UserController { UserController::new() }
/// ```
pub fn resource_impl(args: TokenStream, input: TokenStream) -> TokenStream {
    let path_lit = parse_macro_input!(args as LitStr);
    let input_fn = parse_macro_input!(input as ItemFn);
    
    let resource_path = path_lit.value();
    
    let func_name = &input_fn.sig.ident;
    let resource_path_fn = quote::format_ident!("{}_resource_path", func_name);
    
    // Add resource attribute metadata to the function
    let expanded = quote! {
        #[doc = concat!("RESTful resource at: ", #resource_path)]
        #input_fn
        
        // Generate a helper function with resource metadata
        #[allow(non_snake_case)]
        pub fn #resource_path_fn() -> &'static str {
            #resource_path
        }
    };
    
    TokenStream::from(expanded)
}