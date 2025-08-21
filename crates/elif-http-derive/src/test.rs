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
}