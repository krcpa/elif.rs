//! Procedural macros for elif.rs framework
//!
//! This crate provides macros that simplify common patterns in elif.rs applications,
//! particularly around the bootstrap system and server startup.

use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream, Result},
    parse_macro_input, 
    punctuated::Punctuated,
    token::Comma,
    Error, Expr, ItemFn, LitStr, ReturnType, Token, Type,
};

/// Attribute macro for async main functions in elif.rs applications
/// 
/// This macro simplifies the bootstrap process by handling tokio runtime setup,
/// logging initialization, and proper error conversion between bootstrap and HTTP errors.
/// 
/// # Examples
/// 
/// ## Basic Bootstrap Usage
/// ```rust
/// use elif::prelude::*;
/// 
/// #[elif::main]
/// async fn main() -> Result<(), HttpError> {
///     AppModule::bootstrap().listen("127.0.0.1:3000").await
/// }
/// ```
/// 
/// ## With Manual Server Setup
/// ```rust
/// use elif::prelude::*;
/// 
/// #[elif::main]
/// async fn main() -> Result<(), HttpError> {
///     let server = Server::new();
///     server.listen("127.0.0.1:3000").await
/// }
/// ```
#[proc_macro_attribute]
pub fn main(_args: TokenStream, input: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(input as ItemFn);
    let fn_name = &input_fn.sig.ident;
    let fn_block = &input_fn.block;
    let fn_inputs = &input_fn.sig.inputs;
    
    // Check if function returns Result (more precise detection)
    let returns_result = if let ReturnType::Type(_, ty) = &input_fn.sig.output {
        if let syn::Type::Path(type_path) = &**ty {
            type_path.path.segments.last().is_some_and(|s| s.ident == "Result")
        } else {
            false
        }
    } else {
        false
    };
    
    let expanded = if returns_result {
        // Function returns Result - handle bootstrap/HTTP errors properly
        quote! {
            #[tokio::main]
            async fn main() -> Result<(), Box<dyn std::error::Error>> {
                // Initialize logging (if not already done)
                if std::env::var("RUST_LOG").is_err() {
                    std::env::set_var("RUST_LOG", "info");
                }
                
                // Try to initialize logger, but don't fail if already initialized
                let _ = env_logger::try_init();
                
                // Define the original async function inline
                async fn #fn_name(#fn_inputs) -> Result<(), Box<dyn std::error::Error + Send + Sync>> #fn_block
                
                // Run it and handle errors
                match #fn_name().await {
                    Ok(()) => Ok(()),
                    Err(e) => {
                        eprintln!("Application failed: {}", e);
                        Err(e)
                    }
                }
            }
        }
    } else {
        // Function doesn't return Result
        quote! {
            #[tokio::main]
            async fn main() {
                // Initialize logging (if not already done)
                if std::env::var("RUST_LOG").is_err() {
                    std::env::set_var("RUST_LOG", "info");
                }
                
                // Try to initialize logger, but don't fail if already initialized
                let _ = env_logger::try_init();
                
                // Define the original async function inline
                async fn #fn_name(#fn_inputs) #fn_block
                
                // Run it
                #fn_name().await;
            }
        }
    };
    
    TokenStream::from(expanded)
}

/// Bootstrap arguments for the bootstrap macro
struct BootstrapArgs {
    app_module: Option<Type>,
    address: Option<String>,
    config: Option<Expr>,
    middleware: Option<Vec<Expr>>,
}

impl Parse for BootstrapArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut app_module: Option<Type> = None;
        let mut address: Option<String> = None;
        let mut config: Option<Expr> = None;
        let mut middleware: Option<Vec<Expr>> = None;

        // If input is empty, use auto-discovery mode
        if input.is_empty() {
            return Ok(BootstrapArgs {
                app_module: None,
                address,
                config,
                middleware,
            });
        }

        // Try to parse first argument as app module type (for backward compatibility)
        let lookahead = input.lookahead1();
        if lookahead.peek(syn::Ident) {
            // Check if first token is a named parameter or a type
            let fork = input.fork();
            let _ident: syn::Ident = fork.parse().unwrap();
            if fork.peek(Token![=]) {
                // This is a named parameter, not a type - use auto-discovery
                app_module = None;
                // Don't consume the token here, let the main parsing loop handle it
            } else {
                // This looks like a type - parse it for backward compatibility
                app_module = Some(input.parse()?);
            }
        } else if lookahead.peek(syn::Token![::]) || lookahead.peek(syn::Token![<]) {
            // This is definitely a type
            app_module = Some(input.parse()?);
        } else if lookahead.peek(LitStr) {
            // Legacy support: first argument is address string
            let lit: LitStr = input.parse()?;
            address = Some(lit.value());
        }
        
        // Parse optional named arguments
        let mut first_param = true;
        while !input.is_empty() {
            // Only expect comma if this is not the first parameter or if we parsed a type
            if !first_param || app_module.is_some() {
                let _comma: Token![,] = input.parse()?;
                
                if input.is_empty() {
                    break;
                }
            }
            first_param = false;
            
            let lookahead = input.lookahead1();
            if lookahead.peek(syn::Ident) {
                let ident: syn::Ident = input.parse()?;
                let _eq: Token![=] = input.parse()?;
                
                match ident.to_string().as_str() {
                    "addr" => {
                        let lit: LitStr = input.parse()?;
                        address = Some(lit.value());
                    }
                    "config" => {
                        config = Some(input.parse()?);
                    }
                    "middleware" => {
                        let content;
                        syn::bracketed!(content in input);
                        let middleware_list: Punctuated<Expr, Comma> = 
                            content.parse_terminated(Expr::parse, Comma)?;
                        middleware = Some(middleware_list.into_iter().collect());
                    }
                    _ => {
                        let ident_name = ident.to_string();
                        return Err(Error::new_spanned(
                            ident,
                            format!(
                                "Unknown bootstrap parameter '{}'. Valid parameters are: addr, config, middleware\n\
                                \n\
                                💡 Usage examples:\n\
                                • #[elif::bootstrap] (auto-discovery)\n\
                                • #[elif::bootstrap(addr = \"127.0.0.1:3000\")]\n\
                                • #[elif::bootstrap(config = my_config())]\n\
                                • #[elif::bootstrap(middleware = [cors(), auth()])]\n\
                                • #[elif::bootstrap(AppModule)] (backward compatibility)",
                                ident_name
                            )
                        ));
                    }
                }
            } else if input.peek(LitStr) {
                // Simple string for address (legacy support)
                let lit: LitStr = input.parse()?;
                address = Some(lit.value());
            } else {
                return Err(lookahead.error());
            }
        }
        
        Ok(BootstrapArgs {
            app_module,
            address,
            config,
            middleware,
        })
    }
}

/// Enhanced bootstrap macro for zero-boilerplate application startup
/// 
/// This macro provides Laravel-style "convention over configuration" by automatically
/// generating all the server setup code using auto-discovery of modules and controllers.
/// 
/// # Examples
/// 
/// ## Zero-Boilerplate Bootstrap (NEW!)
/// ```rust
/// use elif::prelude::*;
/// 
/// #[elif::bootstrap]
/// async fn main() -> Result<(), HttpError> {
///     // Automatically generated:
///     // - Module discovery from compile-time registry
///     // - Controller auto-registration from static registry
///     // - DI container configuration
///     // - Router setup with all controllers
///     // - Server startup on 127.0.0.1:3000
/// }
/// ```
/// 
/// ## With Custom Address
/// ```rust
/// #[elif::bootstrap(addr = "0.0.0.0:8080")]
/// async fn main() -> Result<(), HttpError> {}
/// ```
/// 
/// ## With Custom Configuration
/// ```rust
/// #[elif::bootstrap(config = HttpConfig::with_timeout(30))]
/// async fn main() -> Result<(), HttpError> {}
/// ```
/// 
/// ## With Middleware
/// ```rust
/// #[elif::bootstrap(middleware = [cors(), auth(), logging()])]
/// async fn main() -> Result<(), HttpError> {}
/// ```
/// 
/// ## Full Configuration
/// ```rust
/// #[elif::bootstrap(
///     addr = "0.0.0.0:8080",
///     config = HttpConfig::production(),
///     middleware = [cors(), auth()]
/// )]
/// async fn main() -> Result<(), HttpError> {}
/// ```
/// 
/// ## Backward Compatibility with AppModule
/// ```rust
/// #[elif::bootstrap(AppModule)]
/// async fn main() -> Result<(), HttpError> {}
/// ```
#[proc_macro_attribute]
pub fn bootstrap(args: TokenStream, input: TokenStream) -> TokenStream {
    let bootstrap_args = match syn::parse::<BootstrapArgs>(args) {
        Ok(args) => args,
        Err(err) => return err.to_compile_error().into(),
    };

    let input_fn = parse_macro_input!(input as ItemFn);
    let fn_name = &input_fn.sig.ident;
    let fn_inputs = &input_fn.sig.inputs;
    
    // Validate that this is an async function
    if input_fn.sig.asyncness.is_none() {
        let error = Error::new_spanned(
            input_fn.sig.fn_token,
            "Bootstrap macro can only be applied to async functions\n\
            \n\
            💡 Change your function to:\n\
            async fn main() -> Result<(), HttpError> {}"
        );
        return error.to_compile_error().into();
    }
    
    // Check if function returns Result
    let returns_result = if let ReturnType::Type(_, ty) = &input_fn.sig.output {
        if let syn::Type::Path(type_path) = &**ty {
            type_path.path.segments.last().is_some_and(|s| s.ident == "Result")
        } else {
            false
        }
    } else {
        false
    };
    
    if !returns_result {
        let error = Error::new_spanned(
            &input_fn.sig.output,
            "Bootstrap macro requires functions to return Result<(), HttpError>\n\
            \n\
            💡 Change your function signature to:\n\
            async fn main() -> Result<(), HttpError> {}"
        );
        return error.to_compile_error().into();
    }
    
    // Generate bootstrap code
    let address = bootstrap_args.address.as_deref().unwrap_or("127.0.0.1:3000");
    let config_setup = if let Some(config) = &bootstrap_args.config {
        quote! { .with_config(#config) }
    } else {
        quote! {}
    };
    let middleware_setup = if let Some(middleware) = &bootstrap_args.middleware {
        quote! { .with_middleware(vec![#(Box::new(#middleware)),*]) }
    } else {
        quote! {}
    };
    
    // Generate different bootstrap code based on whether app_module is provided
    let bootstrap_code = if let Some(app_module) = &bootstrap_args.app_module {
        // Backward compatibility: use AppModule::bootstrap()
        quote! {
            let bootstrapper = #app_module::bootstrap()
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?
                #config_setup
                #middleware_setup;
        }
    } else {
        // New auto-discovery mode: use AppBootstrapper directly
        quote! {
            let bootstrapper = elif_http::bootstrap::AppBootstrapper::new()
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?
                #config_setup
                #middleware_setup;
        }
    };
    
    let expanded = quote! {
        #[tokio::main]
        async fn main() -> Result<(), Box<dyn std::error::Error>> {
            // Initialize logging (if not already done)
            if std::env::var("RUST_LOG").is_err() {
                std::env::set_var("RUST_LOG", "info");
            }
            
            // Try to initialize logger, but don't fail if already initialized
            let _ = env_logger::try_init();
            
            // Define the original async function inline for any custom setup
            async fn #fn_name(#fn_inputs) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
                // Generate bootstrap code (auto-discovery or backward compatibility)
                #bootstrap_code
                
                // Start the server
                bootstrapper
                    .listen(#address.parse().expect("Invalid socket address"))
                    .await
                    .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
                
                Ok(())
            }
            
            // Run it and handle errors
            match #fn_name().await {
                Ok(()) => Ok(()),
                Err(e) => {
                    eprintln!("Application bootstrap failed: {}", e);
                    Err(e)
                }
            }
        }
    };
    
    TokenStream::from(expanded)
}