//! Utility functions shared across macro implementations

use crate::params::{BodyParamType, BodySpec};
use quote::quote;
use std::collections::HashMap;
use syn::{Attribute, FnArg, Meta, Pat, PatIdent, Signature};

/// Extract HTTP method and path from method attributes
pub fn extract_http_method_info(attrs: &[Attribute]) -> Option<(proc_macro2::Ident, String)> {
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
pub fn extract_resource_info(attrs: &[Attribute]) -> Option<String> {
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
pub fn extract_path_from_meta_list_robust(tokens: &proc_macro2::TokenStream) -> String {
    // If tokens are empty, return empty string
    if tokens.is_empty() {
        return String::new();
    }

    // Try to parse as a string literal directly
    if let Ok(lit_str) = syn::parse2::<syn::LitStr>(tokens.clone()) {
        return lit_str.value();
    }

    // Try to parse as a parenthesized string literal: ("path")
    if let Ok(group) = syn::parse2::<proc_macro2::Group>(tokens.clone()) {
        if group.delimiter() == proc_macro2::Delimiter::Parenthesis {
            if let Ok(inner_lit) = syn::parse2::<syn::LitStr>(group.stream()) {
                return inner_lit.value();
            }
        }
    }

    // Try to manually extract from parentheses format
    let tokens_iter = tokens.clone().into_iter().collect::<Vec<_>>();
    if tokens_iter.len() == 1 {
        if let proc_macro2::TokenTree::Group(group) = &tokens_iter[0] {
            if group.delimiter() == proc_macro2::Delimiter::Parenthesis {
                if let Ok(inner_lit) = syn::parse2::<syn::LitStr>(group.stream()) {
                    return inner_lit.value();
                }
            }
        }
    }

    // Fall back to empty string if we can't parse properly
    String::new()
}

/// Validate route path format and return error message if invalid
pub fn validate_route_path(path: &str) -> Result<(), String> {
    if path.is_empty() {
        return Ok(());
    }

    // Check for malformed parameter syntax
    let chars = path.chars();
    let mut brace_count = 0;
    let mut in_param = false;
    let mut param_content = String::new();

    for ch in chars {
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
                if !param_content
                    .chars()
                    .all(|c| c.is_alphanumeric() || c == '_')
                {
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

/// Extract path parameters from a route path (e.g., "/users/{id}" -> ["id"])
pub fn extract_path_parameters(path: &str) -> Vec<String> {
    let mut params = Vec::new();
    let mut chars = path.chars();

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
pub fn extract_function_parameters(sig: &Signature) -> Vec<(String, String)> {
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
pub fn extract_middleware_from_attrs(attrs: &[Attribute]) -> Vec<String> {
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

/// Convert param type from derive crate to routing crate format
pub fn convert_param_type_to_routing(param_type: &str) -> String {
    match param_type {
        "string" => "String".to_string(),
        "String" => "String".to_string(),
        "int" => "Integer".to_string(),
        "i32" => "Integer".to_string(),
        "i64" => "Integer".to_string(),
        "uint" => "Integer".to_string(),
        "u32" => "Integer".to_string(),
        "u64" => "Integer".to_string(),
        "uuid" => "Uuid".to_string(),
        "Uuid" => "Uuid".to_string(),
        "float" => "String".to_string(), // Float not in routing::ParamType yet
        "f32" => "String".to_string(),
        "f64" => "String".to_string(),
        "bool" => "String".to_string(),  // Bool not in routing::ParamType yet
        _ => "String".to_string(),       // Default fallback
    }
}

/// Extract parameter type specifications from method attributes
/// Returns a map of parameter name -> parameter type
pub fn extract_param_types_from_attrs(attrs: &[Attribute]) -> HashMap<String, String> {
    let mut param_types = HashMap::new();

    for attr in attrs {
        if attr.path().is_ident("param") {
            if let Meta::List(meta_list) = &attr.meta {
                // Parse parameter specifications from the attribute tokens
                let tokens = &meta_list.tokens;

                // Try to parse multiple comma-separated param specs: id: int, name: string
                if let Ok(parsed_specs) = parse_param_specs(tokens.clone()) {
                    for (name, type_str) in parsed_specs {
                        let routing_type = convert_param_type_to_routing(&type_str);
                        param_types.insert(name, routing_type);
                    }
                }
            }
        }
    }

    param_types
}

/// Parse parameter specifications from token stream
/// Expected format: "id: int, name: string" or just "id: int"
fn parse_param_specs(
    tokens: proc_macro2::TokenStream,
) -> Result<Vec<(String, String)>, syn::Error> {
    use syn::parse::{Parse, ParseStream};
    use syn::{Ident, Token};

    struct ParamSpecs {
        specs: Vec<(String, String)>,
    }

    impl Parse for ParamSpecs {
        fn parse(input: ParseStream) -> syn::Result<Self> {
            let mut specs = Vec::new();

            // Parse first param spec
            if !input.is_empty() {
                let name: Ident = input.parse()?;
                input.parse::<Token![:]>()?;
                let type_ident: Ident = input.parse()?;

                specs.push((name.to_string(), type_ident.to_string()));

                // Parse additional comma-separated specs
                while input.parse::<Token![,]>().is_ok() {
                    let name: Ident = input.parse()?;
                    input.parse::<Token![:]>()?;
                    let type_ident: Ident = input.parse()?;

                    specs.push((name.to_string(), type_ident.to_string()));
                }
            }

            Ok(ParamSpecs { specs })
        }
    }

    let parsed: ParamSpecs = syn::parse2(tokens)?;
    Ok(parsed.specs)
}

/// Check if method has #[request] attribute for automatic ElifRequest injection
pub fn has_request_attribute(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| attr.path().is_ident("request"))
}

/// Extract request parameter name from #[request] attribute
/// Returns "req" by default or custom name if specified
pub fn extract_request_param_name(attrs: &[Attribute]) -> String {
    for attr in attrs {
        if attr.path().is_ident("request") {
            if let Meta::List(meta_list) = &attr.meta {
                // Try to parse custom parameter name from tokens
                if let Ok(ident) = syn::parse2::<syn::Ident>(meta_list.tokens.clone()) {
                    return ident.to_string();
                }
            }
            // Default name when no custom name specified
            return "req".to_string();
        }
    }
    "req".to_string()
}

/// Check if method has #[body] attribute for body parameter injection
pub fn has_body_attribute(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| attr.path().is_ident("body"))
}

/// Extract body parameter specification from #[body] attribute
/// Returns (param_name, body_param_type) if found
pub fn extract_body_param_from_attrs(attrs: &[Attribute]) -> Option<(String, BodyParamType)> {
    for attr in attrs {
        if attr.path().is_ident("body") {
            if let Meta::List(meta_list) = &attr.meta {
                // Try to parse body specification from tokens
                if let Ok(body_spec) = syn::parse2::<BodySpec>(meta_list.tokens.clone()) {
                    let param_name = body_spec.name.to_string();
                    return Some((param_name, body_spec.body_type));
                }
            }
        }
    }
    None
}
