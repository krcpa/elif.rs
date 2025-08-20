use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn, ReturnType};

/// Attribute macro for async main functions in elif.rs applications
/// 
/// # Examples
/// 
/// ```rust
/// use elif::prelude::*;
/// 
/// #[elif::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     Server::new()
///         .route("/", hello_handler)
///         .run().await?;
///     Ok(())
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
        // Function returns Result - handle errors
        quote! {
            fn main() -> Result<(), Box<dyn std::error::Error>> {
                // Initialize tokio runtime
                let rt = tokio::runtime::Runtime::new()?;
                
                // Initialize logging (if not already done)
                if std::env::var("RUST_LOG").is_err() {
                    std::env::set_var("RUST_LOG", "info");
                }
                env_logger::init();
                
                // Run the async function
                rt.block_on(async move {
                    async fn #fn_name(#fn_inputs) -> Result<(), Box<dyn std::error::Error + Send + Sync>> #fn_block
                    #fn_name().await
                })
            }
        }
    } else {
        // Function doesn't return Result
        quote! {
            fn main() {
                // Initialize tokio runtime
                let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
                
                // Initialize logging (if not already done)
                if std::env::var("RUST_LOG").is_err() {
                    std::env::set_var("RUST_LOG", "info");
                }
                env_logger::init();
                
                // Run the async function
                rt.block_on(async move {
                    async fn #fn_name(#fn_inputs) #fn_block
                    #fn_name().await
                });
            }
        }
    };
    
    TokenStream::from(expanded)
}