use std::any::Any;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::container::binding::{ServiceBinder, ServiceBindings};
use crate::container::descriptor::ServiceId;
use crate::container::resolver::DependencyResolver as GraphDependencyResolver;
use crate::container::autowiring::{DependencyResolver, Injectable};
use crate::container::scope::{ServiceScope, ScopedServiceManager, ScopeId};
use crate::container::lifecycle::ServiceLifecycleManager;
use crate::container::tokens::{ServiceToken, TokenRegistry};
use crate::errors::CoreError;

/// Service instance storage
#[derive(Debug)]
enum ServiceInstance {
    /// Singleton instance
    Singleton(Arc<dyn Any + Send + Sync>),
    /// Scoped instances by scope ID
    Scoped(HashMap<ScopeId, Arc<dyn Any + Send + Sync>>),
}

/// Modern IoC container with proper dependency injection
#[derive(Debug)]
pub struct IocContainer {
    /// Service bindings and descriptors
    bindings: ServiceBindings,
    /// Token-based service registry
    tokens: TokenRegistry,
    /// Dependency resolver
    resolver: Option<GraphDependencyResolver>,
    /// Instantiated services
    instances: Arc<RwLock<HashMap<ServiceId, ServiceInstance>>>,
    /// Service lifecycle manager
    lifecycle_manager: ServiceLifecycleManager,
    /// Active scopes
    scopes: Arc<RwLock<HashMap<ScopeId, Arc<ScopedServiceManager>>>>,
    /// Whether the container is built and ready
    is_built: bool,
}

impl IocContainer {
    /// Create a new IoC container
    pub fn new() -> Self {
        Self {
            bindings: ServiceBindings::new(),
            tokens: TokenRegistry::new(),
            resolver: None,
            instances: Arc::new(RwLock::new(HashMap::new())),
            lifecycle_manager: ServiceLifecycleManager::new(),
            scopes: Arc::new(RwLock::new(HashMap::new())),
            is_built: false,
        }
    }
    
    /// Create IoC container from existing bindings
    pub fn from_bindings(bindings: ServiceBindings) -> Self {
        Self {
            bindings,
            tokens: TokenRegistry::new(),
            resolver: None,
            instances: Arc::new(RwLock::new(HashMap::new())),
            lifecycle_manager: ServiceLifecycleManager::new(),
            scopes: Arc::new(RwLock::new(HashMap::new())),
            is_built: false,
        }
    }
    
    /// Build the container and prepare for service resolution
    pub fn build(&mut self) -> Result<(), CoreError> {
        if self.is_built {
            return Ok(());
        }
        
        // Build dependency resolver
        let resolver = GraphDependencyResolver::new(self.bindings.descriptors())?;
        self.resolver = Some(resolver);
        
        // Validate dependencies
        let service_ids = self.bindings.service_ids().into_iter().collect();
        if let Some(resolver) = &self.resolver {
            resolver.validate_dependencies(&service_ids)?;
        }
        
        self.is_built = true;
        Ok(())
    }
    
    /// Initialize all async services
    pub async fn initialize_async(&mut self) -> Result<(), CoreError> {
        self.lifecycle_manager.initialize_all().await
    }
    
    /// Initialize all async services with timeout
    pub async fn initialize_async_with_timeout(
        &mut self, 
        timeout: std::time::Duration
    ) -> Result<(), CoreError> {
        self.lifecycle_manager.initialize_all_with_timeout(timeout).await
    }
    
    /// Create a new service scope
    pub fn create_scope(&self) -> Result<ScopeId, CoreError> {
        let scope_manager = Arc::new(ScopedServiceManager::new());
        let scope_id = scope_manager.scope_id().clone();
        
        let mut scopes = self.scopes.write().map_err(|_| CoreError::LockError {
            resource: "scopes".to_string(),
        })?;
        
        scopes.insert(scope_id.clone(), scope_manager);
        Ok(scope_id)
    }
    
    /// Create a child scope from an existing scope
    pub fn create_child_scope(&self, parent_scope_id: &ScopeId) -> Result<ScopeId, CoreError> {
        let mut scopes = self.scopes.write().map_err(|_| CoreError::LockError {
            resource: "scopes".to_string(),
        })?;
        
        let parent_scope = scopes.get(parent_scope_id)
            .ok_or_else(|| CoreError::ServiceNotFound {
                service_type: format!("parent scope {}", parent_scope_id),
            })?
            .clone(); // Clone the Arc, not the ScopedServiceManager
        
        let child_scope = Arc::new(ScopedServiceManager::create_child(parent_scope));
        let child_scope_id = child_scope.scope_id().clone();
        
        scopes.insert(child_scope_id.clone(), child_scope);
        Ok(child_scope_id)
    }
    
    /// Dispose of a scope and all its services
    pub async fn dispose_scope(&self, scope_id: &ScopeId) -> Result<(), CoreError> {
        let was_removed = {
            let mut scopes = self.scopes.write().map_err(|_| CoreError::LockError {
                resource: "scopes".to_string(),
            })?;
            scopes.remove(scope_id).is_some()
        };

        if was_removed {
            // Remove scoped instances for this scope
            let mut instances = self.instances.write().map_err(|_| CoreError::LockError {
                resource: "service_instances".to_string(),
            })?;

            for (_, instance) in instances.iter_mut() {
                if let ServiceInstance::Scoped(scoped_instances) = instance {
                    scoped_instances.remove(scope_id);
                }
            }
        }

        Ok(())
    }
    
    /// Dispose all scoped services and lifecycle managed services
    pub async fn dispose_all(&mut self) -> Result<(), CoreError> {
        // Dispose all scoped services first
        let scope_ids: Vec<ScopeId> = {
            let scopes = self.scopes.read().map_err(|_| CoreError::LockError {
                resource: "scopes".to_string(),
            })?;
            scopes.keys().cloned().collect()
        };
        
        for scope_id in scope_ids {
            self.dispose_scope(&scope_id).await?;
        }
        
        // Dispose lifecycle managed services
        self.lifecycle_manager.dispose_all().await?;
        
        Ok(())
    }
    
    /// Get a reference to the lifecycle manager
    pub fn lifecycle_manager(&self) -> &ServiceLifecycleManager {
        &self.lifecycle_manager
    }
    
    /// Get a mutable reference to the lifecycle manager
    pub fn lifecycle_manager_mut(&mut self) -> &mut ServiceLifecycleManager {
        &mut self.lifecycle_manager
    }
    
    /// Resolve a service by type
    pub fn resolve<T: Send + Sync + 'static>(&self) -> Result<Arc<T>, CoreError> {
        let service_id = ServiceId::of::<T>();
        self.resolve_by_id(&service_id)
    }
    
    /// Resolve a scoped service by type
    pub fn resolve_scoped<T: Send + Sync + 'static>(&self, scope_id: &ScopeId) -> Result<Arc<T>, CoreError> {
        let service_id = ServiceId::of::<T>();
        self.resolve_by_id_scoped(&service_id, scope_id)
    }
    
    /// Resolve a named service
    pub fn resolve_named<T: Send + Sync + 'static>(&self, name: &str) -> Result<Arc<T>, CoreError> {
        self.resolve_named_by_str::<T>(name)
    }
    
    /// Resolve a named service efficiently without allocating ServiceId
    fn resolve_named_by_str<T: Send + Sync + 'static>(&self, name: &str) -> Result<Arc<T>, CoreError> {
        if !self.is_built {
            return Err(CoreError::InvalidServiceDescriptor {
                message: "Container must be built before resolving services".to_string(),
            });
        }
        
        // Check if we have a cached instance - we need to create ServiceId for lookup in instances
        let service_id = ServiceId::named::<T>(name.to_string());
        {
            let instances = self.instances.read().map_err(|_| CoreError::LockError {
                resource: "service_instances".to_string(),
            })?;
            
            if let Some(ServiceInstance::Singleton(instance)) = instances.get(&service_id) {
                return instance.clone().downcast::<T>()
                    .map_err(|_| CoreError::ServiceNotFound {
                        service_type: format!("{}({})", 
                            std::any::type_name::<T>(),
                            name
                        ),
                    });
            }
        }
        
        // Get service descriptor efficiently without allocating ServiceId
        let descriptor = self.bindings.get_descriptor_named::<T>(name)
            .ok_or_else(|| CoreError::ServiceNotFound {
                service_type: format!("{}({})", 
                    std::any::type_name::<T>(),
                    name
                ),
            })?;
        
        // Resolve dependencies first
        self.resolve_dependencies(&descriptor.dependencies)?;
        
        // Create the service instance based on activation strategy
        let arc_instance = match &descriptor.activation_strategy {
            crate::container::descriptor::ServiceActivationStrategy::Factory(factory) => {
                let instance = factory()?;
                let typed_instance = instance.downcast::<T>()
                    .map_err(|_| CoreError::ServiceNotFound {
                        service_type: format!("{}({})", 
                            std::any::type_name::<T>(),
                            name
                        ),
                    })?;
                Arc::new(*typed_instance)
            },
            crate::container::descriptor::ServiceActivationStrategy::AutoWired => {
                return Err(CoreError::InvalidServiceDescriptor {
                    message: format!(
                        "Service {}({}) is marked as auto-wired but resolve_named was called instead of resolve_injectable. Use resolve_injectable() for auto-wired services.",
                        std::any::type_name::<T>(),
                        name
                    ),
                });
            }
        };
        
        // Cache if singleton (we already have the ServiceId)
        if descriptor.lifetime == ServiceScope::Singleton {
            let mut instances = self.instances.write().map_err(|_| CoreError::LockError {
                resource: "service_instances".to_string(),
            })?;
            instances.insert(service_id, ServiceInstance::Singleton(arc_instance.clone()));
        }
        
        Ok(arc_instance)
    }
    
    /// Resolve a service by service ID
    fn resolve_by_id<T: Send + Sync + 'static>(&self, service_id: &ServiceId) -> Result<Arc<T>, CoreError> {
        if !self.is_built {
            return Err(CoreError::InvalidServiceDescriptor {
                message: "Container must be built before resolving services".to_string(),
            });
        }
        
        // Check if we have a cached instance
        {
            let instances = self.instances.read().map_err(|_| CoreError::LockError {
                resource: "service_instances".to_string(),
            })?;
            
            if let Some(ServiceInstance::Singleton(instance)) = instances.get(service_id) {
                return instance.clone().downcast::<T>()
                    .map_err(|_| CoreError::ServiceNotFound {
                        service_type: format!("{}({})", 
                            std::any::type_name::<T>(),
                            service_id.name.as_deref().unwrap_or("default")
                        ),
                    });
            }
        }
        
        // Get service descriptor
        let descriptor = self.bindings.get_descriptor(service_id)
            .ok_or_else(|| CoreError::ServiceNotFound {
                service_type: format!("{}({})", 
                    std::any::type_name::<T>(),
                    service_id.name.as_deref().unwrap_or("default")
                ),
            })?;
        
        // Resolve dependencies first
        self.resolve_dependencies(&descriptor.dependencies)?;
        
        // Create the service instance based on activation strategy
        let arc_instance = match &descriptor.activation_strategy {
            crate::container::descriptor::ServiceActivationStrategy::Factory(factory) => {
                let instance = factory()?;
                let typed_instance = instance.downcast::<T>()
                    .map_err(|_| CoreError::ServiceNotFound {
                        service_type: format!("{}({})", 
                            std::any::type_name::<T>(),
                            service_id.name.as_deref().unwrap_or("default")
                        ),
                    })?;
                Arc::new(*typed_instance)
            },
            crate::container::descriptor::ServiceActivationStrategy::AutoWired => {
                return Err(CoreError::InvalidServiceDescriptor {
                    message: format!(
                        "Service {} is marked as auto-wired but resolve_by_id was called instead of resolve_injectable. Use resolve_injectable() for auto-wired services.",
                        std::any::type_name::<T>()
                    ),
                });
            }
        };
        
        // Cache if singleton
        if descriptor.lifetime == ServiceScope::Singleton {
            let mut instances = self.instances.write().map_err(|_| CoreError::LockError {
                resource: "service_instances".to_string(),
            })?;
            instances.insert(service_id.clone(), ServiceInstance::Singleton(arc_instance.clone()));
        }
        
        Ok(arc_instance)
    }
    
    /// Resolve a service by service ID in a specific scope
    fn resolve_by_id_scoped<T: Send + Sync + 'static>(&self, service_id: &ServiceId, scope_id: &ScopeId) -> Result<Arc<T>, CoreError> {
        if !self.is_built {
            return Err(CoreError::InvalidServiceDescriptor {
                message: "Container must be built before resolving services".to_string(),
            });
        }
        
        // Get service descriptor first to check lifetime
        let descriptor = self.bindings.get_descriptor(service_id)
            .ok_or_else(|| CoreError::ServiceNotFound {
                service_type: format!("{}({})", 
                    std::any::type_name::<T>(),
                    service_id.name.as_deref().unwrap_or("default")
                ),
            })?;
        
        // Handle based on lifetime
        match descriptor.lifetime {
            ServiceScope::Singleton => {
                // For singleton, ignore scope and use regular resolution
                self.resolve_by_id(service_id)
            },
            ServiceScope::Transient => {
                // For transient, create new instance every time
                self.create_service_instance::<T>(service_id, descriptor)
            },
            ServiceScope::Scoped => {
                // Check if we have a cached instance for this scope
                {
                    let instances = self.instances.read().map_err(|_| CoreError::LockError {
                        resource: "service_instances".to_string(),
                    })?;
                    
                    if let Some(ServiceInstance::Scoped(scoped_instances)) = instances.get(service_id) {
                        if let Some(instance) = scoped_instances.get(scope_id) {
                            return instance.clone().downcast::<T>()
                                .map_err(|_| CoreError::ServiceNotFound {
                                    service_type: format!("{}({})", 
                                        std::any::type_name::<T>(),
                                        service_id.name.as_deref().unwrap_or("default")
                                    ),
                                });
                        }
                    }
                }
                
                // Create new scoped instance
                let arc_instance = self.create_service_instance::<T>(service_id, descriptor)?;
                
                // Cache it for this scope
                let mut instances = self.instances.write().map_err(|_| CoreError::LockError {
                    resource: "service_instances".to_string(),
                })?;
                
                use std::collections::hash_map::Entry;
                match instances.entry(service_id.clone()) {
                    Entry::Occupied(mut entry) => {
                        match entry.get_mut() {
                            ServiceInstance::Scoped(scoped_instances) => {
                                scoped_instances.insert(scope_id.clone(), arc_instance.clone() as Arc<dyn Any + Send + Sync>);
                            }
                            ServiceInstance::Singleton(_) => {
                                return Err(CoreError::InvalidServiceDescriptor {
                                    message: format!(
                                        "Service {} is registered as both Singleton and Scoped. This is a configuration error.",
                                        std::any::type_name::<T>()
                                    ),
                                });
                            }
                        }
                    }
                    Entry::Vacant(entry) => {
                        let mut scoped_map = HashMap::new();
                        scoped_map.insert(scope_id.clone(), arc_instance.clone() as Arc<dyn Any + Send + Sync>);
                        entry.insert(ServiceInstance::Scoped(scoped_map));
                    }
                }
                
                Ok(arc_instance)
            }
        }
    }
    
    /// Create a service instance
    fn create_service_instance<T: Send + Sync + 'static>(&self, service_id: &ServiceId, descriptor: &crate::container::descriptor::ServiceDescriptor) -> Result<Arc<T>, CoreError> {
        // Resolve dependencies first
        self.resolve_dependencies(&descriptor.dependencies)?;
        
        // Create the service instance based on activation strategy
        match &descriptor.activation_strategy {
            crate::container::descriptor::ServiceActivationStrategy::Factory(factory) => {
                let instance = factory()?;
                let typed_instance = instance.downcast::<T>()
                    .map_err(|_| CoreError::ServiceNotFound {
                        service_type: format!("{}({})", 
                            std::any::type_name::<T>(),
                            service_id.name.as_deref().unwrap_or("default")
                        ),
                    })?;
                Ok(Arc::new(*typed_instance))
            },
            crate::container::descriptor::ServiceActivationStrategy::AutoWired => {
                Err(CoreError::InvalidServiceDescriptor {
                    message: format!(
                        "Service {} is marked as auto-wired but create_service_instance was called. Use resolve_injectable() for auto-wired services.",
                        std::any::type_name::<T>()
                    ),
                })
            }
        }
    }
    
    /// Resolve all dependencies for a service
    fn resolve_dependencies(&self, dependencies: &[ServiceId]) -> Result<(), CoreError> {
        for dep_id in dependencies {
            // For now, we'll just validate that the dependency exists
            if !self.bindings.contains(dep_id) {
                return Err(CoreError::ServiceNotFound {
                    service_type: format!("{}({})", 
                        dep_id.type_name(),
                        dep_id.name.as_deref().unwrap_or("default")
                    ),
                });
            }
        }
        Ok(())
    }
    
    /// Try to resolve a service, returning None if not found
    pub fn try_resolve<T: Send + Sync + 'static>(&self) -> Option<Arc<T>> {
        self.resolve::<T>().ok()
    }
    
    /// Try to resolve a named service, returning None if not found
    pub fn try_resolve_named<T: Send + Sync + 'static>(&self, name: &str) -> Option<Arc<T>> {
        self.resolve_named::<T>(name).ok()
    }
    
    /// Resolve a service using the Injectable trait (auto-wiring)
    pub fn resolve_injectable<T: Injectable>(&self) -> Result<Arc<T>, CoreError> {
        if !self.is_built {
            return Err(CoreError::InvalidServiceDescriptor {
                message: "Container must be built before resolving services".to_string(),
            });
        }
        
        let service_id = ServiceId::of::<T>();
        
        // Check if we have a cached instance
        {
            let instances = self.instances.read().map_err(|_| CoreError::LockError {
                resource: "service_instances".to_string(),
            })?;
            
            if let Some(ServiceInstance::Singleton(instance)) = instances.get(&service_id) {
                return instance.clone().downcast::<T>()
                    .map_err(|_| CoreError::ServiceNotFound {
                        service_type: std::any::type_name::<T>().to_string(),
                    });
            }
        }
        
        // Verify the service is configured for auto-wiring
        let descriptor = self.bindings.get_descriptor(&service_id)
            .ok_or_else(|| CoreError::ServiceNotFound {
                service_type: std::any::type_name::<T>().to_string(),
            })?;
            
        let arc_instance = match &descriptor.activation_strategy {
            crate::container::descriptor::ServiceActivationStrategy::AutoWired => {
                // Create the service using Injectable
                let service_instance = T::create(self)?;
                Arc::new(service_instance)
            },
            crate::container::descriptor::ServiceActivationStrategy::Factory(_) => {
                return Err(CoreError::InvalidServiceDescriptor {
                    message: format!(
                        "Service {} is configured with a factory but resolve_injectable was called. Use resolve() for factory-based services.",
                        std::any::type_name::<T>()
                    ),
                });
            }
        };
        
        // Cache if singleton
        if descriptor.lifetime == ServiceScope::Singleton {
            let mut instances = self.instances.write().map_err(|_| CoreError::LockError {
                resource: "service_instances".to_string(),
            })?;
            instances.insert(service_id, ServiceInstance::Singleton(arc_instance.clone()));
        }
        
        Ok(arc_instance)
    }
    
    /// Resolve a trait object by downcasting from a concrete implementation
    pub fn resolve_trait<T: ?Sized + Send + Sync + 'static>(&self) -> Result<Arc<T>, CoreError> {
        // For trait objects, we need special handling
        // This is a placeholder - in a real implementation, we'd need metadata about
        // which concrete type implements which trait
        Err(CoreError::ServiceNotFound {
            service_type: std::any::type_name::<T>().to_string(),
        })
    }
    
    /// Bind a service token to a concrete implementation with transient lifetime
    ///
    /// This creates a mapping from a service token to a concrete implementation,
    /// enabling semantic dependency resolution through tokens.
    ///
    /// ## Example
    /// ```rust
    /// use elif_core::container::{IocContainer, ServiceToken};
    ///
    /// // Define service trait and token
    /// trait EmailService: Send + Sync {}
    /// struct EmailToken;
    /// impl ServiceToken for EmailToken {
    ///     type Service = dyn EmailService;
    /// }
    ///
    /// // Implementation
    /// struct SmtpService;
    /// impl EmailService for SmtpService {}
    ///
    /// // Bind token to implementation
    /// let mut container = IocContainer::new();
    /// container.bind_token::<EmailToken, SmtpService>();
    /// ```
    pub fn bind_token<Token, Impl>(&mut self) -> Result<&mut Self, CoreError>
    where
        Token: ServiceToken,
        Impl: Send + Sync + Default + 'static,
    {
        self.bind_token_with_lifetime::<Token, Impl>(ServiceScope::Transient)
    }
    
    /// Bind a service token to a concrete implementation as a singleton
    pub fn bind_token_singleton<Token, Impl>(&mut self) -> Result<&mut Self, CoreError>
    where
        Token: ServiceToken,
        Impl: Send + Sync + Default + 'static,
    {
        self.bind_token_with_lifetime::<Token, Impl>(ServiceScope::Singleton)
    }
    
    /// Bind a service token to a concrete implementation as a scoped service
    pub fn bind_token_scoped<Token, Impl>(&mut self) -> Result<&mut Self, CoreError>
    where
        Token: ServiceToken,
        Impl: Send + Sync + Default + 'static,
    {
        self.bind_token_with_lifetime::<Token, Impl>(ServiceScope::Scoped)
    }
    
    /// Bind a service token to a concrete implementation with a specific lifetime
    pub fn bind_token_with_lifetime<Token, Impl>(&mut self, lifetime: ServiceScope) -> Result<&mut Self, CoreError>
    where
        Token: ServiceToken,
        Impl: Send + Sync + Default + 'static,
    {
        if self.is_built {
            return Err(CoreError::InvalidServiceDescriptor {
                message: "Cannot bind tokens after container is built".to_string(),
            });
        }
        
        // Register the token binding
        self.tokens.register::<Token, Impl>()
            .map_err(|e| CoreError::InvalidServiceDescriptor {
                message: format!("Failed to register token binding: {}", e),
            })?;
            
        // Get the token binding to create a service descriptor
        let token_binding = self.tokens.get_default::<Token>()
            .ok_or_else(|| CoreError::InvalidServiceDescriptor {
                message: "Failed to retrieve token binding after registration".to_string(),
            })?;
            
        // Create service descriptor for the implementation
        let service_id = token_binding.to_service_id();
        
        // Create a service descriptor directly with the token's service ID and specified lifetime
        let descriptor = crate::container::descriptor::ServiceDescriptor {
            service_id,
            implementation_id: std::any::TypeId::of::<Impl>(),
            lifetime,
            activation_strategy: crate::container::descriptor::ServiceActivationStrategy::Factory(
                Box::new(|| Ok(Box::new(Impl::default()) as Box<dyn Any + Send + Sync>))
            ),
            dependencies: Vec::new(),
        };
        
        self.bindings.add_descriptor(descriptor);
        
        Ok(self)
    }
    
    /// Bind a named service token to a concrete implementation
    pub fn bind_token_named<Token, Impl>(&mut self, name: impl Into<String>) -> Result<&mut Self, CoreError>
    where
        Token: ServiceToken,
        Impl: Send + Sync + Default + 'static,
    {
        if self.is_built {
            return Err(CoreError::InvalidServiceDescriptor {
                message: "Cannot bind tokens after container is built".to_string(),
            });
        }
        
        let name = name.into();
        
        // Register the named token binding
        self.tokens.register_named::<Token, Impl>(&name)
            .map_err(|e| CoreError::InvalidServiceDescriptor {
                message: format!("Failed to register named token binding: {}", e),
            })?;
            
        // Get the token binding to create a service descriptor
        let token_binding = self.tokens.get_named::<Token>(&name)
            .ok_or_else(|| CoreError::InvalidServiceDescriptor {
                message: "Failed to retrieve named token binding after registration".to_string(),
            })?;
            
        // Create service descriptor for the implementation
        let service_id = token_binding.to_service_id();
        
        // Create a service descriptor directly with the token's service ID
        let descriptor = crate::container::descriptor::ServiceDescriptor {
            service_id,
            implementation_id: std::any::TypeId::of::<Impl>(),
            lifetime: ServiceScope::Transient,
            activation_strategy: crate::container::descriptor::ServiceActivationStrategy::Factory(
                Box::new(|| Ok(Box::new(Impl::default()) as Box<dyn Any + Send + Sync>))
            ),
            dependencies: Vec::new(),
        };
        
        self.bindings.add_descriptor(descriptor);
        
        Ok(self)
    }
    
    /// Resolve a service by its token type
    ///
    /// This enables semantic dependency resolution where services are identified
    /// by tokens rather than concrete types, enabling true dependency inversion.
    ///
    /// ## Example
    /// ```rust
    /// let service = container.resolve_by_token::<EmailToken>()?;
    /// service.send("user@example.com", "Welcome", "Hello world!");
    /// ```
    pub fn resolve_by_token<Token>(&self) -> Result<Arc<Token::Service>, CoreError>
    where
        Token: ServiceToken,
        Token::Service: 'static,
    {
        if !self.is_built {
            return Err(CoreError::InvalidServiceDescriptor {
                message: "Container must be built before resolving services".to_string(),
            });
        }
        
        // Get the token binding
        let token_binding = self.tokens.get_default::<Token>()
            .ok_or_else(|| CoreError::ServiceNotFound {
                service_type: format!("token {} -> {}", 
                    Token::token_type_name(),
                    Token::service_type_name()
                ),
            })?;
            
        // Create service ID and resolve
        let service_id = token_binding.to_service_id();
        
        // Use a type-erased approach for trait object resolution
        // We need to resolve the concrete implementation and cast it to the trait
        self.resolve_by_id_as_trait::<Token::Service>(&service_id)
    }
    
    /// Resolve a named service by its token type
    pub fn resolve_by_token_named<Token>(&self, name: &str) -> Result<Arc<Token::Service>, CoreError>
    where
        Token: ServiceToken,
        Token::Service: 'static,
    {
        if !self.is_built {
            return Err(CoreError::InvalidServiceDescriptor {
                message: "Container must be built before resolving services".to_string(),
            });
        }
        
        // Get the named token binding
        let token_binding = self.tokens.get_named::<Token>(name)
            .ok_or_else(|| CoreError::ServiceNotFound {
                service_type: format!("named token {}({}) -> {}", 
                    Token::token_type_name(),
                    name,
                    Token::service_type_name()
                ),
            })?;
            
        // Create service ID and resolve
        let service_id = token_binding.to_service_id();
        
        // Use a type-erased approach for trait object resolution
        self.resolve_by_id_as_trait::<Token::Service>(&service_id)
    }
    
    /// Try to resolve a service by its token type, returning None if not found
    pub fn try_resolve_by_token<Token>(&self) -> Option<Arc<Token::Service>>
    where
        Token: ServiceToken,
        Token::Service: 'static,
    {
        self.resolve_by_token::<Token>().ok()
    }
    
    /// Try to resolve a named service by its token type, returning None if not found
    pub fn try_resolve_by_token_named<Token>(&self, name: &str) -> Option<Arc<Token::Service>>
    where
        Token: ServiceToken,
        Token::Service: 'static,
    {
        self.resolve_by_token_named::<Token>(name).ok()
    }
    
    /// Resolve a scoped service by its token type
    /// 
    /// This resolves services within a specific scope, maintaining the lifecycle
    /// and cleanup patterns expected by the existing scope management system.
    pub fn resolve_by_token_scoped<Token>(&self, scope_id: &ScopeId) -> Result<Arc<Token::Service>, CoreError>
    where
        Token: ServiceToken,
        Token::Service: 'static,
    {
        if !self.is_built {
            return Err(CoreError::InvalidServiceDescriptor {
                message: "Container must be built before resolving services".to_string(),
            });
        }
        
        // Get the token binding
        let token_binding = self.tokens.get_default::<Token>()
            .ok_or_else(|| CoreError::ServiceNotFound {
                service_type: format!("token {} -> {}", 
                    Token::token_type_name(),
                    Token::service_type_name()
                ),
            })?;
            
        // Create service ID and resolve in the specified scope
        let service_id = token_binding.to_service_id();
        
        // Use a type-erased approach for trait object resolution in scoped context
        self.resolve_by_id_as_trait_scoped::<Token::Service>(&service_id, scope_id)
    }
    
    /// Resolve a named scoped service by its token type
    pub fn resolve_by_token_named_scoped<Token>(&self, name: &str, scope_id: &ScopeId) -> Result<Arc<Token::Service>, CoreError>
    where
        Token: ServiceToken,
        Token::Service: 'static,
    {
        if !self.is_built {
            return Err(CoreError::InvalidServiceDescriptor {
                message: "Container must be built before resolving services".to_string(),
            });
        }
        
        // Get the named token binding
        let token_binding = self.tokens.get_named::<Token>(name)
            .ok_or_else(|| CoreError::ServiceNotFound {
                service_type: format!("named token {}({}) -> {}", 
                    Token::token_type_name(),
                    name,
                    Token::service_type_name()
                ),
            })?;
            
        // Create service ID and resolve in the specified scope
        let service_id = token_binding.to_service_id();
        
        // Use a type-erased approach for trait object resolution in scoped context
        self.resolve_by_id_as_trait_scoped::<Token::Service>(&service_id, scope_id)
    }
    
    /// Try to resolve a scoped service by its token type, returning None if not found
    pub fn try_resolve_by_token_scoped<Token>(&self, scope_id: &ScopeId) -> Option<Arc<Token::Service>>
    where
        Token: ServiceToken,
        Token::Service: 'static,
    {
        self.resolve_by_token_scoped::<Token>(scope_id).ok()
    }
    
    /// Try to resolve a named scoped service by its token type, returning None if not found
    pub fn try_resolve_by_token_named_scoped<Token>(&self, name: &str, scope_id: &ScopeId) -> Option<Arc<Token::Service>>
    where
        Token: ServiceToken,
        Token::Service: 'static,
    {
        self.resolve_by_token_named_scoped::<Token>(name, scope_id).ok()
    }
    
    /// Check if a token is registered
    pub fn contains_token<Token: ServiceToken>(&self) -> bool {
        self.tokens.contains::<Token>()
    }
    
    /// Check if a named token is registered
    pub fn contains_token_named<Token: ServiceToken>(&self, name: &str) -> bool {
        self.tokens.contains_named::<Token>(name)
    }
    
    /// Get token registry statistics
    pub fn token_stats(&self) -> crate::container::tokens::TokenRegistryStats {
        self.tokens.stats()
    }
    
    /// Internal method to resolve services as trait objects
    /// 
    /// This handles the complex type casting required for trait object resolution
    fn resolve_by_id_as_trait<T: ?Sized + Send + Sync + 'static>(&self, service_id: &ServiceId) -> Result<Arc<T>, CoreError> {
        // For now, this is a simplified implementation
        // In a full implementation, we would need more sophisticated trait object handling
        // that involves storing metadata about how to cast concrete types to trait objects
        
        // This is a placeholder that shows the intended API
        // The actual implementation would require additional metadata in the token bindings
        Err(CoreError::ServiceNotFound {
            service_type: format!("trait object resolution not yet fully implemented for service {}", 
                service_id.type_name()
            ),
        })
    }
    
    /// Internal method to resolve scoped services as trait objects
    /// 
    /// This handles scoped trait object resolution with proper lifecycle management
    fn resolve_by_id_as_trait_scoped<T: ?Sized + Send + Sync + 'static>(&self, service_id: &ServiceId, _scope_id: &ScopeId) -> Result<Arc<T>, CoreError> {
        // For now, this is a simplified implementation
        // In a full implementation, this would integrate with the scoped service resolution
        // and maintain proper lifecycle management within the specified scope
        
        // This is a placeholder that shows the intended API
        // The actual implementation would require additional metadata in the token bindings
        // and proper integration with the scope management system
        Err(CoreError::ServiceNotFound {
            service_type: format!("scoped trait object resolution not yet fully implemented for service {}", 
                service_id.type_name()
            ),
        })
    }
    
    /// Check if a service is registered
    pub fn contains<T: 'static>(&self) -> bool {
        let service_id = ServiceId::of::<T>();
        self.bindings.contains(&service_id)
    }
    
    /// Check if a named service is registered
    pub fn contains_named<T: 'static>(&self, name: &str) -> bool {
        self.bindings.contains_named::<T>(name)
    }
    
    /// Get the number of registered services
    pub fn service_count(&self) -> usize {
        self.bindings.count()
    }
    
    /// Get all registered service IDs
    pub fn registered_services(&self) -> Vec<ServiceId> {
        self.bindings.service_ids()
    }
    
    /// Check if the container is built and ready
    pub fn is_built(&self) -> bool {
        self.is_built
    }

    /// Validate the container configuration
    pub fn validate(&self) -> Result<(), CoreError> {
        if !self.is_built {
            return Err(CoreError::InvalidServiceDescriptor {
                message: "Container must be built before validation".to_string(),
            });
        }
        
        // Validate dependency resolution
        if let Some(resolver) = &self.resolver {
            let service_ids = self.bindings.service_ids().into_iter().collect();
            resolver.validate_dependencies(&service_ids)?;
        }
        
        Ok(())
    }

    /// Resolve all implementations of an interface as a vector
    pub fn resolve_all<T: Send + Sync + 'static>(&self) -> Result<Vec<Arc<T>>, CoreError> {
        if !self.is_built {
            return Err(CoreError::InvalidServiceDescriptor {
                message: "Container must be built before resolving services".to_string(),
            });
        }
        
        let mut implementations = Vec::new();
        
        // Find all descriptors that match the type
        for descriptor in self.bindings.descriptors() {
            if descriptor.service_id.type_id == std::any::TypeId::of::<T>() {
                match self.resolve_by_id::<T>(&descriptor.service_id) {
                    Ok(instance) => implementations.push(instance),
                    Err(_) => continue, // Skip failed resolutions
                }
            }
        }
        
        if implementations.is_empty() {
            return Err(CoreError::ServiceNotFound {
                service_type: std::any::type_name::<T>().to_string(),
            });
        }
        
        Ok(implementations)
    }
    
    /// Resolve all implementations of an interface as a HashMap with their names
    pub fn resolve_all_named<T: Send + Sync + 'static>(&self) -> Result<std::collections::HashMap<String, Arc<T>>, CoreError> {
        if !self.is_built {
            return Err(CoreError::InvalidServiceDescriptor {
                message: "Container must be built before resolving services".to_string(),
            });
        }
        
        let mut implementations = std::collections::HashMap::new();
        
        // Find all named descriptors that match the type
        for descriptor in self.bindings.descriptors() {
            if descriptor.service_id.type_id == std::any::TypeId::of::<T>() {
                if let Some(name) = &descriptor.service_id.name {
                    match self.resolve_by_id::<T>(&descriptor.service_id) {
                        Ok(instance) => {
                            implementations.insert(name.clone(), instance);
                        },
                        Err(_) => continue, // Skip failed resolutions
                    }
                }
            }
        }
        
        if implementations.is_empty() {
            return Err(CoreError::ServiceNotFound {
                service_type: format!("named implementations of {}", std::any::type_name::<T>()),
            });
        }
        
        Ok(implementations)
    }
    
    /// Get default implementation for a type (marked with is_default in BindingConfig)
    pub fn resolve_default<T: Send + Sync + 'static>(&self) -> Result<Arc<T>, CoreError> {
        // For now, just resolve the first unnamed implementation
        // In a full implementation, we'd track which binding was marked as default
        self.resolve::<T>()
    }
    
    /// Get service information for debugging
    pub fn get_service_info<T: 'static>(&self) -> Option<String> {
        let service_id = ServiceId::of::<T>();
        self.bindings.get_descriptor(&service_id)
            .map(|desc| format!("{:?}", desc))
    }
    
    /// Get all registered service IDs for debugging
    pub fn get_registered_services(&self) -> Vec<String> {
        self.bindings.service_ids()
            .into_iter()
            .map(|id| format!("{} ({})", id.type_name(), id.name.unwrap_or_else(|| "default".to_string())))
            .collect()
    }
    
    /// Validate that all registered services can be resolved
    pub fn validate_all_services(&self) -> Result<(), Vec<CoreError>> {
        if !self.is_built {
            return Err(vec![CoreError::InvalidServiceDescriptor {
                message: "Container must be built before validation".to_string(),
            }]);
        }
        
        let mut errors = Vec::new();
        
        for descriptor in self.bindings.descriptors() {
            // Validate dependencies exist
            for dependency in &descriptor.dependencies {
                if !self.bindings.contains(dependency) {
                    errors.push(CoreError::ServiceNotFound {
                        service_type: format!("{} (dependency of {})", 
                            dependency.type_name(),
                            descriptor.service_id.type_name()
                        ),
                    });
                }
            }
        }
        
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
    
    /// Get service statistics
    pub fn get_statistics(&self) -> ServiceStatistics {
        let mut stats = ServiceStatistics::default();
        
        stats.total_services = self.bindings.count();
        stats.singleton_services = 0;
        stats.transient_services = 0;
        stats.scoped_services = 0;
        stats.cached_instances = 0;
        
        for descriptor in self.bindings.descriptors() {
            match descriptor.lifetime {
                crate::container::scope::ServiceScope::Singleton => stats.singleton_services += 1,
                crate::container::scope::ServiceScope::Transient => stats.transient_services += 1,
                crate::container::scope::ServiceScope::Scoped => stats.scoped_services += 1,
            }
        }
        
        if let Ok(instances) = self.instances.read() {
            stats.cached_instances = instances.len();
        }
        
        stats
    }
}

/// Service statistics for monitoring and debugging
#[derive(Debug, Default)]
pub struct ServiceStatistics {
    pub total_services: usize,
    pub singleton_services: usize,
    pub transient_services: usize,
    pub scoped_services: usize,
    pub cached_instances: usize,
}

impl ServiceBinder for IocContainer {
    fn add_service_descriptor(&mut self, descriptor: crate::container::descriptor::ServiceDescriptor) -> Result<&mut Self, CoreError> {
        if self.is_built {
            return Err(CoreError::InvalidServiceDescriptor { 
                message: "Cannot add service descriptors after container is built".to_string() 
            });
        }
        self.bindings.add_descriptor(descriptor);
        Ok(self)
    }
    
    fn bind<TInterface: ?Sized + 'static, TImpl: Send + Sync + Default + 'static>(&mut self) -> &mut Self {
        if self.is_built {
            panic!("Cannot add bindings after container is built");
        }
        self.bindings.bind::<TInterface, TImpl>();
        self
    }
    
    fn bind_singleton<TInterface: ?Sized + 'static, TImpl: Send + Sync + Default + 'static>(&mut self) -> &mut Self {
        if self.is_built {
            panic!("Cannot add bindings after container is built");
        }
        self.bindings.bind_singleton::<TInterface, TImpl>();
        self
    }
    
    fn bind_transient<TInterface: ?Sized + 'static, TImpl: Send + Sync + Default + 'static>(&mut self) -> &mut Self {
        if self.is_built {
            panic!("Cannot add bindings after container is built");
        }
        self.bindings.bind_transient::<TInterface, TImpl>();
        self
    }
    
    fn bind_factory<TInterface: ?Sized + 'static, F, T>(&mut self, factory: F) -> &mut Self
    where
        F: Fn() -> Result<T, CoreError> + Send + Sync + 'static,
        T: Send + Sync + 'static,
    {
        if self.is_built {
            panic!("Cannot add bindings after container is built");
        }
        self.bindings.bind_factory::<TInterface, _, _>(factory);
        self
    }
    
    fn bind_instance<TInterface: ?Sized + 'static, TImpl: Send + Sync + Clone + 'static>(&mut self, instance: TImpl) -> &mut Self {
        if self.is_built {
            panic!("Cannot add bindings after container is built");
        }
        self.bindings.bind_instance::<TInterface, TImpl>(instance);
        self
    }
    
    fn bind_named<TInterface: ?Sized + 'static, TImpl: Send + Sync + Default + 'static>(&mut self, name: &str) -> &mut Self {
        if self.is_built {
            panic!("Cannot add bindings after container is built");
        }
        self.bindings.bind_named::<TInterface, TImpl>(name);
        self
    }
    
    fn bind_injectable<T: Injectable>(&mut self) -> &mut Self {
        if self.is_built {
            panic!("Cannot add bindings after container is built");
        }
        self.bindings.bind_injectable::<T>();
        self
    }
    
    fn bind_injectable_singleton<T: Injectable>(&mut self) -> &mut Self {
        if self.is_built {
            panic!("Cannot add bindings after container is built");
        }
        self.bindings.bind_injectable_singleton::<T>();
        self
    }

    // Advanced binding methods implementation

    fn bind_with<TInterface: ?Sized + 'static, TImpl: Send + Sync + Default + 'static>(&mut self) -> crate::container::binding::AdvancedBindingBuilder<TInterface> {
        if self.is_built {
            panic!("Cannot add bindings after container is built");
        }
        self.bindings.bind_with::<TInterface, TImpl>()
    }

    fn with_implementation<TInterface: ?Sized + 'static, TImpl: Send + Sync + Default + 'static>(&mut self, config: crate::container::binding::BindingConfig) -> &mut Self {
        if self.is_built {
            panic!("Cannot add bindings after container is built");
        }
        self.bindings.with_implementation::<TInterface, TImpl>(config);
        self
    }

    fn bind_lazy<TInterface: ?Sized + 'static, F, T>(&mut self, factory: F) -> &mut Self
    where
        F: Fn() -> T + Send + Sync + 'static,
        T: Send + Sync + 'static,
    {
        if self.is_built {
            panic!("Cannot add bindings after container is built");
        }
        self.bindings.bind_lazy::<TInterface, F, T>(factory);
        self
    }

    fn bind_parameterized_factory<TInterface: ?Sized + 'static, P, F, T>(&mut self, factory: F) -> &mut Self
    where
        F: Fn(P) -> Result<T, CoreError> + Send + Sync + 'static,
        T: Send + Sync + 'static,
        P: Send + Sync + 'static,
    {
        if self.is_built {
            panic!("Cannot add bindings after container is built");
        }
        self.bindings.bind_parameterized_factory::<TInterface, P, F, T>(factory);
        self
    }

    fn bind_collection<TInterface: ?Sized + 'static, F>(&mut self, configure: F) -> &mut Self
    where
        F: FnOnce(&mut crate::container::binding::CollectionBindingBuilder<TInterface>),
    {
        if self.is_built {
            panic!("Cannot add bindings after container is built");
        }
        self.bindings.bind_collection::<TInterface, F>(configure);
        self
    }

}

impl Default for IocContainer {
    fn default() -> Self {
        Self::new()
    }
}

impl DependencyResolver for IocContainer {
    fn resolve<T: Send + Sync + 'static>(&self) -> Result<Arc<T>, CoreError> {
        self.resolve::<T>()
    }
    
    fn resolve_named<T: Send + Sync + 'static>(&self, name: &str) -> Result<Arc<T>, CoreError> {
        self.resolve_named::<T>(name)
    }
    
    fn try_resolve<T: Send + Sync + 'static>(&self) -> Option<Arc<T>> {
        self.try_resolve::<T>()
    }
    
    fn try_resolve_named<T: Send + Sync + 'static>(&self, name: &str) -> Option<Arc<T>> {
        self.try_resolve_named::<T>(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    trait TestRepository: Send + Sync {
        fn find(&self, id: u32) -> Option<String>;
    }

    #[derive(Default)]
    struct PostgresRepository;
    
    unsafe impl Send for PostgresRepository {}
    unsafe impl Sync for PostgresRepository {}

    impl TestRepository for PostgresRepository {
        fn find(&self, _id: u32) -> Option<String> {
            Some("postgres_data".to_string())
        }
    }

    trait TestService: Send + Sync {
        fn get_data(&self) -> String;
    }

    #[derive(Default)]
    struct UserService;
    
    unsafe impl Send for UserService {}
    unsafe impl Sync for UserService {}

    impl TestService for UserService {
        fn get_data(&self) -> String {
            "user_data".to_string()
        }
    }

    #[test]
    fn test_basic_binding_and_resolution() {
        let mut container = IocContainer::new();
        
        container
            .bind::<PostgresRepository, PostgresRepository>()
            .bind_singleton::<UserService, UserService>();
        
        container.build().unwrap();
        
        let repo = container.resolve::<PostgresRepository>().unwrap();
        assert_eq!(repo.find(1), Some("postgres_data".to_string()));
        
        let service = container.resolve::<UserService>().unwrap();
        assert_eq!(service.get_data(), "user_data");
    }

    #[test]
    fn test_named_services() {
        let mut container = IocContainer::new();
        
        container
            .bind_named::<PostgresRepository, PostgresRepository>("postgres")
            .bind_named::<PostgresRepository, PostgresRepository>("backup");
        
        container.build().unwrap();
        
        let postgres_repo = container.resolve_named::<PostgresRepository>("postgres").unwrap();
        let backup_repo = container.resolve_named::<PostgresRepository>("backup").unwrap();
        
        assert_eq!(postgres_repo.find(1), Some("postgres_data".to_string()));
        assert_eq!(backup_repo.find(1), Some("postgres_data".to_string()));
    }

    #[test]
    fn test_singleton_behavior() {
        let mut container = IocContainer::new();
        
        container.bind_singleton::<UserService, UserService>();
        container.build().unwrap();
        
        let service1 = container.resolve::<UserService>().unwrap();
        let service2 = container.resolve::<UserService>().unwrap();
        
        // Should be the same instance
        assert!(Arc::ptr_eq(&service1, &service2));
    }

    #[test]
    fn test_transient_behavior() {
        let mut container = IocContainer::new();
        
        container.bind_transient::<UserService, UserService>();
        container.build().unwrap();
        
        let service1 = container.resolve::<UserService>().unwrap();
        let service2 = container.resolve::<UserService>().unwrap();
        
        // Should be different instances
        assert!(!Arc::ptr_eq(&service1, &service2));
    }

    #[test]
    #[should_panic(expected = "Cannot add bindings after container is built")]
    fn test_cannot_bind_after_build() {
        let mut container = IocContainer::new();
        container.build().unwrap();
        
        // This should panic
        container.bind::<UserService, UserService>();
    }

    #[test]
    fn test_service_not_found() {
        let mut container = IocContainer::new();
        container.build().unwrap();
        
        let result = container.resolve::<UserService>();
        assert!(result.is_err());
        
        if let Err(CoreError::ServiceNotFound { service_type }) = result {
            assert!(service_type.contains("UserService"));
        }
    }
}