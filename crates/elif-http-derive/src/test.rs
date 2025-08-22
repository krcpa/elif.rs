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
    
    #[test]
    fn test_body_spec_parsing() {
        use syn::parse_quote;
        use crate::params::{BodySpec, BodyParamType};
        
        // Test custom type parsing
        let custom_body: BodySpec = parse_quote!(user_data: CreateUserRequest);
        assert_eq!(custom_body.name.to_string(), "user_data");
        match custom_body.body_type {
            BodyParamType::Custom(_) => {}, // Success
            _ => panic!("Expected Custom body type"),
        }
        
        // Test form data parsing
        let form_body: BodySpec = parse_quote!(form_data: form);
        assert_eq!(form_body.name.to_string(), "form_data");
        assert_eq!(form_body.body_type, BodyParamType::Form);
        
        // Test bytes parsing
        let bytes_body: BodySpec = parse_quote!(file_data: bytes);
        assert_eq!(bytes_body.name.to_string(), "file_data");
        assert_eq!(bytes_body.body_type, BodyParamType::Bytes);
    }
    
    #[test]
    fn test_body_attribute_detection() {
        use syn::{parse_quote, Attribute};
        use crate::utils::has_body_attribute;
        
        // Test that we can detect #[body] attribute
        let attrs: Vec<Attribute> = vec![
            parse_quote!(#[post("/users")]),
            parse_quote!(#[body(user_data: CreateUserRequest)]),
            parse_quote!(#[param(id: int)]),
        ];
        
        assert!(has_body_attribute(&attrs));
        
        // Test that we don't false-positive on other attributes
        let attrs_without_body: Vec<Attribute> = vec![
            parse_quote!(#[post("/users")]),
            parse_quote!(#[param(id: int)]),
        ];
        
        assert!(!has_body_attribute(&attrs_without_body));
    }
    
    #[test]
    fn test_body_param_extraction() {
        use syn::{parse_quote, Attribute};
        use crate::utils::extract_body_param_from_attrs;
        use crate::params::BodyParamType;
        
        // Test custom type extraction
        let attrs: Vec<Attribute> = vec![
            parse_quote!(#[post("/users")]),
            parse_quote!(#[body(user_data: CreateUserRequest)]),
        ];
        
        let result = extract_body_param_from_attrs(&attrs);
        assert!(result.is_some());
        let (param_name, body_type) = result.unwrap();
        assert_eq!(param_name, "user_data");
        match body_type {
            BodyParamType::Custom(_) => {}, // Success
            _ => panic!("Expected Custom body type"),
        }
        
        // Test form data extraction
        let form_attrs: Vec<Attribute> = vec![
            parse_quote!(#[post("/contact")]),
            parse_quote!(#[body(form_data: form)]),
        ];
        
        let form_result = extract_body_param_from_attrs(&form_attrs);
        assert!(form_result.is_some());
        let (form_param_name, form_body_type) = form_result.unwrap();
        assert_eq!(form_param_name, "form_data");
        assert_eq!(form_body_type, BodyParamType::Form);
        
        // Test bytes extraction
        let bytes_attrs: Vec<Attribute> = vec![
            parse_quote!(#[post("/upload")]),
            parse_quote!(#[body(file_data: bytes)]),
        ];
        
        let bytes_result = extract_body_param_from_attrs(&bytes_attrs);
        assert!(bytes_result.is_some());
        let (bytes_param_name, bytes_body_type) = bytes_result.unwrap();
        assert_eq!(bytes_param_name, "file_data");
        assert_eq!(bytes_body_type, BodyParamType::Bytes);
    }
    
    #[test]
    fn test_body_param_validation() {
        use syn::parse_quote;
        use crate::params::{BodySpec, BodyParamType, validate_body_param_consistency};
        
        // Test valid custom type validation
        let valid_fn: syn::ItemFn = parse_quote! {
            async fn create_user(&self, user_data: CreateUserRequest) -> HttpResult<ElifResponse> {
                Ok(ElifResponse::created())
            }
        };
        
        let valid_spec = BodySpec {
            name: parse_quote!(user_data),
            body_type: BodyParamType::Custom(Box::new(parse_quote!(CreateUserRequest))),
        };
        
        let validation_result = validate_body_param_consistency(&valid_spec, &valid_fn.sig);
        assert!(validation_result.is_ok(), "Valid body parameter should pass validation");
        
        // Test invalid parameter name
        let invalid_name_spec = BodySpec {
            name: parse_quote!(wrong_name),
            body_type: BodyParamType::Custom(Box::new(parse_quote!(CreateUserRequest))),
        };
        
        let name_validation_result = validate_body_param_consistency(&invalid_name_spec, &valid_fn.sig);
        assert!(name_validation_result.is_err(), "Invalid parameter name should fail validation");
        assert!(name_validation_result.unwrap_err().contains("not found in function signature"));
        
        // Test valid form data validation
        let form_fn: syn::ItemFn = parse_quote! {
            async fn contact(&self, form_data: HashMap<String, String>) -> HttpResult<ElifResponse> {
                Ok(ElifResponse::ok())
            }
        };
        
        let form_spec = BodySpec {
            name: parse_quote!(form_data),
            body_type: BodyParamType::Form,
        };
        
        let form_validation_result = validate_body_param_consistency(&form_spec, &form_fn.sig);
        assert!(form_validation_result.is_ok(), "Valid form parameter should pass validation");
        
        // Test valid bytes validation
        let bytes_fn: syn::ItemFn = parse_quote! {
            async fn upload(&self, file_data: Vec<u8>) -> HttpResult<ElifResponse> {
                Ok(ElifResponse::ok())
            }
        };
        
        let bytes_spec = BodySpec {
            name: parse_quote!(file_data),
            body_type: BodyParamType::Bytes,
        };
        
        let bytes_validation_result = validate_body_param_consistency(&bytes_spec, &bytes_fn.sig);
        assert!(bytes_validation_result.is_ok(), "Valid bytes parameter should pass validation");
        
        // Test invalid type mismatch
        let invalid_type_spec = BodySpec {
            name: parse_quote!(user_data),
            body_type: BodyParamType::Form, // Wrong type for the parameter
        };
        
        let type_validation_result = validate_body_param_consistency(&invalid_type_spec, &valid_fn.sig);
        assert!(type_validation_result.is_err(), "Type mismatch should fail validation");
        assert!(type_validation_result.unwrap_err().contains("expected 'HashMap<String, String>'"));
    }
    
    #[test]
    fn test_combined_attributes_detection() {
        use syn::{parse_quote, Attribute};
        use crate::utils::{has_request_attribute, has_body_attribute, extract_body_param_from_attrs};
        use crate::params::BodyParamType;
        
        // Test that both #[request] and #[body] attributes can coexist
        let combined_attrs: Vec<Attribute> = vec![
            parse_quote!(#[put("/users/{id}")]),
            parse_quote!(#[param(id: int)]),
            parse_quote!(#[body(user_data: UpdateUserRequest)]),
            parse_quote!(#[request]),
        ];
        
        assert!(has_request_attribute(&combined_attrs), "Should detect #[request] attribute");
        assert!(has_body_attribute(&combined_attrs), "Should detect #[body] attribute");
        
        let body_param = extract_body_param_from_attrs(&combined_attrs);
        assert!(body_param.is_some());
        let (param_name, body_type) = body_param.unwrap();
        assert_eq!(param_name, "user_data");
        match body_type {
            BodyParamType::Custom(_) => {}, // Success
            _ => panic!("Expected Custom body type"),
        }
    }
}