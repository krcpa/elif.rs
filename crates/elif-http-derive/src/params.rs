//! Parameter and body validation macros
//! 
//! Provides #[param] and #[body] macros for type-safe parameter handling.

use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse_macro_input, ItemFn, Signature, FnArg, Pat, PatIdent, Type, Ident,
    parse::Parse, parse::ParseStream, Token
};

/// Parameter specification for type validation
#[derive(Debug, Clone)]
pub struct ParamSpec {
    pub name: Ident,
    pub param_type: ParamType,
}

/// Body parameter specification for injection
#[derive(Debug, Clone)]
pub struct BodySpec {
    pub name: Ident,
    pub body_type: BodyParamType,
}

/// Supported parameter types for validation
#[derive(Debug, Clone, PartialEq)]
pub enum ParamType {
    String,
    Int,
    UInt,
    Float,
    Bool,
    Uuid,
}

/// Supported body parameter types for injection
#[derive(Debug, Clone, PartialEq)]
pub enum BodyParamType {
    Custom(Type),  // Custom deserializable type
    Form,          // HashMap<String, String> from form data
    Bytes,         // Vec<u8> for raw bytes
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
        let type_name = type_ident.to_string();
        
        let param_type = match type_name.as_str() {
            "string" => ParamType::String,
            "int" => ParamType::Int,
            "uint" => ParamType::UInt,
            "float" => ParamType::Float,
            "bool" => ParamType::Bool,
            "uuid" => ParamType::Uuid,
            _ => {
                return Err(syn::Error::new_spanned(
                    type_ident,
                    format!(
                        "Unsupported parameter type '{}'. Supported types: string, int, uint, float, bool, uuid. \
                        Hint: Use #[param({}: string)] for string parameters or #[param({}: int)] for integer parameters.", 
                        type_name, 
                        name, 
                        name
                    )
                ));
            }
        };
        
        Ok(ParamSpec { name, param_type })
    }
}

impl Parse for BodySpec {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name: Ident = input.parse()?;
        input.parse::<Token![:]>()?;
        
        // Check for special types first (form, bytes)
        if input.peek(syn::Ident) {
            let lookahead = input.fork();
            if let Ok(type_ident) = lookahead.parse::<Ident>() {
                match type_ident.to_string().as_str() {
                    "form" => {
                        input.parse::<Ident>()?; // consume the 'form' ident
                        return Ok(BodySpec { name, body_type: BodyParamType::Form });
                    }
                    "bytes" => {
                        input.parse::<Ident>()?; // consume the 'bytes' ident
                        return Ok(BodySpec { name, body_type: BodyParamType::Bytes });
                    }
                    _ => {}
                }
            }
        }
        
        // Parse as custom type
        let custom_type: Type = input.parse()?;
        Ok(BodySpec { 
            name, 
            body_type: BodyParamType::Custom(custom_type) 
        })
    }
}

/// Validate that parameter specification matches function signature
pub fn validate_param_consistency(param_spec: &ParamSpec, sig: &Signature) -> Result<(), String> {
    let param_name = param_spec.name.to_string();
    
    // Find the parameter in the function signature
    let mut found_param = None;
    for input in &sig.inputs {
        if let FnArg::Typed(pat_type) = input {
            if let Pat::Ident(PatIdent { ident, .. }) = pat_type.pat.as_ref() {
                if *ident == param_name {
                    found_param = Some(&*pat_type.ty);
                    break;
                }
            }
        }
    }
    
    let param_type = match found_param {
        Some(ty) => ty,
        None => return Err(format!(
            "Parameter '{}' specified in #[param] not found in function signature. \
            Hint: Add '{}: SomeType' as a function parameter or remove the #[param({})] annotation.",
            param_name, param_name, param_name
        )),
    };
    
    // Validate type compatibility
    let type_str = quote! { #param_type }.to_string().replace(" ", "");
    let expected_types = get_compatible_rust_types(&param_spec.param_type);
    
    if !expected_types.iter().any(|expected| type_str.contains(expected)) {
        return Err(format!(
            "Parameter '{}' has type '{}' but expected one of: {}. \
            Hint: Change the function parameter to match #[param({}: {})] or change the #[param] declaration to match the function parameter.",
            param_name,
            type_str,
            expected_types.join(", "),
            param_name,
            get_recommended_param_type(&param_spec.param_type)
        ));
    }
    
    Ok(())
}

/// Get compatible Rust types for a parameter type
pub fn get_compatible_rust_types(param_type: &ParamType) -> Vec<&'static str> {
    match param_type {
        ParamType::String => vec!["String", "&str", "str"],
        ParamType::Int => vec!["i32", "i64", "isize"],
        ParamType::UInt => vec!["u32", "u64", "usize"],
        ParamType::Float => vec!["f32", "f64"],
        ParamType::Bool => vec!["bool"],
        ParamType::Uuid => vec!["Uuid", "uuid::Uuid"],
    }
}

/// Get the recommended parameter type string for error messages
pub fn get_recommended_param_type(param_type: &ParamType) -> &'static str {
    match param_type {
        ParamType::String => "string",
        ParamType::Int => "int", 
        ParamType::UInt => "uint",
        ParamType::Float => "float",
        ParamType::Bool => "bool",
        ParamType::Uuid => "uuid",
    }
}

/// Validate that function signature is compatible with the specified body type
/// NOTE: This function is deprecated - use validate_body_param_consistency for new parameter injection system
#[allow(dead_code)]
pub fn validate_body_compatibility(body_type: &Type, sig: &Signature) -> Result<(), String> {
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

/// Validate that body parameter specification matches function signature
pub fn validate_body_param_consistency(body_spec: &BodySpec, sig: &Signature) -> Result<(), String> {
    let param_name = body_spec.name.to_string();
    
    // Find the parameter in the function signature
    let mut found_param = None;
    for input in &sig.inputs {
        if let FnArg::Typed(pat_type) = input {
            if let Pat::Ident(PatIdent { ident, .. }) = pat_type.pat.as_ref() {
                if *ident == param_name {
                    found_param = Some(&*pat_type.ty);
                    break;
                }
            }
        }
    }
    
    let param_type = match found_param {
        Some(ty) => ty,
        None => return Err(format!("Body parameter '{}' not found in function signature", param_name)),
    };
    
    // Validate type compatibility based on body type
    let type_str = quote! { #param_type }.to_string().replace(" ", "");
    
    match &body_spec.body_type {
        BodyParamType::Custom(expected_type) => {
            let expected_type_str = quote! { #expected_type }.to_string().replace(" ", "");
            // For custom types, the function parameter should match exactly
            if type_str != expected_type_str {
                return Err(format!(
                    "Body parameter '{}' has type '{}' but expected '{}'",
                    param_name, type_str, expected_type_str
                ));
            }
        }
        BodyParamType::Form => {
            // Form data should be HashMap<String, String>
            if !type_str.contains("HashMap") || !type_str.contains("String") {
                return Err(format!(
                    "Body parameter '{}' has type '{}' but expected 'HashMap<String, String>' for form data",
                    param_name, type_str
                ));
            }
        }
        BodyParamType::Bytes => {
            // Bytes should be Vec<u8>
            if !type_str.contains("Vec") || !type_str.contains("u8") {
                return Err(format!(
                    "Body parameter '{}' has type '{}' but expected 'Vec<u8>' for bytes data",
                    param_name, type_str
                ));
            }
        }
    }
    
    Ok(())
}

/// Route parameter specification macro
/// Validates that the parameter type matches what's expected from the route path
pub fn param_impl(args: TokenStream, input: TokenStream) -> TokenStream {
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
        if let Err(msg) = validate_param_consistency(spec, &input_fn.sig) {
            return syn::Error::new_spanned(
                &input_fn.sig,
                format!("Parameter type mismatch: {}", msg)
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
pub fn body_impl(args: TokenStream, input: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(input as ItemFn);
    
    // Parse body parameter specification from args
    let body_spec = if !args.is_empty() {
        match syn::parse::<BodySpec>(args) {
            Ok(spec) => Some(spec),
            Err(err) => {
                return syn::Error::new(
                    err.span(),
                    format!("Invalid body specification: {}. Hint: Use #[body(param_name: Type)] for parameter injection or #[body(user_data: CreateUserRequest)]", err)
                )
                .to_compile_error()
                .into();
            }
        }
    } else {
        return syn::Error::new(
            proc_macro2::Span::call_site(),
            "Body specification required. Hint: Use #[body(param_name: Type)] to specify the body parameter."
        )
        .to_compile_error()
        .into();
    };
    
    // Validate that function signature matches body specification
    if let Some(spec) = &body_spec {
        if let Err(msg) = validate_body_param_consistency(spec, &input_fn.sig) {
            return syn::Error::new_spanned(
                &input_fn.sig,
                format!("Body parameter mismatch: {}. Hint: Ensure function parameter name and type match #[body] declaration.", msg)
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

/// Request injection macro for automatic ElifRequest parameter injection
/// Marks methods that should receive an ElifRequest parameter automatically
pub fn request_impl(args: TokenStream, input: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(input as ItemFn);
    
    // Parse optional parameter name from args (for future extensibility)
    let req_param_name = if !args.is_empty() {
        match syn::parse::<Ident>(args) {
            Ok(name) => name.to_string(),
            Err(_) => {
                return syn::Error::new(
                    proc_macro2::Span::call_site(),
                    "Invalid request parameter name. Hint: Use #[request] or #[request(req)] for custom naming."
                )
                .to_compile_error()
                .into();
            }
        }
    } else {
        "req".to_string()
    };
    
    // Check if the function already has a parameter that conflicts with the request name
    for input in &input_fn.sig.inputs {
        if let FnArg::Typed(pat_type) = input {
            if let Pat::Ident(PatIdent { ident, .. }) = pat_type.pat.as_ref() {
                if *ident == req_param_name {
                    // A parameter with the same name exists. Check if it's a conflict.
                    let param_type_str = quote! { #pat_type.ty }.to_string();
                    if !param_type_str.contains("ElifRequest") {
                        // It's a conflict if the type is not ElifRequest.
                        return syn::Error::new_spanned(
                            &input_fn.sig,
                            format!("Function already has a parameter named '{}' which conflicts with #[request]. Either rename the parameter or change its type to ElifRequest.", req_param_name)
                        )
                        .to_compile_error()
                        .into();
                    }
                    // If it is an ElifRequest, it's not a conflict. We can stop checking.
                    break;
                }
            }
        }
    }
    
    // The actual signature modification will be handled by the HTTP method macros
    // This macro validates the function and marks it for request parameter injection
    let expanded = quote! {
        #input_fn
    };
    
    TokenStream::from(expanded)
}