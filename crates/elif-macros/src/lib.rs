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
    app_module: Type,
    address: Option<String>,
    config: Option<Expr>,
    middleware: Option<Vec<Expr>>,
}

impl Parse for BootstrapArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        // First argument is required: the app module type
        let app_module: Type = input.parse()?;
        
        let mut address: Option<String> = None;
        let mut config: Option<Expr> = None;
        let mut middleware: Option<Vec<Expr>> = None;
        
        // Parse optional named arguments
        while !input.is_empty() {
            let _comma: Token![,] = input.parse()?;
            
            if input.is_empty() {
                break;
            }
            
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
                                • #[elif::bootstrap(AppModule)]\n\
                                • #[elif::bootstrap(AppModule, addr = \"127.0.0.1:3000\")]\n\
                                • #[elif::bootstrap(AppModule, config = my_config())]\n\
                                • #[elif::bootstrap(AppModule, middleware = [cors(), auth()])]",
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
/// generating all the server setup code based on your app module.
/// 
/// # Examples
/// 
/// ## Basic Bootstrap
/// ```rust
/// use elif::prelude::*;
/// 
/// #[elif::bootstrap(AppModule)]
/// async fn main() -> Result<(), HttpError> {
///     // Automatically generated:
///     // - Module discovery from compile-time registry
///     // - DI container configuration
///     // - Router setup with all controllers
///     // - Server startup on 127.0.0.1:3000
/// }
/// ```
/// 
/// ## With Custom Address
/// ```rust
/// #[elif::bootstrap(AppModule, addr = "0.0.0.0:8080")]
/// async fn main() -> Result<(), HttpError> {}
/// ```
/// 
/// ## With Custom Configuration
/// ```rust
/// #[elif::bootstrap(AppModule, config = HttpConfig::with_timeout(30))]
/// async fn main() -> Result<(), HttpError> {}
/// ```
/// 
/// ## With Middleware
/// ```rust
/// #[elif::bootstrap(AppModule, middleware = [cors(), auth(), logging()])]
/// async fn main() -> Result<(), HttpError> {}
/// ```
/// 
/// ## Full Configuration
/// ```rust
/// #[elif::bootstrap(
///     AppModule, 
///     addr = "0.0.0.0:8080",
///     config = HttpConfig::production(),
///     middleware = [cors(), auth()]
/// )]
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
    let app_module = &bootstrap_args.app_module;
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
                // Generate bootstrap code
                let bootstrapper = #app_module::bootstrap()
                    .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?
                    #config_setup
                    #middleware_setup;
                
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