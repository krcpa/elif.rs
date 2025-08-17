//! Test container setup utilities

use elif_core::Container;
use std::sync::Arc;

/// Create a test container with proper setup for HTTP tests
pub fn create_test_container() -> Arc<Container> {
    let container = Container::new();
    Arc::new(container)
}

/// Create a test container with additional services registered
pub fn create_test_container_with_services() -> Arc<Container> {
    let container = Container::new();
    Arc::new(container)
}

/// Test container builder for more complex test scenarios
pub struct TestContainerBuilder {
    container: Container,
}

impl TestContainerBuilder {
    pub fn new() -> Self {
        Self {
            container: Container::new(),
        }
    }

    pub fn build(self) -> Arc<Container> {
        Arc::new(self.container)
    }
}

impl Default for TestContainerBuilder {
    fn default() -> Self {
        Self::new()
    }
}