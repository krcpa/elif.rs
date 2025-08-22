//! Module system macro implementation
//! 
//! Provides the `#[module(...)]` attribute macro and `module! { ... }` function-like macro
//! for defining dependency injection modules and application composition.
//!
//! Features:
//! - Provider definitions with trait mappings: `EmailService => SmtpEmailService @ "smtp"`
//! - Controller registration: `controllers: [Controller1, Controller2]`
//! - Module imports/exports: `imports: [Module], exports: [Service]`
//! - Application composition: `app! { modules: [Module1, Module2] }`

use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::quote;
use syn::{
    parse::{Parse, ParseStream, Result},
    parse_macro_input,
    punctuated::Punctuated,
    token::{Comma, FatArrow, At},
    Error, ItemStruct, Type, LitStr, Token,
};

/// Main implementation function for the module attribute macro
pub fn module_impl(args: TokenStream, input: TokenStream) -> TokenStream {
    let module_args = match syn::parse::<ModuleArgs>(args) {
        Ok(args) => args,
        Err(err) => return err.to_compile_error().into(),
    };
    
    let mut item_struct = parse_macro_input!(input as ItemStruct);
    
    match process_module_attribute(&mut item_struct, module_args) {
        Ok(result) => result.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Main implementation function for the module function-like macro
pub fn module_composition_impl(input: TokenStream) -> TokenStream {
    let composition_args = match syn::parse::<ModuleCompositionArgs>(input) {
        Ok(args) => args,
        Err(err) => return err.to_compile_error().into(),
    };
    
    match generate_application_composition(composition_args) {
        Ok(result) => result.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Arguments parsed from the #[module(...)] attribute
#[derive(Debug, Clone)]
pub struct ModuleArgs {
    pub providers: Vec<ProviderDef>,
    pub controllers: Vec<Type>,
    pub imports: Vec<Type>,
    pub exports: Vec<Type>,
}

impl Parse for ModuleArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut providers = Vec::new();
        let mut controllers = Vec::new();
        let mut imports = Vec::new();
        let mut exports = Vec::new();
        
        // Parse comma-separated key-value pairs
        while !input.is_empty() {
            let key: Ident = input.parse()?;
            let _colon: Token![:] = input.parse()?;
            
            let key_str = key.to_string();
            match key_str.as_str() {
                "providers" => {
                    providers = parse_provider_list(input)?;
                },
                "controllers" => {
                    controllers = parse_type_list(input)?;
                },
                "imports" => {
                    imports = parse_type_list(input)?;
                },
                "exports" => {
                    exports = parse_type_list(input)?;
                },
                _ => {
                    return Err(Error::new_spanned(
                        key,
                        format!(
                            "Unknown module section '{}'. Valid sections are: providers, controllers, imports, exports",
                            key_str
                        )
                    ));
                }
            }
            
            // Optional comma between sections
            if !input.is_empty() {
                let _comma: Option<Comma> = input.parse().ok();
            }
        }
        
        Ok(ModuleArgs {
            providers,
            controllers,
            imports,
            exports,
        })
    }
}

/// Definition of a provider in the module
/// Supports various patterns:
/// - `UserService` (concrete service)
/// - `EmailService => SmtpEmailService` (trait mapping)
/// - `EmailService => SmtpEmailService @ "smtp"` (named trait mapping)
/// - `dyn EmailService => SmtpEmailService` (explicit dyn syntax still supported)
#[derive(Debug, Clone)]
pub struct ProviderDef {
    pub service_type: ProviderType,
    pub implementation: Option<Type>,
    pub name: Option<String>,
}

#[derive(Debug, Clone)]
pub enum ProviderType {
    /// Concrete service type: UserService
    Concrete(Type),
    /// Trait type: dyn EmailService
    Trait(Type),
}

impl Parse for ProviderDef {
    fn parse(input: ParseStream) -> Result<Self> {
        // Parse the service type (may be dyn Trait, bare Trait, or concrete type)
        let service_type = if input.peek(Token![dyn]) {
            // Explicit dyn Trait syntax
            let _dyn: Token![dyn] = input.parse()?;
            let trait_type: Type = input.parse()?;
            ProviderType::Trait(trait_type)
        } else {
            let parsed_type: Type = input.parse()?;
            
            // Check if this will be followed by => (trait mapping)
            if input.peek(FatArrow) {
                // If there's a =>, it's a trait mapping, so treat as trait
                ProviderType::Trait(parsed_type)
            } else {
                // No =>, so it's a concrete service
                ProviderType::Concrete(parsed_type)
            }
        };
        
        let mut implementation = None;
        let mut name = None;
        
        // Check for trait mapping: => Implementation
        if input.peek(FatArrow) {
            let _arrow: FatArrow = input.parse()?;
            implementation = Some(input.parse()?);
            
            // Check for named mapping: @ "name"
            if input.peek(At) {
                let _at: At = input.parse()?;
                let name_lit: LitStr = input.parse()?;
                name = Some(name_lit.value());
            }
        }
        
        Ok(ProviderDef {
            service_type,
            implementation,
            name,
        })
    }
}

/// Parse a list of providers: [Provider1, dyn Trait => Impl, ...]
fn parse_provider_list(input: ParseStream) -> Result<Vec<ProviderDef>> {
    let content;
    let _bracket = syn::bracketed!(content in input);
    let providers: Punctuated<ProviderDef, Comma> = content.parse_terminated(ProviderDef::parse, Comma)?;
    Ok(providers.into_iter().collect())
}

/// Parse a list of types: [Type1, Type2, ...]
fn parse_type_list(input: ParseStream) -> Result<Vec<Type>> {
    let content;
    let _bracket = syn::bracketed!(content in input);
    let types: Punctuated<Type, Comma> = content.parse_terminated(Type::parse, Comma)?;
    Ok(types.into_iter().collect())
}

/// Arguments for module composition macro: module! { ... }
#[derive(Debug, Clone)]
pub struct ModuleCompositionArgs {
    pub modules: Vec<Type>,
    pub overrides: Vec<ProviderDef>,
}

impl Parse for ModuleCompositionArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut modules = Vec::new();
        let mut overrides = Vec::new();
        
        while !input.is_empty() {
            let key: Ident = input.parse()?;
            let _colon: Token![:] = input.parse()?;
            
            let key_str = key.to_string();
            match key_str.as_str() {
                "modules" => {
                    modules = parse_type_list(input)?;
                },
                "overrides" => {
                    overrides = parse_provider_list(input)?;
                },
                _ => {
                    return Err(Error::new_spanned(
                        key,
                        format!(
                            "Unknown composition section '{}'. Valid sections are: modules, overrides",
                            key_str
                        )
                    ));
                }
            }
            
            if !input.is_empty() {
                let _comma: Option<Comma> = input.parse().ok();
            }
        }
        
        if modules.is_empty() {
            return Err(Error::new(
                Span::call_site(),
                "module! composition requires at least one module in the 'modules' section"
            ));
        }
        
        Ok(ModuleCompositionArgs {
            modules,
            overrides,
        })
    }
}

/// Process the module attribute and generate module registration code
fn process_module_attribute(
    item_struct: &mut ItemStruct,
    module_args: ModuleArgs,
) -> Result<proc_macro2::TokenStream> {
    let struct_name = &item_struct.ident;
    
    // Generate module descriptor method
    let module_descriptor_impl = generate_module_descriptor_method(struct_name, &module_args)?;
    
    Ok(quote! {
        #item_struct
        
        #module_descriptor_impl
    })
}

/// Generate module descriptor method for runtime registration
fn generate_module_descriptor_method(
    struct_name: &Ident,
    module_args: &ModuleArgs,
) -> Result<proc_macro2::TokenStream> {
    let providers_code = generate_providers_registration(&module_args.providers)?;
    let controllers_code = generate_controllers_registration(&module_args.controllers)?;
    let imports_code = generate_imports_registration(&module_args.imports)?;
    let exports_code = generate_exports_registration(&module_args.exports)?;
    
    let descriptor_name = quote::format_ident!("{}ModuleDescriptor", struct_name);
    
    Ok(quote! {
        impl #struct_name {
            /// Get the module descriptor for this module
            /// Note: This is a stub implementation for Epic 1 (Parser Foundation)
            /// Full integration with elif-core will be implemented in Epic 4 (Runtime Integration)
            pub fn module_descriptor() -> #descriptor_name {
                let mut descriptor = #descriptor_name::new(stringify!(#struct_name));
                
                #providers_code
                #controllers_code
                #imports_code
                #exports_code
                
                descriptor
            }
        }
        
        /// Stub implementation for module descriptor - will be replaced with real elif-core types in Epic 4
        #[derive(Debug, Clone)]
        pub struct #descriptor_name {
            name: String,
        }
        
        impl #descriptor_name {
            pub fn new(name: &str) -> Self {
                Self {
                    name: name.to_string(),
                }
            }
            
            pub fn name(&self) -> &str {
                &self.name
            }
        }
    })
}

/// Generate provider registration code  
fn generate_providers_registration(providers: &[ProviderDef]) -> Result<proc_macro2::TokenStream> {
    if providers.is_empty() {
        return Ok(quote! {
            // No providers specified
        });
    }
    
    let mut comments = Vec::new();
    
    for provider in providers {
        let comment = match &provider.service_type {
            ProviderType::Concrete(_service_type) => {
                match &provider.name {
                    Some(_name) => {
                        quote! {
                            // Provider: #service_type (named: #name)
                        }
                    },
                    None => {
                        quote! {
                            // Provider: #service_type
                        }
                    }
                }
            },
            ProviderType::Trait(trait_type) => {
                match &provider.implementation {
                    Some(_impl_type) => {
                        match &provider.name {
                            Some(_name) => {
                                quote! {
                                    // Provider: dyn #trait_type => #impl_type (named: #name)
                                }
                            },
                            None => {
                                quote! {
                                    // Provider: dyn #trait_type => #impl_type
                                }
                            }
                        }
                    },
                    None => {
                        return Err(Error::new_spanned(
                            trait_type,
                            "Trait providers must specify implementation type: dyn Trait => Implementation"
                        ));
                    }
                }
            }
        };
        
        comments.push(comment);
    }
    
    Ok(quote! {
        #(#comments)*
    })
}

/// Generate controller registration code
fn generate_controllers_registration(controllers: &[Type]) -> Result<proc_macro2::TokenStream> {
    if controllers.is_empty() {
        return Ok(quote! {
            // No controllers specified
        });
    }
    
    let comments: Vec<_> = controllers.iter().map(|_controller| {
        quote! {
            // Controller: #controller
        }
    }).collect();
    
    Ok(quote! {
        #(#comments)*
    })
}

/// Generate imports registration code  
fn generate_imports_registration(imports: &[Type]) -> Result<proc_macro2::TokenStream> {
    if imports.is_empty() {
        return Ok(quote! {
            // No imports specified
        });
    }
    
    let comments: Vec<_> = imports.iter().map(|_import| {
        quote! {
            // Import: #import
        }
    }).collect();
    
    Ok(quote! {
        #(#comments)*
    })
}

/// Generate exports registration code
fn generate_exports_registration(exports: &[Type]) -> Result<proc_macro2::TokenStream> {
    if exports.is_empty() {
        return Ok(quote! {
            // No exports specified
        });
    }
    
    let comments: Vec<_> = exports.iter().map(|_export| {
        quote! {
            // Export: #export
        }
    }).collect();
    
    Ok(quote! {
        #(#comments)*
    })
}

/// Generate application composition code
fn generate_application_composition(
    composition_args: ModuleCompositionArgs,
) -> Result<proc_macro2::TokenStream> {
    let modules_registration = generate_modules_registration(&composition_args.modules)?;
    let overrides_registration = generate_overrides_registration(&composition_args.overrides)?;
    
    Ok(quote! {
        {
            // Stub implementation for Epic 1 (Parser Foundation)
            // Full runtime integration will be implemented in Epic 4
            
            #modules_registration
            #overrides_registration
            
            // Return placeholder
            ()
        }
    })
}

/// Generate modules registration for application composition
fn generate_modules_registration(modules: &[Type]) -> Result<proc_macro2::TokenStream> {
    let comments: Vec<_> = modules.iter().map(|_module| {
        quote! {
            // Module: #module
        }
    }).collect();
    
    Ok(quote! {
        #(#comments)*
    })
}

/// Generate overrides registration for application composition
fn generate_overrides_registration(overrides: &[ProviderDef]) -> Result<proc_macro2::TokenStream> {
    if overrides.is_empty() {
        return Ok(quote! {
            // No overrides specified
        });
    }
    
    let mut comments = Vec::new();
    
    for override_def in overrides {
        let comment = match &override_def.service_type {
            ProviderType::Concrete(_service_type) => {
                match &override_def.name {
                    Some(_name) => {
                        quote! {
                            // Override: #service_type (named: #name)
                        }
                    },
                    None => {
                        quote! {
                            // Override: #service_type
                        }
                    }
                }
            },
            ProviderType::Trait(trait_type) => {
                match &override_def.implementation {
                    Some(_impl_type) => {
                        match &override_def.name {
                            Some(_name) => {
                                quote! {
                                    // Override: dyn #trait_type => #impl_type (named: #name)
                                }
                            },
                            None => {
                                quote! {
                                    // Override: dyn #trait_type => #impl_type
                                }
                            }
                        }
                    },
                    None => {
                        return Err(Error::new_spanned(
                            trait_type,
                            "Trait overrides must specify implementation type: dyn Trait => Implementation"
                        ));
                    }
                }
            }
        };
        
        comments.push(comment);
    }
    
    Ok(quote! {
        #(#comments)*
    })
}