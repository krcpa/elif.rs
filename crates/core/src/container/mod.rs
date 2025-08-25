#[cfg(test)]
pub mod activation_strategy_test;
pub mod advanced_binding_examples;
pub mod api_demo;
pub mod autowiring;
pub mod autowiring_example;
pub mod binding;
pub mod builder;
#[allow(clippy::module_inception)]
pub mod container;
pub mod conventions;
pub mod debug;
pub mod descriptor;
pub mod examples;
pub mod integration_test;
pub mod ioc_builder;
pub mod ioc_container;
pub mod lifecycle;
pub mod module;
#[cfg(test)]
pub mod performance_test;
pub mod phase5_demo;
pub mod phase6_demo;
pub mod registry;
pub mod resolver;
pub mod scope;
pub mod tokens;
pub mod validation;
pub mod visualization;

#[cfg(test)]
pub mod advanced_binding_test;
#[cfg(test)]
pub mod deadlock_prevention_test;
#[cfg(test)]
pub mod ioc_integration_tests;
#[cfg(test)]
pub mod race_condition_tests;
#[cfg(test)]
pub mod scoped_singleton_conflict_test;
#[cfg(test)]
pub mod simple_lifecycle_tests;

// Legacy container exports (deprecated in favor of IoC container)
pub use autowiring::{
    ConstructorInfo, ConstructorParameter, DependencyResolver, Injectable, ParameterInfo,
};
pub use binding::{
    AdvancedBindingBuilder, BindingConfig, CollectionBindingBuilder, ConditionFn, EnvCondition,
    ServiceBinder, ServiceBindings,
};
#[deprecated(since = "0.6.0", note = "Use IocContainerBuilder instead")]
pub use builder::ContainerBuilder;
#[deprecated(since = "0.6.0", note = "Use IocContainer instead")]
pub use container::Container;
pub use conventions::{
    AutoDiscoverable, ConventionRule, ServiceAttribute, ServiceConventions, ServiceMetadata,
    ServiceRegistry as ConventionServiceRegistry,
};
pub use debug::{
    ContainerHealthChecker, ContainerInspector, HealthCheck, HealthReport, HealthStatus,
    PerformanceProfiler, ResolutionTracer,
};
pub use descriptor::{ServiceDescriptor, ServiceId};
pub use ioc_builder::IocContainerBuilder;
pub use ioc_container::{IocContainer, ServiceStatistics};
pub use lifecycle::{
    AsyncInitializable, Disposable, LifecycleManaged, ServiceLifecycleManager, ServiceState,
};
pub use module::{
    LoadedModule, ModularContainerBuilder, ModuleConfig, ModuleId, ModuleMetadata, ModuleRegistry,
    ModuleState, ServiceModule,
};
pub use registry::{ServiceEntry, ServiceRegistry};
pub use resolver::{
    DependencyGraph, DependencyResolver as GraphDependencyResolver, ResolutionPath,
};
pub use scope::{ScopeId, ScopedServiceManager, ServiceLifetime, ServiceScope};
pub use tokens::{
    ServiceToken, TokenBinding, TokenInfo, TokenReference, TokenRegistry, TokenRegistryStats,
};
pub use validation::{
    ContainerValidator, DependencyValidator, ValidationError, ValidationReport, ValidationWarning,
};
pub use visualization::{
    DependencyVisualizer, ServiceExplorer, VisualizationFormat, VisualizationStyle,
};
