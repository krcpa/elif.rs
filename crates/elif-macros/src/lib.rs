//! Procedural macros for elif.rs framework
//!
//! This crate provides macros that simplify common patterns in elif.rs applications,
//! particularly around the bootstrap system and server startup.

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn, ReturnType};

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
    
    // Check if function returns Result
    let returns_result = matches!(input_fn.sig.output, ReturnType::Type(_, _));
    
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