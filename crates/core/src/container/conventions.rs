use std::collections::HashMap;
use std::marker::PhantomData;

use crate::container::binding::ServiceBinder;
use crate::container::scope::ServiceScope;
use crate::errors::CoreError;

/// Marker trait for services that can be automatically discovered
pub trait AutoDiscoverable {
    /// Get the service lifetime convention (default: Singleton)
    fn lifetime_convention() -> ServiceScope {
        ServiceScope::Singleton
    }

    /// Check if this service implements a trait (for interface binding)
    fn implements_trait() -> Option<&'static str> {
        None
    }

    /// Get service tags for multi-implementation scenarios
    fn service_tags() -> Vec<String> {
        vec![]
    }
}

/// Attribute-based service registration
pub trait ServiceAttribute {
    /// Get the configured lifetime
    fn lifetime() -> ServiceScope;

    /// Get the service name if specified
    fn name() -> Option<String>;

    /// Get the interface this service implements
    fn implements() -> Option<String>;

    /// Check if this is the default implementation
    fn is_default() -> bool;
}

/// Convention-based service discovery
pub struct ServiceConventions {
    /// Type name patterns and their default lifetimes
    naming_conventions: HashMap<String, ServiceScope>,
    /// Interface naming conventions (e.g., "I" prefix)
    interface_patterns: Vec<String>,
    /// Assembly/module scan paths
    scan_paths: Vec<String>,
    /// Custom convention rules
    custom_rules: Vec<Box<dyn ConventionRule>>,
}

impl ServiceConventions {
    /// Create new service conventions with defaults
    pub fn new() -> Self {
        let mut conventions = HashMap::new();

        // Default naming conventions
        conventions.insert("*Service".to_string(), ServiceScope::Singleton);
        conventions.insert("*Repository".to_string(), ServiceScope::Scoped);
        conventions.insert("*Factory".to_string(), ServiceScope::Transient);
        conventions.insert("*Handler".to_string(), ServiceScope::Singleton);
        conventions.insert("*Controller".to_string(), ServiceScope::Scoped);
        conventions.insert("*Validator".to_string(), ServiceScope::Transient);
        conventions.insert("*Cache".to_string(), ServiceScope::Singleton);
        conventions.insert("*Logger".to_string(), ServiceScope::Singleton);

        Self {
            naming_conventions: conventions,
            interface_patterns: vec!["I*".to_string(), "*Trait".to_string()],
            scan_paths: vec!["src/**/*.rs".to_string()],
            custom_rules: vec![],
        }
    }

    /// Add a naming convention
    pub fn add_naming_convention(&mut self, pattern: &str, lifetime: ServiceScope) -> &mut Self {
        self.naming_conventions
            .insert(pattern.to_string(), lifetime);
        self
    }

    /// Add an interface naming pattern
    pub fn add_interface_pattern(&mut self, pattern: &str) -> &mut Self {
        self.interface_patterns.push(pattern.to_string());
        self
    }

    /// Add a scan path
    pub fn add_scan_path(&mut self, path: &str) -> &mut Self {
        self.scan_paths.push(path.to_string());
        self
    }

    /// Add a custom convention rule
    pub fn add_custom_rule<R: ConventionRule + 'static>(&mut self, rule: R) -> &mut Self {
        self.custom_rules.push(Box::new(rule));
        self
    }

    /// Get lifetime for a service type name
    pub fn get_lifetime_for_type(&self, type_name: &str) -> ServiceScope {
        // Check custom rules first (higher priority)
        for rule in &self.custom_rules {
            if let Some(lifetime) = rule.get_lifetime(type_name) {
                return lifetime;
            }
        }

        // Check naming conventions
        for (pattern, lifetime) in &self.naming_conventions {
            if self.matches_pattern(type_name, pattern) {
                return *lifetime;
            }
        }

        // Default to transient
        ServiceScope::Transient
    }

    /// Check if a type name is an interface
    pub fn is_interface(&self, type_name: &str) -> bool {
        for pattern in &self.interface_patterns {
            if self.matches_pattern(type_name, pattern) {
                return true;
            }
        }
        false
    }

    /// Find implementation for an interface by convention
    pub fn find_implementation_for_interface(&self, interface_name: &str) -> Option<String> {
        // Remove 'I' prefix if present
        if interface_name.starts_with('I') && interface_name.len() > 1 {
            let impl_name = &interface_name[1..];
            return Some(format!("{}Impl", impl_name)); // or just impl_name
        }

        // Remove 'Trait' suffix if present
        if let Some(impl_name) = interface_name.strip_suffix("Trait") {
            return Some(format!("{}Impl", impl_name));
        }

        None
    }

    /// Match a string against a pattern (* wildcard supported)
    fn matches_pattern(&self, text: &str, pattern: &str) -> bool {
        if pattern == "*" {
            return true;
        }

        if pattern.starts_with('*') && pattern.ends_with('*') {
            let middle = &pattern[1..pattern.len() - 1];
            return text.contains(middle);
        }

        if let Some(suffix) = pattern.strip_prefix('*') {
            return text.ends_with(suffix);
        }

        if let Some(prefix) = pattern.strip_suffix('*') {
            return text.starts_with(prefix);
        }

        text == pattern
    }
}

impl Default for ServiceConventions {
    fn default() -> Self {
        Self::new()
    }
}

/// Custom convention rule trait
pub trait ConventionRule: Send + Sync {
    /// Get lifetime for a type name, return None if rule doesn't apply
    fn get_lifetime(&self, type_name: &str) -> Option<ServiceScope>;

    /// Check if a type should be treated as an interface
    fn is_interface(&self, _type_name: &str) -> Option<bool> {
        None
    }

    /// Find implementation for an interface
    fn find_implementation(&self, _interface_name: &str) -> Option<String> {
        None
    }
}

/// Builder for convention-based container configuration
#[allow(dead_code)]
pub struct ConventionBasedBuilder<T> {
    conventions: ServiceConventions,
    _phantom: PhantomData<T>,
}

impl<T> ConventionBasedBuilder<T>
where
    T: ServiceBinder,
{
    /// Create a new convention-based builder
    pub fn new(conventions: ServiceConventions) -> Self {
        Self {
            conventions,
            _phantom: PhantomData,
        }
    }

    /// Configure container using conventions
    pub fn configure_container(self, _container: &mut T) -> Result<(), CoreError> {
        // This would scan the codebase and register services automatically
        // For now, this is a placeholder that would integrate with proc macros
        // or reflection to discover services at compile time or runtime

        // The actual implementation would:
        // 1. Scan specified paths for types with #[service] attribute
        // 2. Apply naming conventions to discovered types
        // 3. Register bindings based on conventions
        // 4. Handle interface-to-implementation bindings

        Ok(())
    }
}

/// Attribute macro support structures
/// These would be used by proc macros to store metadata
///
/// Service metadata extracted from attributes
#[derive(Debug, Clone)]
pub struct ServiceMetadata {
    pub type_name: String,
    pub lifetime: Option<ServiceScope>,
    pub name: Option<String>,
    pub implements: Option<String>,
    pub is_default: bool,
    pub tags: Vec<String>,
}

impl ServiceMetadata {
    pub fn new(type_name: String) -> Self {
        Self {
            type_name,
            lifetime: None,
            name: None,
            implements: None,
            is_default: false,
            tags: vec![],
        }
    }

    pub fn with_lifetime(mut self, lifetime: ServiceScope) -> Self {
        self.lifetime = Some(lifetime);
        self
    }

    pub fn with_name(mut self, name: String) -> Self {
        self.name = Some(name);
        self
    }

    pub fn implements(mut self, interface: String) -> Self {
        self.implements = Some(interface);
        self
    }

    pub fn as_default(mut self) -> Self {
        self.is_default = true;
        self
    }

    pub fn with_tag(mut self, tag: String) -> Self {
        self.tags.push(tag);
        self
    }
}

/// Service registry for storing discovered services
#[derive(Debug, Default)]
pub struct ServiceRegistry {
    services: HashMap<String, ServiceMetadata>,
    interfaces: HashMap<String, Vec<String>>, // interface -> [implementations]
}

impl ServiceRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a service
    pub fn register_service(&mut self, metadata: ServiceMetadata) {
        let type_name = metadata.type_name.clone();

        // Register interface binding if specified
        if let Some(interface) = &metadata.implements {
            self.interfaces
                .entry(interface.clone())
                .or_default()
                .push(type_name.clone());
        }

        self.services.insert(type_name, metadata);
    }

    /// Get service metadata
    pub fn get_service(&self, type_name: &str) -> Option<&ServiceMetadata> {
        self.services.get(type_name)
    }

    /// Get all implementations of an interface
    pub fn get_implementations(&self, interface: &str) -> Option<&Vec<String>> {
        self.interfaces.get(interface)
    }

    /// Get all registered services
    pub fn all_services(&self) -> &HashMap<String, ServiceMetadata> {
        &self.services
    }

    /// Apply conventions to fill in missing metadata
    pub fn apply_conventions(&mut self, conventions: &ServiceConventions) {
        for (type_name, metadata) in &mut self.services {
            // Apply lifetime convention if not specified
            if metadata.lifetime.is_none() {
                metadata.lifetime = Some(conventions.get_lifetime_for_type(type_name));
            }

            // Apply interface binding conventions
            if metadata.implements.is_none() && conventions.is_interface(type_name) {
                if let Some(impl_type) = conventions.find_implementation_for_interface(type_name) {
                    metadata.implements = Some(impl_type);
                }
            }
        }
    }
}

/// Example convention rule: Database-related services are scoped
pub struct DatabaseConventionRule;

impl ConventionRule for DatabaseConventionRule {
    fn get_lifetime(&self, type_name: &str) -> Option<ServiceScope> {
        if type_name.contains("Database")
            || type_name.contains("DbContext")
            || type_name.contains("Connection")
        {
            Some(ServiceScope::Scoped)
        } else {
            None
        }
    }
}

/// Example convention rule: Event handlers are transient
pub struct EventHandlerConventionRule;

impl ConventionRule for EventHandlerConventionRule {
    fn get_lifetime(&self, type_name: &str) -> Option<ServiceScope> {
        if type_name.ends_with("EventHandler")
            || type_name.ends_with("Handler")
            || type_name.contains("Event")
        {
            Some(ServiceScope::Transient)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_naming_conventions() {
        let conventions = ServiceConventions::new();

        assert_eq!(
            conventions.get_lifetime_for_type("UserService"),
            ServiceScope::Singleton
        );
        assert_eq!(
            conventions.get_lifetime_for_type("UserRepository"),
            ServiceScope::Scoped
        );
        assert_eq!(
            conventions.get_lifetime_for_type("PaymentFactory"),
            ServiceScope::Transient
        );
        assert_eq!(
            conventions.get_lifetime_for_type("SomeRandomType"),
            ServiceScope::Transient
        );
    }

    #[test]
    fn test_interface_patterns() {
        let conventions = ServiceConventions::new();

        assert!(conventions.is_interface("IUserService"));
        assert!(conventions.is_interface("PaymentTrait"));
        assert!(!conventions.is_interface("UserService"));
    }

    #[test]
    fn test_pattern_matching() {
        let conventions = ServiceConventions::new();

        assert!(conventions.matches_pattern("UserService", "*Service"));
        assert!(conventions.matches_pattern("IPaymentService", "I*"));
        assert!(conventions.matches_pattern("PaymentTrait", "*Trait"));
        assert!(conventions.matches_pattern("DatabaseFactory", "*Factory"));
        assert!(!conventions.matches_pattern("UserService", "*Repository"));
    }

    #[test]
    fn test_interface_implementation_discovery() {
        let conventions = ServiceConventions::new();

        assert_eq!(
            conventions.find_implementation_for_interface("IUserService"),
            Some("UserServiceImpl".to_string())
        );

        assert_eq!(
            conventions.find_implementation_for_interface("PaymentTrait"),
            Some("PaymentImpl".to_string())
        );
    }

    #[test]
    fn test_custom_convention_rules() {
        let mut conventions = ServiceConventions::new();
        conventions.add_custom_rule(DatabaseConventionRule);

        assert_eq!(
            conventions.get_lifetime_for_type("DatabaseService"),
            ServiceScope::Scoped
        );
        assert_eq!(
            conventions.get_lifetime_for_type("UserDbContext"),
            ServiceScope::Scoped
        );
        assert_eq!(
            conventions.get_lifetime_for_type("ConnectionPool"),
            ServiceScope::Scoped
        );
    }

    #[test]
    fn test_service_registry() {
        let mut registry = ServiceRegistry::new();

        let metadata = ServiceMetadata::new("UserService".to_string())
            .with_lifetime(ServiceScope::Singleton)
            .implements("IUserService".to_string())
            .as_default();

        registry.register_service(metadata);

        let service = registry.get_service("UserService").unwrap();
        assert_eq!(service.type_name, "UserService");
        assert_eq!(service.lifetime, Some(ServiceScope::Singleton));
        assert!(service.is_default);

        let implementations = registry.get_implementations("IUserService").unwrap();
        assert_eq!(implementations, &vec!["UserService".to_string()]);
    }

    #[test]
    fn test_apply_conventions_to_registry() {
        let mut registry = ServiceRegistry::new();
        let conventions = ServiceConventions::new();

        // Register service without explicit lifetime
        let metadata = ServiceMetadata::new("PaymentService".to_string());
        registry.register_service(metadata);

        // Apply conventions
        registry.apply_conventions(&conventions);

        let service = registry.get_service("PaymentService").unwrap();
        assert_eq!(service.lifetime, Some(ServiceScope::Singleton)); // *Service convention
    }
}
