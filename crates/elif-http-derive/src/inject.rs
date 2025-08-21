//! Service injection macro implementation
//! 
//! Provides the `#[inject]` attribute macro for declarative dependency injection.
//! Applied to struct definitions to automatically generate service fields and
//! a `from_container()` constructor method.

use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::quote;
use syn::{
    parse::{Parse, ParseStream, Result},
    parse_macro_input,
    punctuated::Punctuated,
    token::{Colon, Comma},
    Error, ItemStruct, Type,
};

/// Main implementation function for the inject macro
pub fn inject_impl(args: TokenStream, input: TokenStream) -> TokenStream {
    let inject_args = parse_macro_input!(args as InjectArgs);
    let mut item_struct = parse_macro_input!(input as ItemStruct);
    
    match process_inject_attribute(&mut item_struct, inject_args) {
        Ok(result) => result.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Parse the arguments to the #[inject(...)] macro
#[derive(Debug, Clone)]
struct InjectArgs {
    services: Punctuated<ServiceDef, Comma>,
}

impl Parse for InjectArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(InjectArgs {
            services: input.parse_terminated(ServiceDef::parse, Comma)?,
        })
    }
}

/// Represents a single service definition in the inject macro
/// Supports patterns like:
/// - `user_service: UserService` (basic service)
/// - `optional_service: Option<CacheService>` (optional service)
#[derive(Debug, Clone)]
struct ServiceDef {
    field_name: Ident,
    field_type: Type,
}

impl Parse for ServiceDef {
    fn parse(input: ParseStream) -> Result<Self> {
        let field_name: Ident = input.parse()?;
        let _colon: Colon = input.parse()?;
        let field_type: Type = input.parse()?;
        
        Ok(ServiceDef {
            field_name,
            field_type,
        })
    }
}

impl ServiceDef {
    /// Check if this is an optional service (wrapped in Option<T>)
    fn is_optional(&self) -> bool {
        if let Type::Path(type_path) = &self.field_type {
            if let Some(segment) = type_path.path.segments.last() {
                return segment.ident == "Option";
            }
        }
        false
    }
    
    /// Get the inner type for optional services
    /// For `Option<UserService>`, returns `UserService`
    fn get_inner_type(&self) -> Result<&Type> {
        if let Type::Path(type_path) = &self.field_type {
            if let Some(segment) = type_path.path.segments.last() {
                if segment.ident == "Option" {
                    if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                        if let Some(syn::GenericArgument::Type(inner_type)) = args.args.first() {
                            return Ok(inner_type);
                        }
                    }
                }
            }
        }
        
        if self.is_optional() {
            Err(Error::new_spanned(
                &self.field_type,
                "Failed to extract inner type from Option<T>",
            ))
        } else {
            Ok(&self.field_type)
        }
    }
}

/// Process the inject attribute and generate the modified struct
fn process_inject_attribute(
    item_struct: &mut ItemStruct,
    inject_args: InjectArgs,
) -> Result<proc_macro2::TokenStream> {
    if inject_args.services.is_empty() {
        return Err(Error::new(
            Span::call_site(),
            "#[inject] requires at least one service definition",
        ));
    }
    
    // Generate service fields for the struct
    let service_fields = generate_service_fields(&inject_args.services)?;
    
    // Add service fields to the struct
    match &mut item_struct.fields {
        syn::Fields::Named(fields) => {
            for field in service_fields {
                fields.named.push(field);
            }
        }
        syn::Fields::Unnamed(_) => {
            return Err(Error::new_spanned(
                item_struct,
                "#[inject] can only be applied to structs with named fields",
            ));
        }
        syn::Fields::Unit => {
            // Convert unit struct to named fields struct
            item_struct.fields = syn::Fields::Named(syn::FieldsNamed {
                brace_token: Default::default(),
                named: service_fields.into_iter().collect(),
            });
        }
    }
    
    // Generate the from_container method
    let from_container_impl = generate_from_container_method(
        &item_struct.ident,
        &inject_args.services,
    )?;
    
    // Return both the modified struct and the impl block
    Ok(quote! {
        #item_struct
        
        #from_container_impl
    })
}

/// Generate struct fields for injected services
fn generate_service_fields(
    services: &Punctuated<ServiceDef, Comma>,
) -> Result<Vec<syn::Field>> {
    let mut fields = Vec::new();
    
    for service in services {
        let field_name = &service.field_name;
        let field_type = &service.field_type;
        
        // Services are already wrapped in Arc by the container
        let wrapped_type = if service.is_optional() {
            // For Option<T>, we want Option<Arc<T>>
            let inner_type = service.get_inner_type()?;
            quote! { Option<std::sync::Arc<#inner_type>> }
        } else {
            // For T, we want Arc<T>
            quote! { std::sync::Arc<#field_type> }
        };
        
        let field = syn::Field {
            attrs: vec![],
            vis: syn::Visibility::Inherited, // private field
            mutability: syn::FieldMutability::None,
            ident: Some(field_name.clone()),
            colon_token: Some(Default::default()),
            ty: syn::parse2(wrapped_type)?,
        };
        
        fields.push(field);
    }
    
    Ok(fields)
}

/// Generate the from_container implementation
fn generate_from_container_method(
    struct_name: &Ident,
    services: &Punctuated<ServiceDef, Comma>,
) -> Result<proc_macro2::TokenStream> {
    let mut field_initializers = Vec::new();
    
    for service in services {
        let field_name = &service.field_name;
        
        if service.is_optional() {
            // Optional service - use try_resolve which returns Option<Arc<T>>
            let inner_type = service.get_inner_type()?;
            field_initializers.push(quote! {
                #field_name: container.try_resolve::<#inner_type>()
            });
        } else {
            // Required service - use resolve which returns Result<Arc<T>, CoreError>
            let service_type = &service.field_type;
            field_initializers.push(quote! {
                #field_name: container
                    .resolve::<#service_type>()
                    .map_err(|e| format!("Failed to inject service {}: {}", stringify!(#service_type), e))?
            });
        }
    }
    
    Ok(quote! {
        impl #struct_name {
            /// Create a new instance with services resolved from the DI container
            pub fn from_container(
                container: &elif_core::container::Container
            ) -> Result<Self, String> {
                Ok(Self {
                    #(#field_initializers),*
                })
            }
        }
    })
}

// Generate error type as part of the generated code rather than defining it here
// This allows each generated controller to have its own error handling