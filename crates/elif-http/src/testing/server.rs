//! Test server utilities

use crate::{Server, HttpConfig, ElifRouter};
use crate::testing::container::create_test_container;
use std::net::SocketAddr;

/// Test server builder for integration tests
pub struct TestServerBuilder {
    config: HttpConfig,
    router: Option<ElifRouter>,
}

impl TestServerBuilder {
    pub fn new() -> Self {
        Self {
            config: HttpConfig::default(),
            router: None,
        }
    }

    pub fn with_config(mut self, config: HttpConfig) -> Self {
        self.config = config;
        self
    }

    pub fn with_router(mut self, router: ElifRouter) -> Self {
        self.router = Some(router);
        self
    }

    pub fn build(self) -> Server {
        let container = create_test_container();
        let mut server = Server::with_container(container, self.config)
            .expect("Failed to create test server");
        
        if let Some(router) = self.router {
            server.use_router(router);
        }

        server
    }
}

impl Default for TestServerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Get a free port for testing
pub fn get_test_port() -> u16 {
    // In a real implementation, this would find an available port
    // For now, return a commonly available test port
    3001
}

/// Create a test socket address
pub fn test_socket_addr() -> SocketAddr {
    format!("127.0.0.1:{}", get_test_port()).parse().unwrap()
}