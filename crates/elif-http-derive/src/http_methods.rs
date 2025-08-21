//! HTTP method macros implementation
//! 
//! Provides #[get], #[post], #[put], #[delete], #[patch], #[head], #[options] macros.

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn};

use crate::utils::{validate_route_path, extract_path_parameters, extract_function_parameters, extract_param_types_from_attrs};
use std::collections::HashMap;

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
    let fn_params = extract_function_parameters(&input_fn.sig);
    let param_types = extract_param_types_from_attrs(&input_fn.attrs);
    
    // Check if this method needs parameter injection
    // Only apply injection if:
    // 1. There are path parameters in the route
    // 2. The function has matching parameters in its signature  
    // 3. The function has a &self parameter (is a method, not a standalone function)
    // 4. There are explicit #[param] annotations
    let has_self = input_fn.sig.inputs.iter().any(|arg| matches!(arg, syn::FnArg::Receiver(_)));
    let has_param_annotations = !param_types.is_empty();
    let needs_injection = !path_params.is_empty() 
        && has_self
        && has_param_annotations
        && has_injectable_params(&fn_params, &path_params);
    
    if needs_injection {
        // Generate wrapper method with parameter injection
        generate_injected_method(&input_fn, &path_params, &param_types)
    } else {
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

/// Check if the function signature has parameters that match route parameters
/// and can be injected (excluding 'req' parameter which is special)
fn has_injectable_params(fn_params: &[(String, String)], path_params: &[String]) -> bool {
    // Check if any function parameters match path parameters (excluding ElifRequest)
    for (param_name, param_type) in fn_params {
        if path_params.contains(param_name) && !param_type.contains("ElifRequest") {
            return true;
        }
    }
    false
}

/// Generate a wrapper method that injects path parameters into the original method
fn generate_injected_method(
    input_fn: &ItemFn, 
    path_params: &[String], 
    param_types: &HashMap<String, String>
) -> TokenStream {
    let original_name = &input_fn.sig.ident;
    let original_fn_name = quote::format_ident!("{}_original", original_name);
    
    // Build parameter extraction and call arguments in order
    let mut param_extractions = Vec::new();
    let mut call_args = Vec::new();
    
    // Process function parameters in order to preserve original signature order
    for input in &input_fn.sig.inputs {
        match input {
            syn::FnArg::Receiver(_) => {
                // Skip self parameter - it's implicit in method call
            }
            syn::FnArg::Typed(pat_type) => {
                if let syn::Pat::Ident(pat_ident) = pat_type.pat.as_ref() {
                    let param_name = pat_ident.ident.to_string();
                    let param_type = &pat_type.ty;
                    let param_type_str = quote! { #param_type }.to_string();
                    
                    if path_params.contains(&param_name) {
                        // This is a path parameter - generate extraction code
                        let param_ident = &pat_ident.ident;
                        let extraction_method = get_extraction_method(&param_name, param_types, &param_type_str);
                        
                        param_extractions.push(quote! {
                            let #param_ident = request.#extraction_method(#param_name)
                                .map_err(|e| HttpError::bad_request(format!("Invalid parameter '{}': {:?}", #param_name, e)))?;
                        });
                        call_args.push(quote! { #param_ident });
                    } else if param_type_str.contains("ElifRequest") {
                        // This is the request parameter - pass it through
                        call_args.push(quote! { request });
                    } else {
                        // Unsupported parameter type - generate compile-time error
                        return syn::Error::new_spanned(
                            pat_type,
                            format!(
                                "Unsupported parameter '{}' of type '{}'. Only path parameters (specified in route) and ElifRequest are supported. \
                                Hint: Remove this parameter or add it to the route path like '/users/{{{}}}' and annotate with #[param({}: type)]",
                                param_name, param_type_str, param_name, param_name
                            )
                        )
                        .to_compile_error()
                        .into();
                    }
                }
            }
        }
    }
    
    // Get the original function's components
    let original_attrs = &input_fn.attrs.iter()
        .filter(|attr| !attr.path().is_ident("get") && 
                       !attr.path().is_ident("post") &&
                       !attr.path().is_ident("put") &&
                       !attr.path().is_ident("delete") &&
                       !attr.path().is_ident("patch") &&
                       !attr.path().is_ident("head") &&
                       !attr.path().is_ident("options") &&
                       !attr.path().is_ident("param"))
        .collect::<Vec<_>>();
    let original_vis = &input_fn.vis;
    let original_block = &input_fn.block;
    let original_return = &input_fn.sig.output;
    let original_asyncness = &input_fn.sig.asyncness;
    let original_inputs = &input_fn.sig.inputs;
    
    // Generate the wrapper method's asyncness (always async for HTTP handlers)
    let wrapper_asyncness = quote! { async };
    
    // Generate the appropriate method call based on original async-ness
    let method_call = if original_asyncness.is_some() {
        quote! { self.#original_fn_name(#(#call_args),*).await }
    } else {
        quote! { self.#original_fn_name(#(#call_args),*) }
    };
    
    let expanded = quote! {
        // Keep the original method with renamed identifier
        #(#original_attrs)*
        #original_vis #original_asyncness fn #original_fn_name(#original_inputs) #original_return #original_block
        
        // Generate wrapper method that extracts parameters from request
        #original_vis #wrapper_asyncness fn #original_name(&self, request: ElifRequest) -> HttpResult<ElifResponse> {
            // Parameter extraction
            #(#param_extractions)*
            
            // Call original method with extracted parameters
            let result = #method_call;
            
            // Convert result to HTTP response
            Ok(ElifResponse::ok().json(&result)?)
        }
    };
    
    TokenStream::from(expanded)
}

/// Get the appropriate extraction method name based on parameter type
fn get_extraction_method(
    param_name: &str, 
    param_types: &HashMap<String, String>, 
    rust_type: &str
) -> proc_macro2::Ident {
    // Check if we have explicit type information from #[param] attribute
    if let Some(param_type) = param_types.get(param_name) {
        return match param_type.as_str() {
            "Integer" => quote::format_ident!("path_param_int"),
            "String" => quote::format_ident!("path_param_string"),
            "Uuid" => quote::format_ident!("path_param_string"), // UUID is handled as string
            _ => quote::format_ident!("path_param_string"), // Default to string
        };
    }
    
    // Fall back to inferring from Rust type
    if rust_type.contains("i32") || rust_type.contains("u32") || rust_type.contains("i64") || rust_type.contains("u64") {
        quote::format_ident!("path_param_int")
    } else {
        quote::format_ident!("path_param_string")
    }
}