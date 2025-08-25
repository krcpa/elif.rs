/// Service scope enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ServiceScope {
    /// Single instance shared across the application
    #[default]
    Singleton,
    /// New instance created for each request
    Transient,
    /// Instance scoped to a particular context (e.g., request scope)
    Scoped,
}

/// Service lifetime type alias for compatibility
pub type ServiceLifetime = ServiceScope;

impl ServiceScope {
    /// Check if the scope is singleton
    pub fn is_singleton(&self) -> bool {
        matches!(self, ServiceScope::Singleton)
    }

    /// Check if the scope is transient
    pub fn is_transient(&self) -> bool {
        matches!(self, ServiceScope::Transient)
    }

    /// Check if the scope is scoped
    pub fn is_scoped(&self) -> bool {
        matches!(self, ServiceScope::Scoped)
    }

    /// Get the scope name as a string
    pub fn as_str(&self) -> &'static str {
        match self {
            ServiceScope::Singleton => "singleton",
            ServiceScope::Transient => "transient",
            ServiceScope::Scoped => "scoped",
        }
    }
}

impl std::fmt::Display for ServiceScope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for ServiceScope {
    type Err = crate::errors::CoreError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "singleton" => Ok(ServiceScope::Singleton),
            "transient" => Ok(ServiceScope::Transient),
            "scoped" => Ok(ServiceScope::Scoped),
            _ => Err(crate::errors::CoreError::InvalidServiceScope {
                scope: s.to_string(),
            }),
        }
    }
}

/// Unique scope identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ScopeId(uuid::Uuid);

impl ScopeId {
    /// Create a new scope ID
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }

    /// Get the inner UUID
    pub fn as_uuid(&self) -> uuid::Uuid {
        self.0
    }
}

impl Default for ScopeId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ScopeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Scoped service manager for managing services within a specific scope
#[derive(Debug)]
pub struct ScopedServiceManager {
    scope_id: ScopeId,
    services: std::sync::RwLock<
        std::collections::HashMap<std::any::TypeId, Box<dyn std::any::Any + Send + Sync>>,
    >,
    parent: Option<std::sync::Arc<ScopedServiceManager>>,
}

impl ScopedServiceManager {
    /// Create a new scoped service manager
    pub fn new() -> Self {
        Self {
            scope_id: ScopeId::new(),
            services: std::sync::RwLock::new(std::collections::HashMap::new()),
            parent: None,
        }
    }

    /// Create a child scope with this scope as parent
    /// This requires the parent to be wrapped in an Arc
    pub fn create_child(parent: std::sync::Arc<Self>) -> Self {
        Self {
            scope_id: ScopeId::new(),
            services: std::sync::RwLock::new(std::collections::HashMap::new()),
            parent: Some(parent),
        }
    }

    /// Get the scope ID
    pub fn scope_id(&self) -> &ScopeId {
        &self.scope_id
    }

    /// Add a service to this scope
    pub fn add_service<T>(&self, service: T)
    where
        T: Send + Sync + 'static,
    {
        let type_id = std::any::TypeId::of::<T>();
        let mut services = self.services.write().unwrap();
        services.insert(type_id, Box::new(service));
    }

    /// Store a service as Arc in this scope
    pub fn add_arc_service<T>(&self, service: std::sync::Arc<T>)
    where
        T: Send + Sync + 'static,
    {
        let type_id = std::any::TypeId::of::<T>();
        let mut services = self.services.write().unwrap();
        services.insert(type_id, Box::new(service));
    }

    /// Get a service from this scope, checking parent scopes if not found
    /// Services must be stored as Arc<T> for this to work
    pub fn get_arc_service<T>(&self) -> Option<std::sync::Arc<T>>
    where
        T: Send + Sync + 'static,
    {
        let type_id = std::any::TypeId::of::<T>();

        // Check current scope first
        {
            let services = self.services.read().unwrap();
            if let Some(service) = services.get(&type_id) {
                if let Some(arc) = service.downcast_ref::<std::sync::Arc<T>>() {
                    return Some(arc.clone());
                }
            }
        }

        // Check parent scopes recursively
        self.parent.as_ref()?.get_arc_service::<T>()
    }

    /// Check if a service exists in this scope or parent scopes
    pub fn has_service<T>(&self) -> bool
    where
        T: Send + Sync + 'static,
    {
        let type_id = std::any::TypeId::of::<T>();

        // Check current scope first
        {
            let services = self.services.read().unwrap();
            if services.contains_key(&type_id) {
                return true;
            }
        }

        // Check parent scopes
        self.parent.as_ref().is_some_and(|p| p.has_service::<T>())
    }

    /// Check if a service exists in this specific scope (not parent scopes)
    pub fn has_service_local<T>(&self) -> bool
    where
        T: Send + Sync + 'static,
    {
        let type_id = std::any::TypeId::of::<T>();
        let services = self.services.read().unwrap();
        services.contains_key(&type_id)
    }

    /// Clear all services from this scope
    pub fn clear(&self) {
        let mut services = self.services.write().unwrap();
        services.clear();
    }

    /// Get the number of services in this scope
    pub fn service_count(&self) -> usize {
        let services = self.services.read().unwrap();
        services.len()
    }

    /// Get parent scope
    pub fn parent(&self) -> Option<&std::sync::Arc<ScopedServiceManager>> {
        self.parent.as_ref()
    }
}

// Note: ScopedServiceManager intentionally does not implement Clone
// to prevent accidental creation of empty service managers.
// Use Arc<ScopedServiceManager> for sharing.

impl Default for ScopedServiceManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_scope_from_str() {
        assert_eq!(
            "singleton".parse::<ServiceScope>().unwrap(),
            ServiceScope::Singleton
        );
        assert_eq!(
            "transient".parse::<ServiceScope>().unwrap(),
            ServiceScope::Transient
        );
        assert_eq!(
            "scoped".parse::<ServiceScope>().unwrap(),
            ServiceScope::Scoped
        );

        assert!("invalid".parse::<ServiceScope>().is_err());
    }

    #[test]
    fn test_service_scope_display() {
        assert_eq!(format!("{}", ServiceScope::Singleton), "singleton");
        assert_eq!(format!("{}", ServiceScope::Transient), "transient");
        assert_eq!(format!("{}", ServiceScope::Scoped), "scoped");
    }

    #[test]
    fn test_scoped_service_manager() {
        let manager = ScopedServiceManager::new();

        // Use Arc for services to enable proper sharing
        manager.add_arc_service(std::sync::Arc::new("test_string".to_string()));
        manager.add_arc_service(std::sync::Arc::new(42u32));

        assert!(manager.has_service::<String>());
        assert!(manager.has_service::<u32>());
        assert!(!manager.has_service::<i32>());

        assert_eq!(
            *manager.get_arc_service::<String>().unwrap(),
            "test_string".to_string()
        );
        assert_eq!(*manager.get_arc_service::<u32>().unwrap(), 42u32);

        assert_eq!(manager.service_count(), 2);

        manager.clear();
        assert_eq!(manager.service_count(), 0);
    }

    #[test]
    fn test_scope_inheritance() {
        let parent = std::sync::Arc::new(ScopedServiceManager::new());

        // Add services to parent
        parent.add_arc_service(std::sync::Arc::new("parent_string".to_string()));
        parent.add_arc_service(std::sync::Arc::new(100u32));

        // Create child scope
        let child = std::sync::Arc::new(ScopedServiceManager::create_child(parent.clone()));

        // Add service to child
        child.add_arc_service(std::sync::Arc::new(200u32));

        // Child should see its own services
        assert_eq!(*child.get_arc_service::<u32>().unwrap(), 200u32);

        // Child should also see parent's services
        assert_eq!(
            *child.get_arc_service::<String>().unwrap(),
            "parent_string".to_string()
        );

        // Parent should not see child's services
        assert_eq!(*parent.get_arc_service::<u32>().unwrap(), 100u32);

        // Service counts
        assert_eq!(parent.service_count(), 2);
        assert_eq!(child.service_count(), 1); // Only its own services

        // has_service should check parent too
        assert!(child.has_service::<String>()); // From parent
        assert!(child.has_service::<u32>()); // From self
        assert!(!child.has_service_local::<String>()); // Not in child itself
        assert!(child.has_service_local::<u32>()); // In child itself
    }
}
