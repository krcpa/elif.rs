//! # elif-http-derive
//! 
//! Derive macros for elif-http declarative routing and controller system.
//! 
//! This crate provides procedural macros to simplify controller development:
//! - `#[controller]`: Define controller base path and metadata
//! - `#[get]`, `#[post]`, etc.: HTTP method routing macros
//! - `#[middleware]`: Apply middleware to controllers and methods
//! - `#[param]`: Route parameter specifications
//! - `#[body]`: Request body type specifications
//! - `#[request]`: Automatic ElifRequest parameter injection
//! - `#[routes]`: Generate route registration code from impl blocks
//! - `#[resource]`: Automatic RESTful resource registration
//! - `#[group]`: Route grouping with shared attributes

use proc_macro::TokenStream;

// Module declarations
mod controller;
mod http_methods;
mod inject;
mod middleware;
mod params;
mod routes;
mod groups;
mod utils;

#[cfg(test)]
mod test;

/// Controller macro for defining controller base path and metadata
#[proc_macro_attribute]
pub fn controller(args: TokenStream, input: TokenStream) -> TokenStream {
    controller::controller_impl(args, input)
}

/// GET method routing macro
#[proc_macro_attribute]
pub fn get(args: TokenStream, input: TokenStream) -> TokenStream {
    http_methods::get_impl(args, input)
}

/// POST method routing macro
#[proc_macro_attribute]
pub fn post(args: TokenStream, input: TokenStream) -> TokenStream {
    http_methods::post_impl(args, input)
}

/// PUT method routing macro
#[proc_macro_attribute]
pub fn put(args: TokenStream, input: TokenStream) -> TokenStream {
    http_methods::put_impl(args, input)
}

/// DELETE method routing macro
#[proc_macro_attribute]
pub fn delete(args: TokenStream, input: TokenStream) -> TokenStream {
    http_methods::delete_impl(args, input)
}

/// PATCH method routing macro
#[proc_macro_attribute]
pub fn patch(args: TokenStream, input: TokenStream) -> TokenStream {
    http_methods::patch_impl(args, input)
}

/// HEAD method routing macro
#[proc_macro_attribute]
pub fn head(args: TokenStream, input: TokenStream) -> TokenStream {
    http_methods::head_impl(args, input)
}

/// OPTIONS method routing macro
#[proc_macro_attribute]
pub fn options(args: TokenStream, input: TokenStream) -> TokenStream {
    http_methods::options_impl(args, input)
}

/// Middleware application macro
#[proc_macro_attribute]
pub fn middleware(args: TokenStream, input: TokenStream) -> TokenStream {
    middleware::middleware_impl(args, input)
}

/// Route parameter specification macro
#[proc_macro_attribute]
pub fn param(args: TokenStream, input: TokenStream) -> TokenStream {
    params::param_impl(args, input)
}

/// Request body specification macro
#[proc_macro_attribute]
pub fn body(args: TokenStream, input: TokenStream) -> TokenStream {
    params::body_impl(args, input)
}

/// Request injection macro for automatic ElifRequest parameter injection
#[proc_macro_attribute]
pub fn request(args: TokenStream, input: TokenStream) -> TokenStream {
    params::request_impl(args, input)
}

/// Route registration macro for impl blocks
#[proc_macro_attribute]
pub fn routes(args: TokenStream, input: TokenStream) -> TokenStream {
    routes::routes_impl(args, input)
}

/// Resource macro for automatic RESTful resource registration
#[proc_macro_attribute]
pub fn resource(args: TokenStream, input: TokenStream) -> TokenStream {
    routes::resource_impl(args, input)
}

/// Route group macro for grouping routes with shared attributes
#[proc_macro_attribute]
pub fn group(args: TokenStream, input: TokenStream) -> TokenStream {
    groups::group_impl(args, input)
}

/// Service injection macro for declarative dependency injection
#[proc_macro_attribute]
pub fn inject(args: TokenStream, input: TokenStream) -> TokenStream {
    inject::inject_impl(args, input)
}