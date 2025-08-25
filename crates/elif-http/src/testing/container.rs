//! Test container setup utilities

use elif_core::container::IocContainer;
use std::sync::Arc;

/// Create a test container with proper setup for HTTP tests
pub fn create_test_container() -> Arc<IocContainer> {
    let mut container = IocContainer::new();
    container.build().expect("Test container build failed");
    Arc::new(container)
}

/// Create a test container with additional services registered
pub fn create_test_container_with_services() -> Arc<IocContainer> {
    let mut container = IocContainer::new();
    container.build().expect("Test container build failed");
    Arc::new(container)
}

/// Test container builder for more complex test scenarios
pub struct TestContainerBuilder {
    container: IocContainer,
}

impl TestContainerBuilder {
    pub fn new() -> Self {
        Self {
            container: IocContainer::new(),
        }
    }

    pub fn build(mut self) -> Arc<IocContainer> {
        self.container.build().expect("Test container build failed");
        Arc::new(self.container)
    }
}

impl Default for TestContainerBuilder {
    fn default() -> Self {
        Self::new()
    }
}
