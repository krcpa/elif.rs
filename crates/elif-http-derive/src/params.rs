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
"Unsupported parameter type. Supported types: string, int, uint, float, bool, uuid".to_string()
                ));
            }
        };
        
        Ok(ParamSpec { name, param_type })
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

/// Validate that function signature is compatible with the specified body type
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
pub fn body_impl(args: TokenStream, input: TokenStream) -> TokenStream {
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
    
    // Check if the function already has a parameter with the request name
    let mut has_existing_req_param = false;
    for input in &input_fn.sig.inputs {
        if let FnArg::Typed(pat_type) = input {
            if let Pat::Ident(PatIdent { ident, .. }) = pat_type.pat.as_ref() {
                if *ident == req_param_name {
                    has_existing_req_param = true;
                    break;
                }
            }
        }
    }
    
    if has_existing_req_param {
        return syn::Error::new_spanned(
            &input_fn.sig,
            format!("Function already has a '{}' parameter. Remove #[request] or use a different parameter name.", req_param_name)
        )
        .to_compile_error()
        .into();
    }
    
    // The actual parameter injection will be handled by the HTTP method macros
    // This macro just validates and marks the function for request injection
    let expanded = quote! {
        #input_fn
    };
    
    TokenStream::from(expanded)
}