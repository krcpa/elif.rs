//! HTTP method macros implementation
//! 
//! Provides #[get], #[post], #[put], #[delete], #[patch], #[head], #[options] macros.

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn};

use crate::utils::{validate_route_path, extract_path_parameters, extract_function_parameters, extract_param_types_from_attrs, has_request_attribute, extract_request_param_name, has_body_attribute, extract_body_param_from_attrs};
use crate::params::BodyParamType;
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
    // Apply injection if:
    // 1. Traditional: There are path parameters + #[param] annotations
    // 2. New: #[request] attribute is present (automatic ElifRequest injection)
    // 3. New: #[body] attribute is present (automatic body parameter injection)
    let has_self = input_fn.sig.inputs.iter().any(|arg| matches!(arg, syn::FnArg::Receiver(_)));
    let has_param_annotations = !param_types.is_empty();
    let has_request_attr = has_request_attribute(&input_fn.attrs);
    let has_body_attr = has_body_attribute(&input_fn.attrs);
    let has_path_param_injection = !path_params.is_empty() 
        && has_self
        && has_param_annotations
        && has_injectable_params(&fn_params, &path_params);
    let needs_injection = has_path_param_injection || (has_self && has_request_attr) || (has_self && has_body_attr);
    
    if needs_injection {
        // Extract body parameter information
        let body_param = extract_body_param_from_attrs(&input_fn.attrs);
        
        // Generate wrapper method with parameter injection
        generate_injected_method(&input_fn, &path_params, &param_types, has_request_attr, body_param)
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

/// Generate a wrapper method that injects path parameters, body parameters, and/or request into the original method
fn generate_injected_method(
    input_fn: &ItemFn, 
    path_params: &[String], 
    param_types: &HashMap<String, String>,
    has_request_attr: bool,
    body_param: Option<(String, BodyParamType)>
) -> TokenStream {
    let original_name = &input_fn.sig.ident;
    let original_fn_name = quote::format_ident!("{}_original", original_name);
    
    // Build parameter extraction and call arguments in order
    let mut param_extractions = Vec::new();
    let mut call_args = Vec::new();
    let mut modified_inputs = Vec::new();
    let request_param_name = if has_request_attr {
        extract_request_param_name(&input_fn.attrs)
    } else {
        "req".to_string()
    };
    
    // First, copy existing parameters and check for request parameter
    let mut has_existing_request_param = false;
    for input in &input_fn.sig.inputs {
        match input {
            syn::FnArg::Receiver(_) => {
                // Keep self parameter
                modified_inputs.push(input.clone());
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
                        modified_inputs.push(input.clone());
                    } else if param_type_str.contains("ElifRequest") {
                        // This is the request parameter - pass it through
                        call_args.push(quote! { request });
                        modified_inputs.push(input.clone());
                        has_existing_request_param = true;
                    } else if let Some((body_param_name, _)) = &body_param {
                        if param_name == *body_param_name {
                            // This is a body parameter - will be handled below
                            let param_ident = &pat_ident.ident;
                            call_args.push(quote! { #param_ident });
                            modified_inputs.push(input.clone());
                        } else {
                            // Unsupported parameter type - generate compile-time error
                            return syn::Error::new_spanned(
                                pat_type,
                                format!(
                                    "Unsupported parameter '{}' of type '{}'. Only path parameters (specified in route), body parameters (annotated with #[body]), and ElifRequest are supported. \
                                    Hint: Remove this parameter, add it to the route path like '/users/{{{}}}' and annotate with #[param({}: type)], annotate with #[body({}: Type)], or use #[request] to enable automatic request injection.",
                                    param_name, param_type_str, param_name, param_name, param_name
                                )
                            )
                            .to_compile_error()
                            .into();
                        }
                    } else {
                        // Unsupported parameter type - generate compile-time error
                        // Any parameter that is not a path parameter or ElifRequest is not supported
                        return syn::Error::new_spanned(
                            pat_type,
                            format!(
                                "Unsupported parameter '{}' of type '{}'. Only path parameters (specified in route), body parameters (annotated with #[body]), and ElifRequest are supported. \
                                Hint: Remove this parameter, add it to the route path like '/users/{{{}}}' and annotate with #[param({}: type)], annotate with #[body({}: Type)], or use #[request] to enable automatic request injection.",
                                param_name, param_type_str, param_name, param_name, param_name
                            )
                        )
                        .to_compile_error()
                        .into();
                    }
                }
            }
        }
    }
    
    // If #[request] is present and method doesn't have ElifRequest parameter, add it to signature
    if has_request_attr && !has_existing_request_param {
        let req_ident = quote::format_ident!("{}", request_param_name);
        let request_param = syn::parse_quote! {
            #req_ident: ::elif_http::ElifRequest
        };
        modified_inputs.push(request_param);
        call_args.push(quote! { request });
    }
    
    // Add body parameter extraction if present
    if let Some((body_param_name, body_param_type)) = &body_param {
        let body_param_ident = quote::format_ident!("{}", body_param_name);
        
        // Generate body parsing code based on the body parameter type
        let body_extraction = match body_param_type {
            BodyParamType::Custom(_) => {
                quote! {
                    let #body_param_ident = request.json().await
                        .map_err(|e| HttpError::bad_request(format!("Invalid JSON body: {:?}", e)))?;
                }
            }
            BodyParamType::Form => {
                quote! {
                    let #body_param_ident = request.form().await
                        .map_err(|e| HttpError::bad_request(format!("Invalid form data: {:?}", e)))?;
                }
            }
            BodyParamType::Bytes => {
                quote! {
                    let #body_param_ident = request.bytes().await
                        .map_err(|e| HttpError::bad_request(format!("Failed to read body bytes: {:?}", e)))?;
                }
            }
        };
        
        param_extractions.push(body_extraction);
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
                       !attr.path().is_ident("param") &&
                       !attr.path().is_ident("request") &&
                       !attr.path().is_ident("body"))
        .collect::<Vec<_>>();
    let original_vis = &input_fn.vis;
    let original_block = &input_fn.block;
    let original_return = &input_fn.sig.output;
    let original_asyncness = &input_fn.sig.asyncness;
    
    // Generate the wrapper method's asyncness (always async for HTTP handlers)
    let wrapper_asyncness = quote! { async };
    
    // Generate the appropriate method call based on original async-ness
    let method_call = if original_asyncness.is_some() {
        quote! { self.#original_fn_name(#(#call_args),*).await }
    } else {
        quote! { self.#original_fn_name(#(#call_args),*) }
    };
    
    // Analyze the return type to determine how to handle the response
    let return_category = analyze_return_type(original_return);
    
    // Generate appropriate response handling based on return type
    let response_handling = match return_category {
        ReturnTypeCategory::HttpResultElifResponse => {
            // Already returns HttpResult<ElifResponse> - pass through directly
            quote! { #method_call }
        }
        ReturnTypeCategory::ElifResponse => {
            // Returns ElifResponse - wrap in Ok()
            quote! { Ok(#method_call) }
        }
        ReturnTypeCategory::ResultType => {
            // Returns Result<T, E> - map the Ok case to JSON, pass through errors
            quote! { 
                match #method_call {
                    Ok(result) => Ok(ElifResponse::ok().json(&result)?),
                    Err(e) => Err(HttpError::internal_server_error(format!("Handler error: {:?}", e)).into()),
                }
            }
        }
        ReturnTypeCategory::Unit => {
            // Returns () - return empty OK response
            quote! { 
                #method_call;
                Ok(ElifResponse::ok())
            }
        }
        ReturnTypeCategory::SerializableType => {
            // Returns serializable type - wrap in JSON response
            quote! { 
                let result = #method_call;
                Ok(ElifResponse::ok().json(&result)?)
            }
        }
    };
    
    let expanded = quote! {
        // Keep the original method with modified signature (includes injected request parameter)
        #(#original_attrs)*
        #original_vis #original_asyncness fn #original_fn_name(#(#modified_inputs),*) #original_return #original_block
        
        // Generate wrapper method that extracts parameters from request
        #original_vis #wrapper_asyncness fn #original_name(&self, request: ElifRequest) -> HttpResult<ElifResponse> {
            // Parameter extraction
            #(#param_extractions)*
            
            // Handle result based on original function's return type
            #response_handling
        }
    };
    
    TokenStream::from(expanded)
}

/// Determine how to handle the return value based on the original function's return type
fn analyze_return_type(return_type: &syn::ReturnType) -> ReturnTypeCategory {
    match return_type {
        syn::ReturnType::Default => ReturnTypeCategory::Unit,
        syn::ReturnType::Type(_, ty) => {
            let type_str = quote! { #ty }.to_string();
            
            // Check for HttpResult<ElifResponse>
            if type_str.contains("HttpResult") && type_str.contains("ElifResponse") {
                ReturnTypeCategory::HttpResultElifResponse
            }
            // Check for ElifResponse
            else if type_str.contains("ElifResponse") {
                ReturnTypeCategory::ElifResponse
            }
            // Check for Result types (including HttpResult<T> where T != ElifResponse)
            else if type_str.contains("Result") || type_str.contains("HttpResult") {
                ReturnTypeCategory::ResultType
            }
            // Everything else (serializable types)
            else {
                ReturnTypeCategory::SerializableType
            }
        }
    }
}

/// Categories of return types for response handling
#[derive(Debug, PartialEq)]
enum ReturnTypeCategory {
    Unit,                      // () - return empty response
    ElifResponse,              // ElifResponse - pass through
    HttpResultElifResponse,    // HttpResult<ElifResponse> - pass through  
    ResultType,                // Result<T, E> - handle error, serialize T
    SerializableType,          // T - serialize to JSON
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
            "Uuid" => quote::format_ident!("path_param_uuid"), // UUID should be parsed with its specific method
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