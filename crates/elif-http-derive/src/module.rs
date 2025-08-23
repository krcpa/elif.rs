//! Module system macro implementation
//! 
//! Provides comprehensive macros for defining dependency injection modules:
//!
//! ## Macros
//! 
//! - `#[module(...)]`: Define modules with providers, controllers, imports, exports
//! - `module_composition!`: Compose multiple modules into applications  
//! - `demo_module!`: Laravel-style simplified syntax for rapid development
//!
//! ## Features
//! 
//! - **Provider definitions**: Concrete services and trait mappings
//! - **Controller registration**: Automatic dependency injection for controllers
//! - **Module composition**: Import/export system for module dependencies
//! - **Compile-time validation**: Type-safe dependency resolution
//! - **IDE support**: Full rust-analyzer integration with autocompletion
//!
//! ## Examples
//!
//! ```rust
//! use elif_http_derive::{module, demo_module, module_composition};
//!
//! // Full syntax
//! #[module(
//!     providers: [UserService, dyn EmailService => SmtpEmailService @ "smtp"],
//!     controllers: [UserController],
//!     exports: [UserService, dyn EmailService]
//! )]
//! pub struct UserModule;
//!
//! // Demo DSL syntax  
//! let simple_module = demo_module! {
//!     services: [UserService, EmailService],
//!     controllers: [UserController],
//!     middleware: ["cors", "auth"]
//! };
//!
//! // Application composition
//! let app = module_composition! {
//!     modules: [UserModule, AuthModule],
//!     overrides: [dyn EmailService => MockEmailService @ "test"]
//! };
//! ```

use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::{quote, ToTokens};
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

/// Demo DSL sugar syntax implementation
/// Supports Laravel-style simplified syntax for common cases:
/// ```
/// module! {
///     services: [UserService, EmailService],
///     controllers: [UserController, PostController],
///     middleware: ["cors", "logging"]
/// }
/// ```
pub fn demo_dsl_impl(input: TokenStream) -> TokenStream {
    let demo_args = match syn::parse::<DemoDslArgs>(input) {
        Ok(args) => args,
        Err(err) => return err.to_compile_error().into(),
    };
    
    match generate_demo_dsl_expansion(demo_args) {
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
                            "Unknown module section '{}'. Valid sections are: providers, controllers, imports, exports.\n\
                            \n\
                            💡 Suggestions:\n\
                            • Use 'providers: [ServiceType]' for concrete services\n\
                            • Use 'providers: [dyn Trait => Implementation]' for trait mappings\n\
                            • Use 'controllers: [ControllerType]' for HTTP controllers\n\
                            • Use 'imports: [ModuleType]' for module dependencies\n\
                            • Use 'exports: [ServiceType]' for services available to other modules\n\
                            \n\
                            📖 See: https://docs.elif.rs/modules/module-definition",
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

/// Parse a list of strings: ["string1", "string2", ...]
fn parse_string_list(input: ParseStream) -> Result<Vec<String>> {
    let content;
    let _bracket = syn::bracketed!(content in input);
    let strings: Punctuated<LitStr, Comma> = content.parse_terminated(|input| input.parse::<LitStr>(), Comma)?;
    Ok(strings.into_iter().map(|s| s.value()).collect())
}

/// Arguments for module composition macro: module! { ... }
#[derive(Debug, Clone)]
pub struct ModuleCompositionArgs {
    pub modules: Vec<Type>,
    pub overrides: Vec<ProviderDef>,
}

/// Arguments for demo DSL sugar syntax: module! { ... }
/// Supports Laravel-style simplified syntax
#[derive(Debug, Clone, Default)]
pub struct DemoDslArgs {
    pub services: Vec<Type>,
    pub controllers: Vec<Type>, 
    pub middleware: Vec<String>,
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
                            "Unknown composition section '{}'. Valid sections are: modules, overrides.\n\
                            \n\
                            💡 Suggestions:\n\
                            • Use 'modules: [ModuleType1, ModuleType2]' to compose multiple modules\n\
                            • Use 'overrides: [Service => Implementation]' to override module bindings\n\
                            \n\
                            📖 Example:\n\
                            module_composition! {{\n\
                                modules: [UserModule, AuthModule],\n\
                                overrides: [dyn EmailService => MockEmailService @ \"test\"]\n\
                            }}\n\
                            \n\
                            📖 See: https://docs.elif.rs/modules/application-composition",
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

impl Parse for DemoDslArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut services = Vec::new();
        let mut controllers = Vec::new(); 
        let mut middleware = Vec::new();
        
        while !input.is_empty() {
            let key: Ident = input.parse()?;
            let _colon: Token![:] = input.parse()?;
            
            let key_str = key.to_string();
            match key_str.as_str() {
                "services" => {
                    services = parse_type_list(input)?;
                },
                "controllers" => {
                    controllers = parse_type_list(input)?;
                },
                "middleware" => {
                    middleware = parse_string_list(input)?;
                },
                _ => {
                    return Err(Error::new_spanned(
                        key,
                        format!(
                            "Unknown demo DSL section '{}'. Valid sections are: services, controllers, middleware.\n\
                            \n\
                            💡 Demo DSL Suggestions:\n\
                            • Use 'services: [ServiceType1, ServiceType2]' for concrete services\n\
                            • Use 'controllers: [ControllerType1, ControllerType2]' for HTTP controllers\n\
                            • Use 'middleware: [\"cors\", \"auth\", \"logging\"]' for middleware stack\n\
                            \n\
                            📖 Example:\n\
                            demo_module! {{\n\
                                services: [UserService, EmailService],\n\
                                controllers: [UserController],\n\
                                middleware: [\"cors\", \"auth\"]\n\
                            }}\n\
                            \n\
                            ⚠️ Note: Demo DSL is simplified syntax. For trait mappings and imports/exports,\n\
                            use the full #[module(...)] attribute syntax instead.\n\
                            \n\
                            📖 See: https://docs.elif.rs/modules/demo-dsl-guide",
                            key_str
                        )
                    ));
                }
            }
            
            if !input.is_empty() {
                let _comma: Option<Comma> = input.parse().ok();
            }
        }
        
        Ok(DemoDslArgs {
            services,
            controllers,
            middleware,
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
    let providers_code = generate_providers_descriptors(&module_args.providers)?;
    let controllers_code = generate_controllers_descriptors(&module_args.controllers)?;
    let imports_list = generate_imports_list(&module_args.imports)?;
    let exports_list = generate_exports_list(&module_args.exports)?;
    let auto_configure_code = generate_auto_configure_function(struct_name, module_args)?;
    
    Ok(quote! {
        impl #struct_name {
            /// Get the module descriptor for this module
            pub fn module_descriptor() -> elif_core::modules::ModuleDescriptor {
                use elif_core::modules::{ModuleDescriptor, ServiceDescriptor, ControllerDescriptor, ServiceLifecycle};
                use std::any::TypeId;
                
                let mut descriptor = ModuleDescriptor::new(stringify!(#struct_name));
                
                // Add providers
                #providers_code
                
                // Add controllers  
                #controllers_code
                
                // Set imports and exports
                descriptor = descriptor
                    .with_imports(#imports_list)
                    .with_exports(#exports_list);
                
                descriptor
            }
        }
        
        impl elif_core::modules::ModuleAutoConfiguration for #struct_name {
            fn module_descriptor() -> elif_core::modules::ModuleDescriptor {
                Self::module_descriptor()
            }
            
            fn auto_configure(container: &mut elif_core::container::IocContainer) -> Result<(), elif_core::modules::ModuleError> {
                #auto_configure_code
            }
        }
    })
}

/// Generate provider descriptors for module descriptor creation
fn generate_providers_descriptors(providers: &[ProviderDef]) -> Result<proc_macro2::TokenStream> {
    if providers.is_empty() {
        return Ok(quote! {
            // No providers specified
        });
    }
    
    let mut descriptor_calls = Vec::new();
    
    for provider in providers {
        let descriptor_call = match &provider.service_type {
            ProviderType::Concrete(service_type) => {
                match &provider.name {
                    Some(name) => {
                        quote! {
                            descriptor = descriptor.with_provider(
                                ServiceDescriptor::new::<#service_type>(stringify!(#service_type), ServiceLifecycle::default())
                                    .with_name(#name)
                            );
                        }
                    },
                    None => {
                        quote! {
                            descriptor = descriptor.with_provider(
                                ServiceDescriptor::new::<#service_type>(stringify!(#service_type), ServiceLifecycle::default())
                            );
                        }
                    }
                }
            },
            ProviderType::Trait(trait_type) => {
                match &provider.implementation {
                    Some(impl_type) => {
                        match &provider.name {
                            Some(name) => {
                                quote! {
                                    descriptor = descriptor.with_provider(
                                        ServiceDescriptor::trait_mapping::<#trait_type, #impl_type>(
                                            stringify!(#trait_type), stringify!(#impl_type), ServiceLifecycle::default()
                                        ).with_name(#name)
                                    );
                                }
                            },
                            None => {
                                quote! {
                                    descriptor = descriptor.with_provider(
                                        ServiceDescriptor::trait_mapping::<#trait_type, #impl_type>(
                                            stringify!(#trait_type), stringify!(#impl_type), ServiceLifecycle::default()
                                        )
                                    );
                                }
                            }
                        }
                    },
                    None => {
                        return Err(Error::new_spanned(
                            trait_type,
                            "Trait providers must specify implementation type: dyn Trait => Implementation.\n\
                            \n\
                            💡 Suggestions:\n\
                            • Use 'dyn EmailService => SmtpEmailService' for trait mapping\n\
                            • Use 'dyn EmailService => SmtpEmailService @ \"smtp\"' for named mapping\n\
                            • Use 'EmailService => SmtpEmailService' (dyn is optional in simplified syntax)\n\
                            \n\
                            📖 Examples:\n\
                            #[module(\n\
                                providers: [\n\
                                    UserService,  // Concrete service\n\
                                    dyn EmailService => SmtpEmailService,  // Trait mapping\n\
                                    dyn EmailService => MockEmailService @ \"test\"  // Named mapping\n\
                                ]\n\
                            )]\n\
                            \n\
                            📖 See: https://docs.elif.rs/modules/dependency-injection"
                        ));
                    }
                }
            }
        };
        
        descriptor_calls.push(descriptor_call);
    }
    
    Ok(quote! {
        #(#descriptor_calls)*
    })
}

/// Generate controller descriptors for module descriptor creation
fn generate_controllers_descriptors(controllers: &[Type]) -> Result<proc_macro2::TokenStream> {
    if controllers.is_empty() {
        return Ok(quote! {
            // No controllers specified
        });
    }
    
    let descriptor_calls: Vec<_> = controllers.iter().map(|controller| {
        quote! {
            descriptor = descriptor.with_controller(
                ControllerDescriptor::new::<#controller>(stringify!(#controller))
            );
        }
    }).collect();
    
    Ok(quote! {
        #(#descriptor_calls)*
    })
}

/// Generate imports list for module descriptor
fn generate_imports_list(imports: &[Type]) -> Result<proc_macro2::TokenStream> {
    if imports.is_empty() {
        return Ok(quote! { vec![] });
    }
    
    let import_strings: Vec<_> = imports.iter().map(|import| {
        quote! { stringify!(#import).to_string() }
    }).collect();
    
    Ok(quote! {
        vec![#(#import_strings),*]
    })
}

/// Generate exports list for module descriptor
fn generate_exports_list(exports: &[Type]) -> Result<proc_macro2::TokenStream> {
    if exports.is_empty() {
        return Ok(quote! { vec![] });
    }
    
    let export_strings: Vec<_> = exports.iter().map(|export| {
        quote! { stringify!(#export).to_string() }
    }).collect();
    
    Ok(quote! {
        vec![#(#export_strings),*]
    })
}

/// Generate auto-configure function for IoC container integration
fn generate_auto_configure_function(
    _struct_name: &Ident,
    module_args: &ModuleArgs,
) -> Result<proc_macro2::TokenStream> {
    let mut configure_calls = Vec::new();
    
    // First, configure imported modules (dependencies must be resolved first)
    for import in &module_args.imports {
        configure_calls.push(quote! {
            <#import as elif_core::modules::ModuleAutoConfiguration>::auto_configure(container)?;
        });
    }
    
    // Configure providers with lifecycle and dependency metadata
    for provider in &module_args.providers {
        let configure_call = match &provider.service_type {
            ProviderType::Concrete(service_type) => {
                match &provider.name {
                    Some(name) => {
                        quote! {
                            // Bind named concrete service with singleton scope by default
                            container.bind_named::<#service_type, #service_type>(#name);
                        }
                    },
                    None => {
                        quote! {
                            // Bind concrete service with singleton scope by default
                            container.bind::<#service_type, #service_type>();
                        }
                    }
                }
            },
            ProviderType::Trait(trait_type) => {
                if let Some(impl_type) = &provider.implementation {
                    match &provider.name {
                        Some(name) => {
                            // Generate a token type based on trait name
                            let _token_name = quote::format_ident!("{}Token", 
                                trait_type.to_token_stream().to_string().replace(" ", ""));
                            quote! {
                                // Bind trait implementation with token-based resolution (named)
                                // For now, we'll use direct concrete binding until token system is fully integrated
                                container.bind_named::<#impl_type, #impl_type>(#name);
                                
                                // TODO: Once token system is integrated:
                                // struct #_token_name;
                                // impl ServiceToken for #_token_name { type Service = dyn #trait_type; }
                                // container.bind_token_named::<#_token_name, #impl_type>(#name)?;
                            }
                        },
                        None => {
                            let _token_name = quote::format_ident!("{}Token", 
                                trait_type.to_token_stream().to_string().replace(" ", ""));
                            quote! {
                                // Bind trait implementation with token-based resolution
                                // For now, we'll use direct concrete binding until token system is fully integrated
                                container.bind::<#impl_type, #impl_type>();
                                
                                // TODO: Once token system is integrated:
                                // struct #_token_name;
                                // impl ServiceToken for #_token_name { type Service = dyn #trait_type; }
                                // container.bind_token::<#_token_name, #impl_type>()?;
                            }
                        }
                    }
                } else {
                    return Err(Error::new_spanned(
                        trait_type,
                        "Trait providers must specify implementation type: dyn Trait => Implementation"
                    ));
                }
            }
        };
        
        configure_calls.push(configure_call);
    }
    
    // Configure controllers with dependency injection
    for controller in &module_args.controllers {
        configure_calls.push(quote! {
            // Bind controller as singleton for injection
            container.bind::<#controller, #controller>();
        });
    }
    
    Ok(quote! {
        use elif_core::modules::{ModuleError, ModuleAutoConfiguration};
        use elif_core::container::ServiceBinder; // Import the binding trait
        
        // Build container if not already built to enable binding
        if !container.is_built() {
            // We need to defer building until all modules are configured
            // The container will be built by the application after all modules are registered
        }
        
        #(#configure_calls)*
        
        Ok(())
    })
}

/// Generate application composition code
fn generate_application_composition(
    composition_args: ModuleCompositionArgs,
) -> Result<proc_macro2::TokenStream> {
    let modules_descriptors = generate_modules_descriptors(&composition_args.modules)?;
    let overrides_descriptors = generate_composition_overrides(&composition_args.overrides)?;
    
    Ok(quote! {
        {
            use elif_core::modules::{ModuleComposition, ModuleDescriptor, ServiceDescriptor};
            
            let mut composition = ModuleComposition::new();
            
            // Add modules to composition
            #modules_descriptors
            
            // Add overrides
            #overrides_descriptors
            
            // Compose and return the final descriptor
            composition.compose().unwrap()
        }
    })
}

/// Generate module descriptors for application composition
fn generate_modules_descriptors(modules: &[Type]) -> Result<proc_macro2::TokenStream> {
    if modules.is_empty() {
        return Ok(quote! {
            // No modules specified
        });
    }
    
    let descriptor_calls: Vec<_> = modules.iter().map(|module| {
        quote! {
            composition = composition.with_module(#module::module_descriptor());
        }
    }).collect();
    
    Ok(quote! {
        #(#descriptor_calls)*
    })
}

/// Generate override descriptors for application composition
fn generate_composition_overrides(overrides: &[ProviderDef]) -> Result<proc_macro2::TokenStream> {
    if overrides.is_empty() {
        return Ok(quote! {
            // No overrides specified
        });
    }
    
    let mut override_descriptors = Vec::new();
    
    for override_def in overrides {
        let override_descriptor = match &override_def.service_type {
            ProviderType::Concrete(service_type) => {
                let service_name = quote! { stringify!(#service_type) }.to_string();
                match &override_def.name {
                    Some(name) => {
                        quote! {
                            ServiceDescriptor::new::<#service_type>(#service_name, ServiceLifecycle::default())
                                .with_name(#name)
                        }
                    },
                    None => {
                        quote! {
                            ServiceDescriptor::new::<#service_type>(#service_name, ServiceLifecycle::default())
                        }
                    }
                }
            },
            ProviderType::Trait(trait_type) => {
                if let Some(impl_type) = &override_def.implementation {
                    let service_name = quote! { stringify!(#trait_type) }.to_string();
                    let impl_name = quote! { stringify!(#impl_type) }.to_string();
                    match &override_def.name {
                        Some(name) => {
                            quote! {
                                ServiceDescriptor::trait_mapping::<#trait_type, #impl_type>(
                                    #service_name, #impl_name, ServiceLifecycle::default()
                                ).with_name(#name)
                            }
                        },
                        None => {
                            quote! {
                                ServiceDescriptor::trait_mapping::<#trait_type, #impl_type>(
                                    #service_name, #impl_name, ServiceLifecycle::default()
                                )
                            }
                        }
                    }
                } else {
                    return Err(Error::new_spanned(
                        trait_type,
                        "Trait overrides must specify implementation type: dyn Trait => Implementation"
                    ));
                }
            }
        };
        
        override_descriptors.push(override_descriptor);
    }
    
    Ok(quote! {
        use elif_core::modules::ServiceLifecycle;
        
        let overrides = vec![
            #(#override_descriptors),*
        ];
        composition = composition.with_overrides(overrides);
    })
}

/// Generate expanded code for demo DSL sugar syntax
/// Converts simplified syntax to full #[module(...)] form
fn generate_demo_dsl_expansion(demo_args: DemoDslArgs) -> Result<proc_macro2::TokenStream> {
    // Convert services to providers (concrete services)
    let providers: Vec<ProviderDef> = demo_args.services.into_iter().map(|service| {
        ProviderDef {
            service_type: ProviderType::Concrete(service),
            implementation: None,
            name: None,
        }
    }).collect();
    
    // Create a module descriptor with the expanded providers
    let module_args = ModuleArgs {
        providers,
        controllers: demo_args.controllers,
        imports: Vec::new(), // Demo DSL doesn't support imports yet
        exports: Vec::new(), // Demo DSL doesn't support exports yet
    };
    
    // Generate a temporary struct name for the module
    let struct_name = Ident::new("DemoDslModule", Span::call_site());
    
    let module_descriptor_impl = generate_module_descriptor_method(&struct_name, &module_args)?;
    
    // Generate middleware application code (simplified for demo)
    let middleware_code = if demo_args.middleware.is_empty() {
        quote! { /* No middleware specified */ }
    } else {
        let middleware_names = &demo_args.middleware;
        quote! {
            // Demo DSL middleware (simplified - would integrate with elif-http middleware system)
            let middleware_stack = vec![#(#middleware_names.to_string()),*];
            println!("Demo DSL: Would apply middleware: {:?}", middleware_stack);
        }
    };
    
    Ok(quote! {
        {
            // Generate a temporary module struct for the demo DSL
            struct #struct_name;
            
            #module_descriptor_impl
            
            // Create the module descriptor
            let descriptor = #struct_name::module_descriptor();
            
            // Apply middleware (demo implementation)
            #middleware_code
            
            // Return the descriptor for use in applications
            descriptor
        }
    })
}