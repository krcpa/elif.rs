/// Service scope enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceScope {
    /// Single instance shared across the application
    Singleton,
    /// New instance created for each request
    Transient,
    /// Instance scoped to a particular context (e.g., request scope)
    Scoped,
}

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

impl Default for ServiceScope {
    fn default() -> Self {
        ServiceScope::Singleton
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

/// Scoped service manager for managing services within a specific scope
#[derive(Debug)]
pub struct ScopedServiceManager {
    scope_id: uuid::Uuid,
    services: std::collections::HashMap<std::any::TypeId, Box<dyn std::any::Any + Send + Sync>>,
}

impl ScopedServiceManager {
    /// Create a new scoped service manager
    pub fn new() -> Self {
        Self {
            scope_id: uuid::Uuid::new_v4(),
            services: std::collections::HashMap::new(),
        }
    }
    
    /// Get the scope ID
    pub fn scope_id(&self) -> uuid::Uuid {
        self.scope_id
    }
    
    /// Add a service to this scope
    pub fn add_service<T>(&mut self, service: T)
    where
        T: Send + Sync + 'static,
    {
        let type_id = std::any::TypeId::of::<T>();
        self.services.insert(type_id, Box::new(service));
    }
    
    /// Get a service from this scope
    pub fn get_service<T>(&self) -> Option<&T>
    where
        T: Send + Sync + 'static,
    {
        let type_id = std::any::TypeId::of::<T>();
        self.services.get(&type_id)?.downcast_ref::<T>()
    }
    
    /// Check if a service exists in this scope
    pub fn has_service<T>(&self) -> bool
    where
        T: Send + Sync + 'static,
    {
        let type_id = std::any::TypeId::of::<T>();
        self.services.contains_key(&type_id)
    }
    
    /// Clear all services from this scope
    pub fn clear(&mut self) {
        self.services.clear();
    }
    
    /// Get the number of services in this scope
    pub fn service_count(&self) -> usize {
        self.services.len()
    }
}

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
        assert_eq!("singleton".parse::<ServiceScope>().unwrap(), ServiceScope::Singleton);
        assert_eq!("transient".parse::<ServiceScope>().unwrap(), ServiceScope::Transient);
        assert_eq!("scoped".parse::<ServiceScope>().unwrap(), ServiceScope::Scoped);
        
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
        let mut manager = ScopedServiceManager::new();
        
        manager.add_service("test_string".to_string());
        manager.add_service(42u32);
        
        assert!(manager.has_service::<String>());
        assert!(manager.has_service::<u32>());
        assert!(!manager.has_service::<i32>());
        
        assert_eq!(manager.get_service::<String>(), Some(&"test_string".to_string()));
        assert_eq!(manager.get_service::<u32>(), Some(&42u32));
        
        assert_eq!(manager.service_count(), 2);
        
        manager.clear();
        assert_eq!(manager.service_count(), 0);
    }
}