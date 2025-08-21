#[allow(clippy::module_inception)]
pub mod container;
pub mod builder;
pub mod registry;
pub mod scope;
pub mod descriptor;
pub mod resolver;
pub mod binding;
pub mod ioc_container;
pub mod ioc_builder;
pub mod autowiring;
pub mod autowiring_example;
pub mod examples;
pub mod integration_test;
pub mod api_demo;

pub use container::Container;
pub use builder::ContainerBuilder;
pub use registry::{ServiceRegistry, ServiceEntry};
pub use scope::{ServiceScope, ScopedServiceManager};
pub use descriptor::{ServiceDescriptor, ServiceId};
pub use resolver::{DependencyResolver as GraphDependencyResolver, DependencyGraph, ResolutionPath};
pub use binding::{ServiceBinder, ServiceBindings};
pub use ioc_container::IocContainer;
pub use ioc_builder::IocContainerBuilder;
pub use autowiring::{Injectable, DependencyResolver, ConstructorParameter, ParameterInfo, ConstructorInfo};