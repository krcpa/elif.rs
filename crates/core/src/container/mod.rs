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
#[cfg(test)]
pub mod activation_strategy_test;
#[cfg(test)]
pub mod performance_test;
pub mod examples;
pub mod integration_test;
pub mod api_demo;
pub mod lifecycle;

#[cfg(test)]
pub mod simple_lifecycle_tests;
#[cfg(test)]
pub mod race_condition_tests;
#[cfg(test)]
pub mod scoped_singleton_conflict_test;
#[cfg(test)]
pub mod deadlock_prevention_test;

pub use container::Container;
pub use builder::ContainerBuilder;
pub use registry::{ServiceRegistry, ServiceEntry};
pub use scope::{ServiceScope, ServiceLifetime, ScopedServiceManager, ScopeId};
pub use descriptor::{ServiceDescriptor, ServiceId};
pub use resolver::{DependencyResolver as GraphDependencyResolver, DependencyGraph, ResolutionPath};
pub use binding::{ServiceBinder, ServiceBindings};
pub use ioc_container::IocContainer;
pub use ioc_builder::IocContainerBuilder;
pub use autowiring::{Injectable, DependencyResolver, ConstructorParameter, ParameterInfo, ConstructorInfo};
pub use lifecycle::{AsyncInitializable, Disposable, LifecycleManaged, ServiceState, ServiceLifecycleManager};