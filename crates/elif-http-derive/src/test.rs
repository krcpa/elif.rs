//! Tests for the derive macros

#[cfg(test)]
mod tests {    
    #[test]
    fn test_basic_functionality() {
        // Basic test to ensure the crate compiles and has the expected structure
        // The actual macro functionality would be tested in integration tests
        // or in dependent crates
        
        // Just verify that we can compile this test
        assert_eq!(2 + 2, 4);
    }
    
    #[test]
    fn test_meta_parsing() {
        // Test that we can parse basic syn structures
        use syn::{parse_quote, Meta};
        
        let meta: Meta = parse_quote!(test_path);
        match meta {
            Meta::Path(path) => {
                assert!(path.is_ident("test_path"));
            }
            _ => panic!("Expected Meta::Path"),
        }
    }
    
    #[test]
    fn test_request_attribute_detection() {
        use syn::{parse_quote, Attribute};
        use crate::utils::has_request_attribute;
        
        // Test that we can detect #[request] attribute
        let attrs: Vec<Attribute> = vec![
            parse_quote!(#[get("/test")]),
            parse_quote!(#[request]),
            parse_quote!(#[param(id: int)]),
        ];
        
        assert!(has_request_attribute(&attrs));
        
        // Test that we don't false-positive on other attributes
        let attrs_without_request: Vec<Attribute> = vec![
            parse_quote!(#[get("/test")]),
            parse_quote!(#[param(id: int)]),
        ];
        
        assert!(!has_request_attribute(&attrs_without_request));
    }
    
    #[test]
    fn test_request_param_name_extraction() {
        use syn::{parse_quote, Attribute};
        use crate::utils::extract_request_param_name;
        
        // Test default request parameter name
        let attrs: Vec<Attribute> = vec![
            parse_quote!(#[request]),
        ];
        
        assert_eq!(extract_request_param_name(&attrs), "req");
        
        // Test custom request parameter name
        let attrs_with_custom_name: Vec<Attribute> = vec![
            parse_quote!(#[request(custom_req)]),
        ];
        
        assert_eq!(extract_request_param_name(&attrs_with_custom_name), "custom_req");
    }
    
    #[test]
    fn test_validation_logic_unit() {
        use syn::{parse_quote, FnArg, Pat, PatIdent};
        
        // Test the validation logic components without calling the full procedural macro
        
        // Test case 1: ElifRequest parameter should not conflict
        let input_fn: syn::ItemFn = parse_quote! {
            async fn handler(&self, req: ElifRequest) -> String {
                "test".to_string()
            }
        };
        
        let req_param_name = "req";
        let mut has_conflict = false;
        
        // Simulate the validation logic from request_impl
        for input in &input_fn.sig.inputs {
            if let FnArg::Typed(pat_type) = input {
                if let Pat::Ident(PatIdent { ident, .. }) = pat_type.pat.as_ref() {
                    if *ident == req_param_name {
                        let param_type_str = quote::quote! { #pat_type.ty }.to_string();
                        if !param_type_str.contains("ElifRequest") {
                            has_conflict = true;
                        }
                        break;
                    }
                }
            }
        }
        
        assert!(!has_conflict, "ElifRequest parameter should not conflict");
        
        // Test case 2: String parameter with same name should conflict
        let input_fn_conflict: syn::ItemFn = parse_quote! {
            async fn handler(&self, req: String) -> String {
                "test".to_string()
            }
        };
        
        let mut has_conflict_2 = false;
        
        for input in &input_fn_conflict.sig.inputs {
            if let FnArg::Typed(pat_type) = input {
                if let Pat::Ident(PatIdent { ident, .. }) = pat_type.pat.as_ref() {
                    if *ident == req_param_name {
                        let param_type_str = quote::quote! { #pat_type.ty }.to_string();
                        if !param_type_str.contains("ElifRequest") {
                            has_conflict_2 = true;
                        }
                        break;
                    }
                }
            }
        }
        
        assert!(has_conflict_2, "String parameter with req name should conflict");
    }
}