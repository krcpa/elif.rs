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
    Attribute, Meta, Signature, FnArg, Pat, PatIdent, parse::Parse, parse::ParseStream, Token, LitStr,
    Ident, Type
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
                return syn::Error::new_spanned(
                    self_ty, 
                    "Cannot extract struct name from type path. Hint: Use a simple struct name like `MyController`."
                )
                .to_compile_error()
                .into();
            }
        } else {
            return syn::Error::new_spanned(
                self_ty, 
                "Expected a simple type for impl block. Hint: Apply #[controller] to `impl MyStruct { ... }` not complex types."
            )
            .to_compile_error()
            .into();
        };
        
        // Collect route information from methods
        let mut routes = Vec::new();
        let mut method_handlers = Vec::new();
        
        for item in &input_impl.items {
            if let ImplItem::Fn(method) = item {
                let method_name = &method.sig.ident;
                
                // Check for HTTP method attributes
                if let Some((http_method, path)) = extract_http_method_info(&method.attrs) {
                    let handler_name = method_name.to_string();
                    let handler_name_lit = syn::LitStr::new(&handler_name, method_name.span());
                    
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
                    
                    // Generate handler for async dispatch with Arc<Self>
                    method_handlers.push(quote! {
                        #handler_name_lit => self.#method_name(request).await
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
        
        // Generate method handlers for async dispatch
        let method_match_arms = method_handlers.iter().enumerate().map(|(_, handler)| handler);
        
        // Generate the expanded code with ElifController trait implementation
        // Using async-trait for proper async method support
        let expanded = quote! {
            #input_impl
            
            #[::async_trait::async_trait]
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
                
                async fn handle_request(
                    self: std::sync::Arc<Self>,
                    method_name: String,
                    request: ElifRequest,
                ) -> HttpResult<ElifResponse> {
                    match method_name.as_str() {
                        #(#method_match_arms,)*
                        _ => {
                            Ok(ElifResponse::not_found()
                                .text(&format!("Handler '{}' not found", method_name)))
                        }
                    }
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
            "controller attribute must be applied to an impl block or struct. Hint: Use `#[controller(\"/path\")] impl MyController { ... }` or `#[controller(\"/path\")] struct MyController;`"
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
                let path_lit = match syn::parse::<syn::LitStr>(args) {
                    Ok(lit) => lit,
                    Err(_) => {
                        return syn::Error::new(
                            proc_macro2::Span::call_site(),
                            format!("Invalid path argument for {} macro. Hint: Use a string literal like #[{}(\"/users/{{id}}\")]", $method, $method.to_lowercase())
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
/// Validates that the parameter type matches what's expected from the route path
#[proc_macro_attribute]
pub fn param(args: TokenStream, input: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(input as ItemFn);
    
    // Parse parameter specification from args
    let param_spec = if !args.is_empty() {
        match syn::parse::<ParamSpec>(args) {
            Ok(spec) => Some(spec),
            Err(err) => {
                return syn::Error::new(
                    err.span(),
                    format!("Invalid param specification: {}. Hint: Use #[param(id: int)] or #[param(name: string)]", err)
                )
                .to_compile_error()
                .into();
            }
        }
    } else {
        None
    };
    
    // Validate that function signature matches param specification
    if let Some(spec) = &param_spec {
        if let Err(msg) = validate_param_consistency(&spec, &input_fn.sig) {
            return syn::Error::new_spanned(
                &input_fn.sig,
                format!("Parameter type mismatch: {}. Hint: Ensure function parameter type matches #[param] declaration.", msg)
            )
            .to_compile_error()
            .into();
        }
    }
    
    let expanded = quote! {
        #input_fn
    };
    
    TokenStream::from(expanded)
}

/// Request body specification macro
/// Validates that the handler function can accept the specified body type
#[proc_macro_attribute]
pub fn body(args: TokenStream, input: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(input as ItemFn);
    
    // Parse body type specification from args
    let body_type = if !args.is_empty() {
        match syn::parse::<Type>(args) {
            Ok(ty) => Some(ty),
            Err(err) => {
                return syn::Error::new(
                    err.span(),
                    format!("Invalid body type specification: {}. Hint: Use #[body(UserData)] where UserData is a valid type", err)
                )
                .to_compile_error()
                .into();
            }
        }
    } else {
        return syn::Error::new(
            proc_macro2::Span::call_site(),
            "Body type specification required. Hint: Use #[body(MyType)] to specify the expected request body type."
        )
        .to_compile_error()
        .into();
    };
    
    // Validate that function signature can handle the body type
    if let Some(body_ty) = &body_type {
        if let Err(msg) = validate_body_compatibility(body_ty, &input_fn.sig) {
            return syn::Error::new_spanned(
                &input_fn.sig,
                format!("Body type compatibility issue: {}. Hint: Ensure function accepts ElifJson<{}> or similar parameter.", msg, quote! { #body_ty })
            )
            .to_compile_error()
            .into();
        }
    }
    
    let expanded = quote! {
        #input_fn
    };
    
    TokenStream::from(expanded)
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
                "Cannot get identifier from an empty type path. Hint: Use a proper struct name like `impl MyGroup`.",
            )
            .to_compile_error()
            .into();
        }
    } else {
        return syn::Error::new_spanned(
            &input_impl.self_ty, 
            "Expected a simple type for the impl block. Hint: Apply #[group] to `impl MyStruct { ... }` not complex types."
        )
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

/// Parameter specification for type validation
#[derive(Debug, Clone)]
struct ParamSpec {
    name: Ident,
    param_type: ParamType,
}

/// Supported parameter types for validation
#[derive(Debug, Clone, PartialEq)]
enum ParamType {
    String,
    Int,
    UInt,
    Float,
    Bool,
    Uuid,
}

impl std::fmt::Display for ParamType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParamType::String => write!(f, "string"),
            ParamType::Int => write!(f, "int"),
            ParamType::UInt => write!(f, "uint"),
            ParamType::Float => write!(f, "float"),
            ParamType::Bool => write!(f, "bool"),
            ParamType::Uuid => write!(f, "uuid"),
        }
    }
}

impl Parse for ParamSpec {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name: Ident = input.parse()?;
        input.parse::<Token![:]>()?;
        let type_ident: Ident = input.parse()?;
        
        let param_type = match type_ident.to_string().as_str() {
            "string" => ParamType::String,
            "int" => ParamType::Int,
            "uint" => ParamType::UInt,
            "float" => ParamType::Float,
            "bool" => ParamType::Bool,
            "uuid" => ParamType::Uuid,
            _ => {
                return Err(syn::Error::new_spanned(
                    type_ident,
                    format!("Unsupported parameter type. Supported types: string, int, uint, float, bool, uuid")
                ));
            }
        };
        
        Ok(ParamSpec { name, param_type })
    }
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
                    "Expected 'middleware' keyword. Hint: Use syntax like #[group(\"/api\", middleware = [auth, cors])]"
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

/// Validate route path format and return error message if invalid
fn validate_route_path(path: &str) -> Result<(), String> {
    if path.is_empty() {
        return Ok(());
    }
    
    // Check for malformed parameter syntax
    let mut chars = path.chars().peekable();
    let mut brace_count = 0;
    let mut in_param = false;
    let mut param_content = String::new();
    
    while let Some(ch) = chars.next() {
        match ch {
            '{' => {
                if in_param {
                    return Err("nested braces are not allowed in parameters".to_string());
                }
                brace_count += 1;
                in_param = true;
                param_content.clear();
            }
            '}' => {
                if !in_param {
                    return Err("unmatched closing brace '}'".to_string());
                }
                if param_content.is_empty() {
                    return Err("empty parameter name '{}'".to_string());
                }
                if !param_content.chars().all(|c| c.is_alphanumeric() || c == '_') {
                    return Err(format!("invalid parameter name '{}' - use only alphanumeric characters and underscores", param_content));
                }
                brace_count -= 1;
                in_param = false;
            }
            _ if in_param => {
                param_content.push(ch);
            }
            _ => {}
        }
    }
    
    if brace_count != 0 {
        return Err("unmatched opening brace '{'".to_string());
    }
    
    // Check for double slashes
    if path.contains("//") {
        return Err("double slashes '//' are not allowed".to_string());
    }
    
    Ok(())
}

/// Validate that parameter specification matches function signature
fn validate_param_consistency(param_spec: &ParamSpec, sig: &Signature) -> Result<(), String> {
    let param_name = param_spec.name.to_string();
    
    // Find the parameter in the function signature
    let mut found_param = None;
    for input in &sig.inputs {
        if let FnArg::Typed(pat_type) = input {
            if let Pat::Ident(PatIdent { ident, .. }) = pat_type.pat.as_ref() {
                if ident.to_string() == param_name {
                    found_param = Some(&*pat_type.ty);
                    break;
                }
            }
        }
    }
    
    let param_type = match found_param {
        Some(ty) => ty,
        None => return Err(format!("Parameter '{}' not found in function signature", param_name)),
    };
    
    // Validate type compatibility
    let type_str = quote! { #param_type }.to_string().replace(" ", "");
    let expected_types = get_compatible_rust_types(&param_spec.param_type);
    
    if !expected_types.iter().any(|expected| type_str.contains(expected)) {
        return Err(format!(
            "Parameter '{}' has type '{}' but expected one of: {}",
            param_name,
            type_str,
            expected_types.join(", ")
        ));
    }
    
    Ok(())
}

/// Get compatible Rust types for a parameter type
fn get_compatible_rust_types(param_type: &ParamType) -> Vec<&'static str> {
    match param_type {
        ParamType::String => vec!["String", "&str", "str"],
        ParamType::Int => vec!["i32", "i64", "isize"],
        ParamType::UInt => vec!["u32", "u64", "usize"],
        ParamType::Float => vec!["f32", "f64"],
        ParamType::Bool => vec!["bool"],
        ParamType::Uuid => vec!["Uuid", "uuid::Uuid"],
    }
}

/// Validate that function signature is compatible with the specified body type
fn validate_body_compatibility(body_type: &Type, sig: &Signature) -> Result<(), String> {
    let body_type_str = quote! { #body_type }.to_string().replace(" ", "");
    
    // Look for a parameter that could accept the body
    let mut found_body_param = false;
    for input in &sig.inputs {
        if let FnArg::Typed(pat_type) = input {
            let param_type_str = quote! { #pat_type.ty }.to_string().replace(" ", "");
            
            // Check if this parameter could accept the body type
            if param_type_str.contains("ElifJson") ||
               param_type_str.contains("Json") ||
               param_type_str.contains(&body_type_str) {
                found_body_param = true;
                
                // Validate that it's properly wrapped
                if param_type_str.contains("ElifJson") {
                    // Check if the inner type matches
                    if !param_type_str.contains(&body_type_str) {
                        return Err(format!(
                            "ElifJson parameter type doesn't match expected body type '{}'",
                            body_type_str
                        ));
                    }
                }
                break;
            }
        }
    }
    
    if !found_body_param {
        return Err(format!(
            "No compatible body parameter found for type '{}'. Expected parameter like ElifJson<{}>",
            body_type_str, body_type_str
        ));
    }
    
    Ok(())
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