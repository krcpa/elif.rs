//! Service injection macro implementation
//! 
//! Provides the `#[inject]` attribute macro for declarative dependency injection.
//! Applied to struct definitions to automatically generate service fields and
//! a `from_ioc_container()` constructor method for use with the IoC container.

use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::quote;
use syn::{
    parse::{Parse, ParseStream, Result},
    parse_macro_input,
    punctuated::Punctuated,
    token::{Colon, Comma, Eq},
    Error, ItemStruct, Type, Expr, LitStr, Meta, Attribute,
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
/// - `cache: Cache = "redis_cache"` (named service)
/// - `#[scoped] db_context: DatabaseContext` (scoped service)
/// - `#[factory] logger: Logger = |name| Logger::for_component(name)` (factory injection)
#[derive(Debug, Clone)]
struct ServiceDef {
    field_name: Ident,
    field_type: Type,
    #[allow(dead_code)]
    service_name: Option<String>,
    injection_type: InjectionType,
    factory_expr: Option<Expr>,
    attributes: Vec<Attribute>,
}

/// Type of dependency injection to perform
#[derive(Debug, Clone, PartialEq)]
enum InjectionType {
    /// Regular service resolution
    Regular,
    /// Optional service that may not exist
    Optional,
    /// Named service with specific identifier
    Named(String),
    /// Scoped service tied to request/scope lifecycle
    Scoped,
    /// Factory-created service with custom logic
    Factory,
    /// Token-based service resolution (&TokenType)
    Token,
}

impl Parse for ServiceDef {
    fn parse(input: ParseStream) -> Result<Self> {
        // Parse any attributes first (like #[scoped], #[factory])
        let attributes = input.call(Attribute::parse_outer)?;
        
        let field_name: Ident = input.parse()?;
        let _colon: Colon = input.parse()?;
        let field_type: Type = input.parse()?;
        
        // Check for named service assignment (= "name" or = factory_expr)
        let mut service_name = None;
        let mut factory_expr = None;
        let mut injection_type = InjectionType::Regular;
        
        if input.peek(Eq) {
            let _eq: Eq = input.parse()?;
            
            // Check if it's a string literal (named service) or expression (factory)
            if input.peek(LitStr) {
                let lit: LitStr = input.parse()?;
                service_name = Some(lit.value());
                injection_type = InjectionType::Named(lit.value());
            } else {
                // Parse as factory expression
                let expr: Expr = input.parse()?;
                factory_expr = Some(expr);
                injection_type = InjectionType::Factory;
            }
        }
        
        // Check attributes for injection type modifiers
        for attr in &attributes {
            if let Meta::Path(path) = &attr.meta {
                if let Some(ident) = path.get_ident() {
                    match ident.to_string().as_str() {
                        "scoped" => injection_type = InjectionType::Scoped,
                        "factory" => injection_type = InjectionType::Factory,
                        _ => {}
                    }
                }
            }
        }
        
        // Detect special field types
        if injection_type == InjectionType::Regular {
            if is_option_type(&field_type) {
                injection_type = InjectionType::Optional;
            } else if is_token_reference(&field_type) {
                injection_type = InjectionType::Token;
            }
        }
        
        Ok(ServiceDef {
            field_name,
            field_type,
            service_name,
            injection_type,
            factory_expr,
            attributes,
        })
    }
}

/// Helper function to check if a type is Option<T>
fn is_option_type(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            return segment.ident == "Option";
        }
    }
    false
}

/// Helper function to check if a type is a reference to a token (&TokenType)
fn is_token_reference(ty: &Type) -> bool {
    if let Type::Reference(type_ref) = ty {
        // Check if it's a simple reference to a type (not a complex path)
        if let Type::Path(_type_path) = type_ref.elem.as_ref() {
            // For now, we assume any reference type is a token reference
            // In a more sophisticated implementation, we would check if the type
            // implements the ServiceToken trait, but that's not available at compile time
            return true;
        }
    }
    false
}

/// Extract the token type from a reference type (&TokenType -> TokenType)
fn extract_token_type(ty: &Type) -> Result<&Type> {
    if let Type::Reference(type_ref) = ty {
        return Ok(type_ref.elem.as_ref());
    }
    Err(Error::new_spanned(ty, "Expected reference type (&TokenType)"))
}

impl ServiceDef {
    /// Check if this is an optional service (wrapped in Option<T>)
    fn is_optional(&self) -> bool {
        matches!(self.injection_type, InjectionType::Optional) || is_option_type(&self.field_type)
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
                    // It's an Option but malformed.
                    return Err(Error::new_spanned(
                        &self.field_type,
                        "Failed to extract inner type from Option<T>",
                    ));
                }
            }
        }
        // Not an Option, so return the type itself.
        Ok(&self.field_type)
    }
    
    /// Get the service type for resolution (unwraps Option if needed)
    fn get_service_type(&self) -> Result<&Type> {
        if self.is_optional() {
            self.get_inner_type()
        } else {
            Ok(&self.field_type)
        }
    }
    
    /// Get the token type for token-based services (&TokenType -> TokenType)
    fn get_token_type(&self) -> Result<&Type> {
        if matches!(self.injection_type, InjectionType::Token) {
            extract_token_type(&self.field_type)
        } else {
            Err(Error::new_spanned(
                &self.field_type,
                "Not a token-based service definition",
            ))
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
    
    // Generate the from_ioc_container method
    let from_ioc_container_impl = generate_from_ioc_container_method(
        &item_struct.ident,
        &inject_args.services,
    )?;
    
    // Return both the modified struct and the impl block
    Ok(quote! {
        #item_struct
        
        #from_ioc_container_impl
    })
}

/// Generate struct fields for injected services
fn generate_service_fields(
    services: &Punctuated<ServiceDef, Comma>,
) -> Result<Vec<syn::Field>> {
    let mut fields = Vec::new();
    
    for service in services {
        let field_name = &service.field_name;
        
        // Determine the actual field type based on injection type
        let field_core_type = match service.injection_type {
            InjectionType::Token => {
                // For token-based services, the field should store Arc<Token::Service>
                // We need to get the service type that the token resolves to
                let token_type = service.get_token_type()?;
                quote! { std::sync::Arc<<#token_type as elif_core::container::ServiceToken>::Service> }
            },
            _ => {
                // For regular services, wrap the service type in Arc
                let service_type = service.get_service_type()?;
                quote! { std::sync::Arc<#service_type> }
            }
        };
        
        // Then wrap in Option if the service is optional (regardless of injection type)
        let wrapped_type = if service.is_optional() {
            quote! { Option<#field_core_type> }
        } else {
            field_core_type
        };
        
        let field = syn::Field {
            attrs: service.attributes.clone(),
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

/// Generate the from_ioc_container implementation for the IoC container
fn generate_from_ioc_container_method(
    struct_name: &Ident,
    services: &Punctuated<ServiceDef, Comma>,
) -> Result<proc_macro2::TokenStream> {
    let mut field_initializers = Vec::new();
    
    for service in services {
        let field_name = &service.field_name;
        let service_type = service.get_service_type()?;
        
        let initializer = if service.is_optional() {
            // Handle optional services (can be combined with any injection type)
            match &service.injection_type {
                InjectionType::Regular | InjectionType::Optional => {
                    quote! {
                        #field_name: container.try_resolve::<#service_type>()
                    }
                },
                InjectionType::Named(name) => {
                    quote! {
                        #field_name: container.try_resolve_named::<#service_type>(#name)
                    }
                },
                InjectionType::Scoped => {
                    quote! {
                        #field_name: container.try_resolve_scoped::<#service_type>(&scope_id)
                    }
                },
                InjectionType::Factory => {
                    if let Some(factory_expr) = &service.factory_expr {
                        quote! {
                            #field_name: {
                                let factory = #factory_expr;
                                Some(Arc::new(factory(container)?))
                            }
                        }
                    } else {
                        quote! {
                            #field_name: container.try_resolve::<#service_type>()
                        }
                    }
                },
                InjectionType::Token => {
                    let token_type = service.get_token_type()?;
                    quote! {
                        #field_name: container.try_resolve_by_token::<#token_type>()
                    }
                }
            }
        } else {
            // Handle required services
            match &service.injection_type {
                InjectionType::Regular => {
                    quote! {
                        #field_name: container.resolve::<#service_type>()
                            .map_err(|e| format!("Failed to inject service {}: {}", stringify!(#service_type), e))?
                    }
                },
                InjectionType::Named(name) => {
                    quote! {
                        #field_name: container.resolve_named::<#service_type>(#name)
                            .map_err(|e| format!("Failed to inject named service {}({}): {}", stringify!(#service_type), #name, e))?
                    }
                },
                InjectionType::Scoped => {
                    quote! {
                        #field_name: container.resolve_scoped::<#service_type>(&scope_id)
                            .map_err(|e| format!("Failed to inject scoped service {}: {}", stringify!(#service_type), e))?
                    }
                },
                InjectionType::Factory => {
                    if let Some(factory_expr) = &service.factory_expr {
                        quote! {
                            #field_name: {
                                let factory = #factory_expr;
                                Arc::new(factory(container)?)
                            }
                        }
                    } else {
                        quote! {
                            #field_name: container.resolve::<#service_type>()
                                .map_err(|e| format!("Failed to inject factory service {}: {}", stringify!(#service_type), e))?
                        }
                    }
                },
                InjectionType::Optional => {
                    // This shouldn't happen for non-optional services
                    quote! {
                        #field_name: container.resolve::<#service_type>()
                            .map_err(|e| format!("Failed to inject service {}: {}", stringify!(#service_type), e))?
                    }
                },
                InjectionType::Token => {
                    let token_type = service.get_token_type()?;
                    quote! {
                        #field_name: container.resolve_by_token::<#token_type>()
                            .map_err(|e| format!("Failed to inject token service {}: {}", stringify!(#token_type), e))?
                    }
                }
            }
        };
        
        field_initializers.push(initializer);
    }
    
    // Generate IoC container implementation only
    Ok(quote! {
        impl #struct_name {
            /// Create a new instance with services resolved from the IoC container
            pub fn from_ioc_container(
                container: &elif_core::container::IocContainer,
                scope: Option<&elif_core::container::ScopeId>
            ) -> Result<Self, String> {
                let scope_id = match scope {
                    Some(s) => s.clone(),
                    None => container.create_scope()
                        .map_err(|e| format!("Failed to create scope: {}", e))?
                };
                
                Ok(Self {
                    #(#field_initializers),*
                })
            }
        }
    })
}

// Generate error type as part of the generated code rather than defining it here
// This allows each generated controller to have its own error handling