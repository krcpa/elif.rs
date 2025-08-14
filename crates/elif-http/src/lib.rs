//! # elif-http
//! 
//! HTTP server core for the elif.rs LLM-friendly web framework.
//! 
//! This crate provides the fundamental HTTP server functionality including:
//! - Axum-based HTTP server with async support
//! - Integration with the elif-core DI container
//! - Configuration management
//! - Graceful shutdown handling
//! - Health check endpoints

// pub mod server;
// pub mod simple_server;
pub mod minimal_server;
pub mod config;
pub mod error;
pub mod tests;

// pub use server::{HttpServer, HttpServerBuilder};
// pub use simple_server::SimpleHttpServer;
pub use minimal_server::MinimalHttpServer;
pub use config::HttpConfig;
pub use error::{HttpError, HttpResult};

// Re-export commonly used types
pub use axum::{
    Router, 
    response::{Json, Response},
    extract::{State, Query, Path},
    http::{StatusCode, HeaderMap},
};