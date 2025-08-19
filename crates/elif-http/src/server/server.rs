//! # Elif HTTP Server
//! 
//! A NestJS-like HTTP server that provides a clean, intuitive API while using Axum under the hood.
//! Users interact only with framework types - Axum is completely abstracted away.

use crate::{
    config::HttpConfig, 
    errors::{HttpError, HttpResult},
    routing::ElifRouter, 
    middleware::v2::{MiddlewarePipelineV2, Middleware},
};
use super::lifecycle::{build_internal_router, start_server};
use elif_core::Container;
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::info;

/// The main HTTP server - NestJS-like experience
/// 
/// # Example
/// 
/// ```rust,no_run
/// use elif_http::{Server, HttpConfig};
/// use elif_core::Container;
/// use std::sync::Arc;
/// 
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let container = Container::new();
///     let server = Server::new(container, HttpConfig::default())?;
///     server.listen("0.0.0.0:3000").await?;
///     Ok(())
/// }
/// ```
pub struct Server {
    container: Arc<Container>,
    config: HttpConfig,
    router: Option<ElifRouter>,
    middleware: MiddlewarePipelineV2,
}

impl Server {
    /// Create a new server instance
    pub fn new(container: Container, config: HttpConfig) -> HttpResult<Self> {
        Ok(Self {
            container: Arc::new(container),
            config,
            router: None,
            middleware: MiddlewarePipelineV2::new(),
        })
    }

    /// Create a new server with existing Arc<Container>
    pub fn with_container(container: Arc<Container>, config: HttpConfig) -> HttpResult<Self> {
        Ok(Self {
            container,
            config,
            router: None,
            middleware: MiddlewarePipelineV2::new(),
        })
    }

    /// Set custom routes using framework router
    /// 
    /// # Example
    /// 
    /// ```rust,no_run
    /// use elif_http::{Server, ElifRouter, HttpConfig, ElifRequest, HttpResult};
    /// use elif_core::Container;
    /// use std::sync::Arc;
    /// 
    /// # async fn get_users(_req: ElifRequest) -> HttpResult<&'static str> { Ok("users") }
    /// # async fn create_user(_req: ElifRequest) -> HttpResult<&'static str> { Ok("created") }
    /// 
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let container = Container::new();
    /// let mut server = Server::new(container, HttpConfig::default())?;
    /// 
    /// let router = ElifRouter::new()
    ///     .get("/users", get_users)
    ///     .post("/users", create_user);
    /// 
    /// server.use_router(router);
    /// # Ok(())
    /// # }
    /// ```
    pub fn use_router(&mut self, router: ElifRouter) -> &mut Self {
        self.router = Some(router);
        self
    }

    /// Add middleware to the server
    /// 
    /// # Example
    /// 
    /// ```rust,no_run
    /// use elif_http::{Server, HttpConfig};
    /// use elif_core::Container;
    /// use std::sync::Arc;
    /// 
    /// # struct LoggingMiddleware;
    /// # impl LoggingMiddleware { 
    /// #     fn default() -> Self { LoggingMiddleware } 
    /// # }
    /// # impl elif_http::Middleware for LoggingMiddleware {}
    /// 
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let container = Container::new();
    /// let mut server = Server::new(container, HttpConfig::default())?;
    /// server.use_middleware(LoggingMiddleware::default());
    /// # Ok(())
    /// # }
    /// ```
    pub fn use_middleware<M>(&mut self, middleware: M) -> &mut Self 
    where 
        M: Middleware + 'static,
    {
        self.middleware.add_mut(middleware);
        self
    }

    /// Start the server on the specified address
    /// 
    /// # Example
    /// 
    /// ```rust,no_run
    /// # use elif_http::{Server, HttpConfig};
    /// # use elif_core::Container;
    /// # use std::sync::Arc;
    /// # 
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #     let container = Container::new();
    /// #     let server = Server::new(container, HttpConfig::default())?;
    /// server.listen("0.0.0.0:3000").await?;
    /// #     Ok(())
    /// # }
    /// ```
    pub async fn listen<A: Into<String>>(self, addr: A) -> HttpResult<()> {
        let addr_str = addr.into();
        let socket_addr: SocketAddr = addr_str.parse()
            .map_err(|e| HttpError::config(format!("Invalid address '{}': {}", addr_str, e)))?;

        self.listen_on(socket_addr).await
    }

    /// Start the server on the specified SocketAddr
    pub async fn listen_on(self, addr: SocketAddr) -> HttpResult<()> {
        info!("🚀 Starting Elif server on {}", addr);
        info!("📋 Health check endpoint: {}", self.config.health_check_path);
        
        // Build the internal router
        let axum_router = build_internal_router(self.container.clone(), self.config.clone(), self.router, self.middleware).await?;
        
        // Start the server
        start_server(addr, axum_router).await?;

        info!("🛑 Server shut down gracefully");
        Ok(())
    }

    // Getter methods for testing and inspection
    pub fn container(&self) -> &Arc<Container> {
        &self.container
    }

    pub fn config(&self) -> &HttpConfig {
        &self.config
    }

    pub fn router(&self) -> Option<&ElifRouter> {
        self.router.as_ref()
    }

    pub fn middleware(&self) -> &MiddlewarePipelineV2 {
        &self.middleware
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::create_test_container;

    #[test]
    fn test_server_creation() {
        let container = create_test_container();
        let config = HttpConfig::default();
        
        let server = Server::with_container(container, config);
        assert!(server.is_ok());
    }

    #[test]
    fn test_server_with_arc_container() {
        let container = create_test_container();
        let config = HttpConfig::default();
        
        let server = Server::with_container(container, config);
        assert!(server.is_ok());
    }

    #[test]
    fn test_server_configuration() {
        let container = create_test_container();
        let config = HttpConfig::default();
        let server = Server::with_container(container, config).unwrap();
        
        assert_eq!(server.config().health_check_path, "/health");
    }
}