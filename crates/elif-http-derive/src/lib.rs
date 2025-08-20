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
//! - `#[routes]`: Generate route registration code from impl blocks
//! - `#[resource]`: Automatic RESTful resource registration
//! - `#[group]`: Route grouping with shared attributes

use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse_macro_input, ItemFn, ItemStruct, ItemImpl, ImplItem,
    Attribute, Meta, Signature, FnArg, Pat, PatIdent
};

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
    let path_lit = parse_macro_input!(args as syn::LitStr);
    let input = parse_macro_input!(input as ItemStruct);
    
    // Extract base path from the string literal
    let base_path = path_lit.value();
    
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

/// Generate HTTP method macros with parameter extraction
macro_rules! http_method_macro {
    ($method:literal) => {
        |args: TokenStream, input: TokenStream| -> TokenStream {
            // Parse route path - can be empty for root routes
            let route_path = if args.is_empty() {
                "".to_string()
            } else {
                let path_lit = parse_macro_input!(args as syn::LitStr);
                path_lit.value()
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
/// Usage: #[middleware("auth")] or #[middleware("auth", "logging")]
#[proc_macro_attribute]
pub fn middleware(args: TokenStream, input: TokenStream) -> TokenStream {
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
#[proc_macro_attribute]
pub fn routes(_args: TokenStream, input: TokenStream) -> TokenStream {
    let input_impl = parse_macro_input!(input as ItemImpl);
    let impl_name = if let syn::Type::Path(type_path) = &*input_impl.self_ty {
        &type_path.path.segments.last().unwrap().ident
    } else {
        return syn::Error::new_spanned(&input_impl.self_ty, "Expected simple type path")
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
#[proc_macro_attribute]
pub fn resource(args: TokenStream, input: TokenStream) -> TokenStream {
    let path_lit = parse_macro_input!(args as syn::LitStr);
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

/// Route group macro for grouping routes with shared attributes
/// 
/// Groups routes under a common prefix with shared middleware.
/// 
/// Example:
/// ```rust,ignore
/// #[group("/api/v1", middleware = [cors, auth])]
/// impl ApiV1Routes {
///     #[get("/profile")]
///     fn profile() -> HttpResult<ElifResponse> { ... }
/// }
/// ```
#[proc_macro_attribute]
pub fn group(args: TokenStream, input: TokenStream) -> TokenStream {
    let input_impl = parse_macro_input!(input as ItemImpl);
    
    // Parse group arguments (path and optional middleware)
    let group_config = parse_group_args(args);
    
    let impl_name = if let syn::Type::Path(type_path) = &*input_impl.self_ty {
        &type_path.path.segments.last().unwrap().ident
    } else {
        return syn::Error::new_spanned(&input_impl.self_ty, "Expected simple type path")
            .to_compile_error()
            .into();
    };
    
    let prefix = group_config.prefix;
    let _middleware_items = group_config.middleware.iter().map(|mw| {
        quote! { group = group.middleware(#mw); }
    });
    
    let mut route_registrations = Vec::new();
    
    // Process methods in the group
    for item in &input_impl.items {
        if let ImplItem::Fn(method) = item {
            let method_name = &method.sig.ident;
            
            if let Some((http_method, path)) = extract_http_method_info(&method.attrs) {
                let full_path = if path.is_empty() { "" } else { &path };
                route_registrations.push(quote! {
                    group = group.#http_method(#full_path, Self::#method_name);
                });
            }
        }
    }
    
    let route_count = route_registrations.len();
    
    let expanded = quote! {
        #input_impl
        
        impl #impl_name {
            /// Generated route group setup function
            pub fn build_group() -> String {
                // In production, this would return a proper RouteGroup instance
                // For now, return a string for testing purposes
                format!("RouteGroup at {} with {} routes", #prefix, #route_count)
            }
        }
    };
    
    TokenStream::from(expanded)
}

// Helper functions for attribute parsing

/// Extract HTTP method and path from method attributes
fn extract_http_method_info(attrs: &[Attribute]) -> Option<(proc_macro2::Ident, String)> {
    for attr in attrs {
        if let Meta::List(meta_list) = &attr.meta {
            let path_name = meta_list.path.get_ident()?.to_string();
            let method_ident = match path_name.as_str() {
                "get" => Some(quote::format_ident!("get")),
                "post" => Some(quote::format_ident!("post")),
                "put" => Some(quote::format_ident!("put")),
                "delete" => Some(quote::format_ident!("delete")),
                "patch" => Some(quote::format_ident!("patch")),
                "head" => Some(quote::format_ident!("head")),
                "options" => Some(quote::format_ident!("options")),
                _ => None,
            }?;
            
            // Extract path from the attribute arguments
            let path = extract_path_from_meta_list(&meta_list.tokens);
            return Some((method_ident, path));
        } else if let Meta::Path(path) = &attr.meta {
            let path_name = path.get_ident()?.to_string();
            let method_ident = match path_name.as_str() {
                "get" => Some(quote::format_ident!("get")),
                "post" => Some(quote::format_ident!("post")),
                "put" => Some(quote::format_ident!("put")),
                "delete" => Some(quote::format_ident!("delete")),
                "patch" => Some(quote::format_ident!("patch")),
                "head" => Some(quote::format_ident!("head")),
                "options" => Some(quote::format_ident!("options")),
                _ => None,
            }?;
            
            return Some((method_ident, "".to_string()));
        }
    }
    None
}

/// Extract resource path from method attributes  
fn extract_resource_info(attrs: &[Attribute]) -> Option<String> {
    for attr in attrs {
        if let Meta::List(meta_list) = &attr.meta {
            if meta_list.path.is_ident("resource") {
                // Extract path from the attribute arguments
                return Some(extract_path_from_meta_list(&meta_list.tokens));
            }
        }
    }
    None
}

/// Extract path string from token stream (handles both ("path") and empty cases)
fn extract_path_from_meta_list(tokens: &proc_macro2::TokenStream) -> String {
    let tokens_str = tokens.to_string();
    // Simple string extraction - in production would need more robust parsing
    if tokens_str.is_empty() {
        "".to_string()
    } else if tokens_str.starts_with("(\"") && tokens_str.ends_with("\")") {
        tokens_str[2..tokens_str.len()-2].to_string()
    } else if tokens_str.starts_with("\"") && tokens_str.ends_with("\"") {
        tokens_str[1..tokens_str.len()-1].to_string()  
    } else {
        "".to_string()
    }
}

#[derive(Debug, Default)]
struct GroupConfig {
    prefix: String,
    middleware: Vec<String>,
}

/// Parse group attribute arguments
fn parse_group_args(args: TokenStream) -> GroupConfig {
    let mut config = GroupConfig::default();
    
    // Simple parsing for the example format: "/api/v1", middleware = [cors, auth]
    let args_str = args.to_string();
    
    // Extract prefix (first string literal)
    if let Some(start) = args_str.find('"') {
        if let Some(end) = args_str[start + 1..].find('"') {
            config.prefix = args_str[start + 1..start + 1 + end].to_string();
        }
    }
    
    // Extract middleware (simplified parsing)
    if args_str.contains("middleware") {
        // This would need more sophisticated parsing in production
        // For now, just acknowledge the presence
    }
    
    config
}

/// Extract path parameters from a route path (e.g., "/users/{id}" -> ["id"])
fn extract_path_parameters(path: &str) -> Vec<String> {
    let mut params = Vec::new();
    let mut chars = path.chars().peekable();
    
    while let Some(ch) = chars.next() {
        if ch == '{' {
            let mut param = String::new();
            for ch in chars.by_ref() {
                if ch == '}' {
                    if !param.is_empty() {
                        params.push(param);
                    }
                    break;
                } else {
                    param.push(ch);
                }
            }
        }
    }
    
    params
}

/// Extract function parameter names and types from a function signature
fn extract_function_parameters(sig: &Signature) -> Vec<(String, String)> {
    let mut params = Vec::new();
    
    for input in &sig.inputs {
        if let FnArg::Typed(pat_type) = input {
            if let Pat::Ident(PatIdent { ident, .. }) = pat_type.pat.as_ref() {
                let param_name = ident.to_string();
                let param_type = quote! { #pat_type.ty }.to_string();
                params.push((param_name, param_type));
            }
        }
    }
    
    params
}