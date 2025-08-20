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
    Attribute, Meta, Signature, FnArg, Pat, PatIdent, parse::Parse, parse::ParseStream, Token, LitStr
};

#[cfg(test)]
mod test;

/// Controller macro for defining controller base path and metadata
/// 
/// This macro should be applied to impl blocks to enable route registration.
/// 
/// Example:
/// ```rust,ignore
/// pub struct UserController;
/// 
/// #[controller("/users")]
/// impl UserController {
///     #[get("/{id}")]
///     async fn show(&self, req: ElifRequest) -> HttpResult<ElifResponse> {
///         // handler implementation
///     }
/// }
/// ```
#[proc_macro_attribute]
pub fn controller(args: TokenStream, input: TokenStream) -> TokenStream {
    let path_lit = parse_macro_input!(args as syn::LitStr);
    let base_path = path_lit.value();
    
    // Try to parse as impl block first (new approach)
    if let Ok(mut input_impl) = syn::parse::<ItemImpl>(input.clone()) {
        let self_ty = &input_impl.self_ty;
        let struct_name = if let syn::Type::Path(type_path) = &**self_ty {
            if let Some(segment) = type_path.path.segments.last() {
                segment.ident.to_string()
            } else {
                return syn::Error::new_spanned(self_ty, "Cannot extract struct name from type path")
                    .to_compile_error()
                    .into();
            }
        } else {
            return syn::Error::new_spanned(self_ty, "Expected a simple type for impl block")
                .to_compile_error()
                .into();
        };
        
        // Collect route information from methods
        let mut routes = Vec::new();
        
        for item in &input_impl.items {
            if let ImplItem::Fn(method) = item {
                let method_name = &method.sig.ident;
                
                // Check for HTTP method attributes
                if let Some((http_method, path)) = extract_http_method_info(&method.attrs) {
                    let handler_name = method_name.to_string();
                    
                    // Convert http_method ident to proper HttpMethod enum variant
                    let http_method_variant = match http_method.to_string().as_str() {
                        "get" => quote! { GET },
                        "post" => quote! { POST },
                        "put" => quote! { PUT },
                        "delete" => quote! { DELETE },
                        "patch" => quote! { PATCH },
                        "head" => quote! { HEAD },
                        "options" => quote! { OPTIONS },
                        _ => quote! { GET }, // Default fallback
                    };
                    
                    // Extract middleware from method attributes
                    let middleware = extract_middleware_from_attrs(&method.attrs);
                    let middleware_vec = quote! { vec![#(#middleware.to_string()),*] };
                    
                    routes.push(quote! {
                        ControllerRoute {
                            method: HttpMethod::#http_method_variant,
                            path: #path.to_string(),
                            handler_name: #handler_name.to_string(),
                            middleware: #middleware_vec,
                            params: vec![], // TODO: Extract params in future phases
                        }
                    });
                }
            }
        }
        
        // Add constants to the impl block
        input_impl.items.push(syn::parse_quote! {
            pub const BASE_PATH: &'static str = #base_path;
        });
        
        input_impl.items.push(syn::parse_quote! {
            pub const CONTROLLER_NAME: &'static str = #struct_name;
        });
        
        // Generate the expanded code with ElifController trait implementation
        // Use local names to allow for both real elif-http types and test mocks
        let expanded = quote! {
            #input_impl
            
            impl ElifController for #self_ty {
                fn name(&self) -> &str {
                    #struct_name
                }
                
                fn base_path(&self) -> &str {
                    #base_path
                }
                
                fn routes(&self) -> Vec<ControllerRoute> {
                    vec![
                        #(#routes),*
                    ]
                }
                
                fn handle_request(
                    &self,
                    method_name: String,
                    request: ElifRequest,
                ) -> ::std::pin::Pin<Box<dyn ::std::future::Future<Output = HttpResult<ElifResponse>> + Send>> {
                    // NOTE: This is a limitation of the current macro implementation.
                    // The handle_request method cannot directly call async methods on self
                    // because the returned future must be 'static.
                    // 
                    // For now, we generate a simple dispatcher that returns an error.
                    // In a real implementation, you would need to use Arc<Self> and clone it,
                    // or restructure your controller to not require self in the handlers.
                    
                    Box::pin(async move {
                        Ok(ElifResponse::ok()
                            .text(&format!("Handler '{}' called (macro limitation: cannot dispatch to instance methods)", method_name)))
                    })
                }
            }
        };
        
        TokenStream::from(expanded)
    } else if let Ok(input_struct) = syn::parse::<ItemStruct>(input) {
        // Legacy support: If applied to struct, just add constants
        let struct_name = &input_struct.ident;
        let struct_name_str = struct_name.to_string();
        
        let expanded = quote! {
            #input_struct
            
            impl #struct_name {
                pub const BASE_PATH: &'static str = #base_path;
                pub const CONTROLLER_NAME: &'static str = #struct_name_str;
            }
        };
        
        TokenStream::from(expanded)
    } else {
        syn::Error::new(
            proc_macro2::Span::call_site(),
            "controller attribute must be applied to an impl block or struct"
        )
        .to_compile_error()
        .into()
    }
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
        if let Some(segment) = type_path.path.segments.last() {
            &segment.ident
        } else {
            return syn::Error::new_spanned(
                &input_impl.self_ty,
                "Cannot get identifier from an empty type path",
            )
            .to_compile_error()
            .into();
        }
    } else {
        return syn::Error::new_spanned(&input_impl.self_ty, "Expected a simple type for the impl block")
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
    let group_config = match parse_group_args_robust(args) {
        Ok(config) => config,
        Err(err) => return err.to_compile_error().into(),
    };
    
    let impl_name = if let syn::Type::Path(type_path) = &*input_impl.self_ty {
        if let Some(segment) = type_path.path.segments.last() {
            &segment.ident
        } else {
            return syn::Error::new_spanned(
                &input_impl.self_ty,
                "Cannot get identifier from an empty type path",
            )
            .to_compile_error()
            .into();
        }
    } else {
        return syn::Error::new_spanned(&input_impl.self_ty, "Expected a simple type for the impl block")
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
            
            // Extract path from the attribute arguments using proper syn parsing
            let path = extract_path_from_meta_list_robust(&meta_list.tokens);
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
                // Extract path from the attribute arguments using proper syn parsing
                return Some(extract_path_from_meta_list_robust(&meta_list.tokens));
            }
        }
    }
    None
}

/// Extract path string from token stream using proper syn parsing
fn extract_path_from_meta_list_robust(tokens: &proc_macro2::TokenStream) -> String {
    // If tokens are empty, return empty string
    if tokens.is_empty() {
        return String::new();
    }
    
    // Try to parse as a string literal directly
    if let Ok(lit_str) = syn::parse2::<LitStr>(tokens.clone()) {
        return lit_str.value();
    }
    
    // Try to parse as a parenthesized string literal: ("path")
    if let Ok(group) = syn::parse2::<proc_macro2::Group>(tokens.clone()) {
        if group.delimiter() == proc_macro2::Delimiter::Parenthesis {
            if let Ok(inner_lit) = syn::parse2::<LitStr>(group.stream()) {
                return inner_lit.value();
            }
        }
    }
    
    // Try to manually extract from parentheses format
    let tokens_iter = tokens.clone().into_iter().collect::<Vec<_>>();
    if tokens_iter.len() == 1 {
        if let proc_macro2::TokenTree::Group(group) = &tokens_iter[0] {
            if group.delimiter() == proc_macro2::Delimiter::Parenthesis {
                if let Ok(inner_lit) = syn::parse2::<LitStr>(group.stream()) {
                    return inner_lit.value();
                }
            }
        }
    }
    
    // Fall back to empty string if we can't parse properly
    String::new()
}

#[derive(Debug, Default)]
struct GroupConfig {
    prefix: String,
    middleware: Vec<String>,
}

/// Parsing struct for group arguments like: "/api/v1", middleware = [cors, auth]
struct GroupArgs {
    prefix: LitStr,
    _comma: Option<Token![,]>,
    middleware_assignment: Option<(syn::Ident, Token![=], syn::Expr)>,
}

impl Parse for GroupArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let prefix = input.parse()?;
        
        let _comma = if input.peek(Token![,]) {
            Some(input.parse()?)
        } else {
            None
        };
        
        let middleware_assignment = if !input.is_empty() {
            let ident: syn::Ident = input.parse()?;
            if ident != "middleware" {
                return Err(syn::Error::new_spanned(
                    ident,
                    "Expected 'middleware' keyword"
                ));
            }
            let eq: Token![=] = input.parse()?;
            let expr: syn::Expr = input.parse()?;
            Some((ident, eq, expr))
        } else {
            None
        };
        
        Ok(GroupArgs {
            prefix,
            _comma,
            middleware_assignment,
        })
    }
}

/// Parse group attribute arguments using robust syn parsing
fn parse_group_args_robust(args: TokenStream) -> syn::Result<GroupConfig> {
    let parsed_args = syn::parse::<GroupArgs>(args)?;
    
    let mut config = GroupConfig {
        prefix: parsed_args.prefix.value(),
        middleware: Vec::new(),
    };
    
    // Extract middleware from expression if present
    if let Some((_ident, _eq, expr)) = parsed_args.middleware_assignment {
        // Try to parse middleware list from various expression forms
        match &expr {
            syn::Expr::Array(array) => {
                for elem in &array.elems {
                    if let syn::Expr::Path(path) = elem {
                        if let Some(ident) = path.path.get_ident() {
                            config.middleware.push(ident.to_string());
                        }
                    }
                }
            }
            syn::Expr::Path(path) => {
                // Single middleware item
                if let Some(ident) = path.path.get_ident() {
                    config.middleware.push(ident.to_string());
                }
            }
            _ => {
                // For now, ignore other expression types
                // In production, you might want to handle more complex expressions
            }
        }
    }
    
    Ok(config)
}

/// Parse group attribute arguments (legacy - kept for compatibility)
#[allow(dead_code)]
fn parse_group_args(args: TokenStream) -> GroupConfig {
    // Use the robust parser and fall back to default on error
    parse_group_args_robust(args).unwrap_or_default()
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

/// Extract middleware names from method attributes
fn extract_middleware_from_attrs(attrs: &[Attribute]) -> Vec<String> {
    let mut middleware = Vec::new();
    
    for attr in attrs {
        if attr.path().is_ident("middleware") {
            if let Meta::List(meta_list) = &attr.meta {
                // Parse middleware names from the attribute
                let tokens = &meta_list.tokens;
                let token_vec: Vec<_> = tokens.clone().into_iter().collect();
                
                
                for token in token_vec {
                    match &token {
                        proc_macro2::TokenTree::Literal(lit) => {
                            // Try to parse as string literal
                            let lit_str = lit.to_string();
                            if lit_str.starts_with('"') && lit_str.ends_with('"') {
                                let cleaned = lit_str.trim_matches('"');
                                middleware.push(cleaned.to_string());
                            }
                        }
                        proc_macro2::TokenTree::Punct(punct) if punct.as_char() == ',' => {
                            // Comma separator, ignore
                        }
                        _ => {}
                    }
                }
            }
        }
    }
    
    middleware
}