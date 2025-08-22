//! Injectable derive macro implementation
//! 
//! Provides the `#[injectable]` attribute macro for automatically implementing
//! the Injectable trait by analyzing constructor parameters.

use proc_macro::TokenStream;
use proc_macro2::Ident;
use quote::quote;
use syn::{
    parse::{Result},
    parse_macro_input,
    Error, ItemStruct, Type, Item,
    PathSegment, GenericArgument, TypePath, PathArguments,
};

/// Main implementation function for the injectable macro
pub fn injectable_impl(_args: TokenStream, input: TokenStream) -> TokenStream {
    let input_item = parse_macro_input!(input as Item);
    
    match process_injectable_item(input_item) {
        Ok(result) => result.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Process the injectable attribute on different item types
fn process_injectable_item(item: Item) -> Result<proc_macro2::TokenStream> {
    match item {
        Item::Struct(item_struct) => {
            process_injectable_struct(&item_struct)
        }
        _ => Err(Error::new_spanned(
            item,
            "#[injectable] can only be applied to structs",
        )),
    }
}

/// Process a struct marked with #[injectable]
fn process_injectable_struct(item_struct: &ItemStruct) -> Result<proc_macro2::TokenStream> {
    let struct_name = &item_struct.ident;
    let (impl_generics, ty_generics, where_clause) = item_struct.generics.split_for_impl();
    
    // Find the constructor method (look for 'new' method in impl blocks)
    // For now, we'll analyze the struct fields directly
    let (dependencies, non_dependency_fields) = extract_dependencies_from_struct(item_struct)?;
    
    let injectable_impl = generate_injectable_impl(
        struct_name,
        &dependencies,
        &non_dependency_fields,
        impl_generics,
        ty_generics,
        where_clause,
    )?;
    
    Ok(quote! {
        #item_struct
        
        #injectable_impl
    })
}

/// Dependency information extracted from struct fields or constructor parameters
#[derive(Debug, Clone)]
struct DependencyInfo {
    /// The type to inject (inner type for Arc<T> or Option<Arc<T>>)
    service_type: Type,
    /// Whether this dependency is optional
    is_optional: bool,
    /// The parameter name for the constructor
    param_name: Ident,
}

/// Extract dependency information from struct fields
fn extract_dependencies_from_struct(item_struct: &ItemStruct) -> Result<(Vec<DependencyInfo>, Vec<proc_macro2::TokenStream>)> {
    let mut dependencies = Vec::new();
    let mut non_dependency_fields = Vec::new();
    
    match &item_struct.fields {
        syn::Fields::Named(fields) => {
            for field in &fields.named {
                if let Some(field_name) = &field.ident {
                    // Try to analyze the field type - skip non-dependency fields
                    if let Ok(dep_info) = analyze_field_type(&field.ty, field_name.clone()) {
                        dependencies.push(dep_info);
                    } else {
                        // Handle non-dependency fields (like PhantomData, primitive types, etc.)
                        non_dependency_fields.push(generate_non_dependency_field_initializer(field_name, &field.ty));
                    }
                }
            }
        }
        _ => {
            return Err(Error::new_spanned(
                item_struct,
                "#[injectable] requires structs with named fields",
            ));
        }
    }
    
    if dependencies.is_empty() {
        return Err(Error::new_spanned(
            item_struct,
            "Injectable structs must have at least one dependency field (Arc<T> or Option<Arc<T>>)",
        ));
    }
    
    Ok((dependencies, non_dependency_fields))
}

/// Generate initializer for non-dependency fields
fn generate_non_dependency_field_initializer(field_name: &Ident, field_type: &Type) -> proc_macro2::TokenStream {
    // Check if it's PhantomData
    if let Type::Path(type_path) = field_type {
        if let Some(segment) = type_path.path.segments.last() {
            if segment.ident == "PhantomData" {
                return quote! {
                    #field_name: std::marker::PhantomData
                };
            }
        }
    }
    
    // For other types, try Default::default()
    quote! {
        #field_name: Default::default()
    }
}

/// Analyze a field type to extract dependency information
/// Supports: Arc<T>, Option<Arc<T>>
fn analyze_field_type(field_type: &Type, field_name: Ident) -> Result<DependencyInfo> {
    match field_type {
        Type::Path(type_path) => {
            analyze_type_path(type_path, field_name)
        }
        _ => Err(Error::new_spanned(
            field_type,
            "Unsupported field type for dependency injection. Use Arc<T> or Option<Arc<T>>",
        )),
    }
}

/// Analyze a type path to determine the dependency type
fn analyze_type_path(type_path: &TypePath, field_name: Ident) -> Result<DependencyInfo> {
    if let Some(segment) = type_path.path.segments.last() {
        match segment.ident.to_string().as_str() {
            "Option" => {
                // Option<Arc<T>> pattern
                let inner_type = extract_generic_type(segment, "Option")?;
                let arc_inner = extract_arc_inner_type(&inner_type)?;
                
                Ok(DependencyInfo {
                    service_type: arc_inner,
                    is_optional: true,
                    param_name: field_name,
                })
            }
            "Arc" => {
                // Arc<T> pattern
                let inner_type = extract_generic_type(segment, "Arc")?;
                
                Ok(DependencyInfo {
                    service_type: inner_type,
                    is_optional: false,
                    param_name: field_name,
                })
            }
            _ => Err(Error::new_spanned(
                type_path,
                "Dependency injection fields must be Arc<T> or Option<Arc<T>>",
            )),
        }
    } else {
        Err(Error::new_spanned(
            type_path,
            "Invalid type path",
        ))
    }
}

/// Extract the generic type from a type segment
fn extract_generic_type(segment: &PathSegment, expected_name: &str) -> Result<Type> {
    if let PathArguments::AngleBracketed(args) = &segment.arguments {
        if let Some(GenericArgument::Type(inner_type)) = args.args.first() {
            return Ok(inner_type.clone());
        }
    }
    
    Err(Error::new_spanned(
        segment,
        format!("Failed to extract generic type from {}<T>", expected_name),
    ))
}

/// Extract the inner type from Arc<T>
fn extract_arc_inner_type(ty: &Type) -> Result<Type> {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            if segment.ident == "Arc" {
                return extract_generic_type(segment, "Arc");
            }
        }
    }
    
    Err(Error::new_spanned(
        ty,
        "Expected Arc<T> type",
    ))
}

/// Generate the Injectable trait implementation
fn generate_injectable_impl(
    struct_name: &Ident,
    dependencies: &[DependencyInfo],
    non_dependency_fields: &[proc_macro2::TokenStream],
    impl_generics: syn::ImplGenerics,
    ty_generics: syn::TypeGenerics,
    where_clause: Option<&syn::WhereClause>,
) -> Result<proc_macro2::TokenStream> {
    // Generate the dependencies list
    let dependency_service_ids: Vec<proc_macro2::TokenStream> = dependencies
        .iter()
        .map(|dep| {
            let service_type = &dep.service_type;
            quote! {
                elif_core::container::ServiceId::of::<#service_type>()
            }
        })
        .collect();
    
    // Generate the create method body
    let field_initializers: Vec<proc_macro2::TokenStream> = dependencies
        .iter()
        .map(|dep| {
            let param_name = &dep.param_name;
            let service_type = &dep.service_type;
            
            if dep.is_optional {
                quote! {
                    #param_name: resolver.try_resolve::<#service_type>()
                }
            } else {
                quote! {
                    #param_name: resolver.resolve::<#service_type>()?
                }
            }
        })
        .collect();
    
    Ok(quote! {
        impl #impl_generics elif_core::container::Injectable for #struct_name #ty_generics #where_clause {
            fn dependencies() -> Vec<elif_core::container::ServiceId> {
                vec![#(#dependency_service_ids),*]
            }
            
            fn create<R: elif_core::container::DependencyResolver>(resolver: &R) -> Result<Self, elif_core::errors::CoreError>
            where
                Self: Sized,
            {
                Ok(Self {
                    #(#field_initializers,)*
                    #(#non_dependency_fields,)*
                })
            }
        }
    })
}