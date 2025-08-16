/*!
Procedural macros for OpenAPI schema generation.

This crate provides derive macros for automatically implementing OpenAPI schema traits.
*/

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Data, Fields};

/// Derive macro to automatically implement OpenApiSchema for structs and enums
#[proc_macro_derive(OpenApiSchema)]
pub fn derive_openapi_schema(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    generate_openapi_schema_impl(&input).unwrap_or_else(|err| err.to_compile_error().into())
}

/// Generate implementation for OpenApiSchema trait
fn generate_openapi_schema_impl(input: &DeriveInput) -> Result<TokenStream, syn::Error> {
    let name = &input.ident;
    let name_str = name.to_string();
    
    let schema_impl = match &input.data {
        Data::Struct(data_struct) => generate_struct_schema_impl(&name_str, &data_struct.fields)?,
        Data::Enum(data_enum) => generate_enum_schema_impl(&name_str, data_enum)?,
        Data::Union(_) => {
            return Err(syn::Error::new_spanned(
                input, 
                "OpenApiSchema cannot be derived for union types"
            ));
        }
    };

    let expanded = quote! {
        impl ::elif_openapi::OpenApiSchema for #name {
            fn openapi_schema() -> ::elif_openapi::specification::Schema {
                #schema_impl
            }
            
            fn schema_name() -> String {
                #name_str.to_string()
            }
        }
    };

    Ok(expanded.into())
}

/// Generate schema implementation for struct types
fn generate_struct_schema_impl(type_name: &str, fields: &Fields) -> Result<proc_macro2::TokenStream, syn::Error> {
    match fields {
        Fields::Named(named_fields) => {
            // Generate object schema with properties
            let mut properties = Vec::new();
            let mut required = Vec::new();

            for field in &named_fields.named {
                let field_name = field.ident.as_ref().unwrap().to_string();
                let field_type = &field.ty;
                
                // Check if field is optional (Option<T>)
                let is_optional = is_option_type(field_type);
                
                if !is_optional {
                    required.push(field_name.clone());
                }

                // Generate property schema
                let property_schema = quote! {
                    <#field_type as ::elif_openapi::OpenApiSchema>::openapi_schema()
                };

                properties.push(quote! {
                    properties.insert(#field_name.to_string(), #property_schema);
                });
            }

            let required_fields = if required.is_empty() {
                quote! { Vec::new() }
            } else {
                quote! { vec![#(#required.to_string()),*] }
            };

            Ok(quote! {
                {
                    let mut properties = std::collections::HashMap::new();
                    #(#properties)*
                    
                    ::elif_openapi::specification::Schema {
                        schema_type: Some("object".to_string()),
                        title: Some(#type_name.to_string()),
                        properties: properties,
                        required: #required_fields,
                        ..Default::default()
                    }
                }
            })
        }
        Fields::Unnamed(unnamed_fields) => {
            // Generate tuple schema
            if unnamed_fields.unnamed.len() == 1 {
                // Single field tuple - use the inner type's schema
                let field_type = &unnamed_fields.unnamed.first().unwrap().ty;
                Ok(quote! {
                    <#field_type as ::elif_openapi::OpenApiSchema>::openapi_schema()
                })
            } else {
                // Multiple field tuple - use array with descriptive text
                // OpenAPI 3.0 does not have good tuple support (OpenAPI 3.1 introduced prefixItems)
                let type_descriptions: Vec<String> = unnamed_fields.unnamed.iter()
                    .map(|field| quote::quote!(#field.ty).to_string())
                    .collect();
                
                let field_count = unnamed_fields.unnamed.len();
                
                let description = format!(
                    "A tuple with {} fields in fixed order: ({}). Note: OpenAPI 3.0 cannot precisely represent tuple types - this is a generic array representation.",
                    field_count,
                    type_descriptions.join(", ")
                );

                Ok(quote! {
                    ::elif_openapi::specification::Schema {
                        schema_type: Some("array".to_string()),
                        title: Some(#type_name.to_string()),
                        description: Some(#description.to_string()),
                        // For OpenAPI 3.0, we use a generic array representation
                        // OpenAPI 3.1 would use prefixItems for proper tuple support
                        // Note: minItems/maxItems constraints are not available in this Schema implementation
                        items: Some(Box::new(::elif_openapi::specification::Schema {
                            description: Some("Tuple element (type varies by position)".to_string()),
                            ..Default::default()
                        })),
                        ..Default::default()
                    }
                })
            }
        }
        Fields::Unit => {
            // Unit struct - use null schema
            Ok(quote! {
                ::elif_openapi::specification::Schema {
                    schema_type: Some("null".to_string()),
                    title: Some(#type_name.to_string()),
                    ..Default::default()
                }
            })
        }
    }
}

/// Generate schema implementation for enum types
fn generate_enum_schema_impl(type_name: &str, data_enum: &syn::DataEnum) -> Result<proc_macro2::TokenStream, syn::Error> {
    // For now, generate simple string enum schema
    let variants: Vec<String> = data_enum.variants.iter()
        .map(|variant| variant.ident.to_string())
        .collect();

    Ok(quote! {
        ::elif_openapi::specification::Schema {
            schema_type: Some("string".to_string()),
            title: Some(#type_name.to_string()),
            enum_values: vec![#(serde_json::Value::String(#variants.to_string())),*],
            ..Default::default()
        }
    })
}

/// Helper function to check if a type is Option<T>
fn is_option_type(ty: &syn::Type) -> bool {
    if let syn::Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            return segment.ident == "Option";
        }
    }
    false
}