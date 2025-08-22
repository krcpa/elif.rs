//! # elif-core-derive
//! 
//! Derive macros for elif-core dependency injection system.
//! 
//! This crate provides procedural macros to simplify IoC container usage:
//! - `#[injectable]`: Automatically implement Injectable trait for structs

use proc_macro::TokenStream;

// Module declarations
mod injectable;

/// Injectable attribute macro for automatic dependency injection
#[proc_macro_attribute]
pub fn injectable(args: TokenStream, input: TokenStream) -> TokenStream {
    injectable::injectable_impl(args, input)
}