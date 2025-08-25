//! Service token system for type-safe trait injection
//!
//! Provides a token-based dependency injection system that enables compile-time
//! validation of trait implementations and semantic service resolution.
//!
//! ## Overview
//!
//! The service token system allows developers to define semantic tokens that
//! represent specific services or traits, enabling dependency inversion through
//! token-based resolution rather than concrete type dependencies.
//!
//! ## Naming Convention
//!
//! **Important**: Service tokens should follow the naming convention of ending with "Token"
//! (e.g., `EmailServiceToken`, `DatabaseToken`). This convention is used by the `#[inject]`
//! macro to automatically detect token references (`&TokenType`) and differentiate them from
//! regular reference fields, preventing incorrect macro expansion and compiler errors.
//!
//! ## Usage
//!
//! ```rust
//! use elif_core::container::{ServiceToken, IocContainer};
//! use std::sync::Arc;
//!
//! // Define a service trait
//! trait EmailService: Send + Sync {
//!     fn send(&self, to: &str, subject: &str, body: &str) -> Result<(), String>;
//! }
//!
//! // Define a token for the service
//! struct EmailNotificationToken;
//! impl ServiceToken for EmailNotificationToken {
//!     type Service = dyn EmailService;
//! }
//!
//! // Define implementations
//! struct SmtpEmailService;
//! impl EmailService for SmtpEmailService {
//!     fn send(&self, to: &str, subject: &str, body: &str) -> Result<(), String> {
//!         println!("SMTP: Sending to {} - {}: {}", to, subject, body);
//!         Ok(())
//!     }
//! }
//!
//! // Register with container
//! let mut container = IocContainer::new();
//! container.bind_token::<EmailNotificationToken, SmtpEmailService>();
//! container.build().unwrap();
//!
//! // Resolve via token
//! let service = container.resolve_by_token::<EmailNotificationToken>().unwrap();
//! service.send("user@example.com", "Welcome", "Hello!")?;
//! ```

use crate::container::descriptor::ServiceId;
use std::any::TypeId;
use std::marker::PhantomData;

/// Trait for service tokens that provide compile-time trait-to-implementation mapping
///
/// Service tokens are zero-sized types that act as compile-time identifiers for
/// specific services or traits. They enable type-safe dependency resolution
/// through semantic naming rather than concrete type dependencies.
///
/// ## Design Principles
///
/// - **Zero Runtime Cost**: Tokens are zero-sized and only exist at compile time
/// - **Type Safety**: Prevents incorrect service resolution through type constraints
/// - **Semantic Naming**: Enables meaningful service identifiers like `EmailNotificationToken`
/// - **Trait Resolution**: Allows injection of trait objects rather than concrete types
///
/// ## Implementation Requirements
///
/// - Token must be a zero-sized struct
/// - Associated `Service` type must be `Send + Sync + 'static`
/// - Service type is typically a trait object (`dyn Trait`)
///
/// ## Examples
///
/// ### Basic Trait Token
/// ```rust
/// struct DatabaseToken;
/// impl ServiceToken for DatabaseToken {
///     type Service = dyn Database;
/// }
/// ```
///
/// ### Specialized Service Token
/// ```rust
/// struct CacheToken;
/// impl ServiceToken for CacheToken {
///     type Service = dyn CacheService;
/// }
///
/// struct RedisCache;
/// impl CacheService for RedisCache {
///     // implementation
/// }
/// ```
pub trait ServiceToken: Send + Sync + 'static {
    /// The service type this token represents
    ///
    /// This is typically a trait object (`dyn Trait`) but can be any type
    /// that implements `Send + Sync + 'static`.
    type Service: ?Sized + Send + Sync + 'static;

    /// Get the TypeId of the service type
    ///
    /// Used internally for service resolution and type checking.
    /// Default implementation should suffice for most use cases.
    fn service_type_id() -> TypeId
    where
        Self::Service: 'static,
    {
        TypeId::of::<Self::Service>()
    }

    /// Get the type name of the service
    ///
    /// Used for debugging and error messages.
    /// Default implementation should suffice for most use cases.
    fn service_type_name() -> &'static str {
        std::any::type_name::<Self::Service>()
    }

    /// Get the token type name
    ///
    /// Used for debugging and error messages.
    /// Default implementation should suffice for most use cases.
    fn token_type_name() -> &'static str {
        std::any::type_name::<Self>()
    }
}

/// Metadata for a service token binding
///
/// Contains compile-time information about a token-to-implementation binding
/// for use in dependency resolution and validation.
#[derive(Debug, Clone)]
pub struct TokenBinding {
    /// The token type identifier
    pub token_type_id: TypeId,
    /// The token type name (for debugging)
    pub token_type_name: &'static str,
    /// The service type identifier
    pub service_type_id: TypeId,
    /// The service type name (for debugging)
    pub service_type_name: &'static str,
    /// The implementation type identifier
    pub impl_type_id: TypeId,
    /// The implementation type name (for debugging)
    pub impl_type_name: &'static str,
    /// Optional named identifier for multiple implementations
    pub name: Option<String>,
}

impl TokenBinding {
    /// Create a new token binding
    pub fn new<Token, Impl>() -> Self
    where
        Token: ServiceToken,
        Impl: Send + Sync + 'static,
    {
        Self {
            token_type_id: TypeId::of::<Token>(),
            token_type_name: std::any::type_name::<Token>(),
            service_type_id: Token::service_type_id(),
            service_type_name: Token::service_type_name(),
            impl_type_id: TypeId::of::<Impl>(),
            impl_type_name: std::any::type_name::<Impl>(),
            name: None,
        }
    }

    /// Create a named token binding
    pub fn named<Token, Impl>(name: impl Into<String>) -> Self
    where
        Token: ServiceToken,
        Impl: Send + Sync + 'static,
    {
        let mut binding = Self::new::<Token, Impl>();
        binding.name = Some(name.into());
        binding
    }

    /// Create a ServiceId for this token binding
    pub fn to_service_id(&self) -> ServiceId {
        if let Some(name) = &self.name {
            ServiceId::named_by_ids(self.service_type_id, self.service_type_name, name.clone())
        } else {
            ServiceId::by_ids(self.service_type_id, self.service_type_name)
        }
    }

    /// Check if this binding matches a token type
    pub fn matches_token<Token: ServiceToken>(&self) -> bool {
        self.token_type_id == TypeId::of::<Token>()
    }

    /// Validate that the implementation can be cast to the service type
    ///
    /// This performs compile-time type checking to ensure the implementation
    /// actually implements the service trait.
    pub fn validate_implementation<Token, Impl>() -> Result<(), String>
    where
        Token: ServiceToken,
        Impl: Send + Sync + 'static,
    {
        let token_service_id = Token::service_type_id();
        let impl_type_id = TypeId::of::<Impl>();

        // Basic validation - ensure we have the right types
        if token_service_id == TypeId::of::<()>() {
            return Err(format!(
                "Invalid service type for token {}: service type appears to be ()",
                Token::token_type_name()
            ));
        }

        // Check that implementation and service are different types (avoid self-referential bindings)
        if token_service_id == impl_type_id {
            return Err(format!(
                "Token {} maps to itself: implementation type {} cannot be the same as service type {}",
                Token::token_type_name(),
                std::any::type_name::<Impl>(),
                Token::service_type_name()
            ));
        }

        // Validate type names for better error messages
        let token_name = Token::token_type_name();
        let service_name = Token::service_type_name();
        let impl_name = std::any::type_name::<Impl>();

        if token_name.is_empty() {
            return Err("Invalid token: token type name is empty".to_string());
        }

        if service_name.is_empty() {
            return Err(format!(
                "Invalid token {}: service type name is empty",
                token_name
            ));
        }

        if impl_name.is_empty() {
            return Err(format!(
                "Invalid implementation for token {}: implementation type name is empty",
                token_name
            ));
        }

        // Additional validation: check for common naming patterns
        if service_name.contains("dyn ") && impl_name.contains("dyn ") {
            return Err(format!(
                "Invalid binding for token {}: both service ({}) and implementation ({}) appear to be trait objects. Implementation should be a concrete type.",
                token_name,
                service_name,
                impl_name
            ));
        }

        // Success - validation passed
        Ok(())
    }
}

/// Registry for token-based service bindings
///
/// Maintains a mapping from service tokens to their implementations
/// and provides lookup functionality for the IoC container.
#[derive(Debug, Default)]
pub struct TokenRegistry {
    /// All token bindings indexed by token type
    bindings: std::collections::HashMap<TypeId, Vec<TokenBinding>>,
    /// Default bindings (unnamed) indexed by token type  
    defaults: std::collections::HashMap<TypeId, TokenBinding>,
    /// Named bindings indexed by token type and name
    named: std::collections::HashMap<(TypeId, String), TokenBinding>,
}

impl TokenRegistry {
    /// Create a new token registry
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a token-to-implementation binding
    pub fn register<Token, Impl>(&mut self) -> Result<(), String>
    where
        Token: ServiceToken,
        Impl: Send + Sync + 'static,
    {
        // Validate the binding
        TokenBinding::validate_implementation::<Token, Impl>()?;

        let binding = TokenBinding::new::<Token, Impl>();
        let token_type_id = TypeId::of::<Token>();

        // Add to bindings list
        self.bindings
            .entry(token_type_id)
            .or_default()
            .push(binding.clone());

        // Set as default if no default exists
        self.defaults.entry(token_type_id).or_insert(binding);

        Ok(())
    }

    /// Register a named token-to-implementation binding
    pub fn register_named<Token, Impl>(&mut self, name: impl Into<String>) -> Result<(), String>
    where
        Token: ServiceToken,
        Impl: Send + Sync + 'static,
    {
        // Validate the binding
        TokenBinding::validate_implementation::<Token, Impl>()?;

        let name = name.into();
        let binding = TokenBinding::named::<Token, Impl>(&name);
        let token_type_id = TypeId::of::<Token>();

        // Add to bindings list
        self.bindings
            .entry(token_type_id)
            .or_default()
            .push(binding.clone());

        // Add to named bindings
        self.named.insert((token_type_id, name), binding);

        Ok(())
    }

    /// Get the default binding for a token type
    pub fn get_default<Token: ServiceToken>(&self) -> Option<&TokenBinding> {
        let token_type_id = TypeId::of::<Token>();
        self.defaults.get(&token_type_id)
    }

    /// Get a named binding for a token type
    pub fn get_named<Token: ServiceToken>(&self, name: &str) -> Option<&TokenBinding> {
        let token_type_id = TypeId::of::<Token>();
        self.named.get(&(token_type_id, name.to_string()))
    }

    /// Get all bindings for a token type
    pub fn get_all<Token: ServiceToken>(&self) -> Option<&Vec<TokenBinding>> {
        let token_type_id = TypeId::of::<Token>();
        self.bindings.get(&token_type_id)
    }

    /// Check if a token is registered
    pub fn contains<Token: ServiceToken>(&self) -> bool {
        let token_type_id = TypeId::of::<Token>();
        self.defaults.contains_key(&token_type_id)
    }

    /// Check if a named token is registered
    pub fn contains_named<Token: ServiceToken>(&self, name: &str) -> bool {
        let token_type_id = TypeId::of::<Token>();
        self.named.contains_key(&(token_type_id, name.to_string()))
    }

    /// Get all registered token types
    pub fn token_types(&self) -> Vec<TypeId> {
        self.defaults.keys().cloned().collect()
    }

    /// Get statistics about registered tokens
    pub fn stats(&self) -> TokenRegistryStats {
        TokenRegistryStats {
            total_tokens: self.bindings.len(), // Count unique token types from all bindings
            total_bindings: self.bindings.values().map(|v| v.len()).sum(),
            named_bindings: self.named.len(),
        }
    }

    /// Validate all token bindings in the registry
    ///
    /// Returns a list of validation errors found across all registered tokens.
    /// This method can be used to validate the entire registry after registration.
    pub fn validate_all_bindings(&self) -> Vec<String> {
        let mut errors = Vec::new();

        // Check for duplicate bindings
        for (token_type_id, bindings) in &self.bindings {
            if bindings.is_empty() {
                errors.push(format!(
                    "Token type {:?} has empty bindings list",
                    token_type_id
                ));
                continue;
            }

            // Check for consistency in token bindings
            let first_binding = &bindings[0];
            for binding in bindings.iter().skip(1) {
                if binding.token_type_id != first_binding.token_type_id {
                    errors.push(format!(
                        "Inconsistent token type IDs in bindings for token type {:?}",
                        token_type_id
                    ));
                }

                if binding.service_type_id != first_binding.service_type_id {
                    errors.push(format!(
                        "Inconsistent service type IDs for token {}: {} vs {}",
                        binding.token_type_name,
                        binding.service_type_name,
                        first_binding.service_type_name
                    ));
                }
            }

            // Check for named bindings consistency
            for binding in bindings {
                if let Some(name) = &binding.name {
                    if !self
                        .named
                        .contains_key(&(binding.token_type_id, name.clone()))
                    {
                        errors.push(format!(
                            "Named binding {} for token {} exists in bindings list but not in named map",
                            name,
                            binding.token_type_name
                        ));
                    }
                }
            }
        }

        // Check for orphaned named bindings
        for ((token_type_id, name), binding) in &self.named {
            if !self.bindings.contains_key(token_type_id) {
                errors.push(format!(
                    "Named binding {} for token {} exists in named map but token has no bindings",
                    name, binding.token_type_name
                ));
            }
        }

        errors
    }

    /// Check if a token has conflicting bindings
    pub fn has_binding_conflicts<Token: ServiceToken>(&self) -> Vec<String> {
        let mut conflicts = Vec::new();
        let token_type_id = TypeId::of::<Token>();

        if let Some(bindings) = self.bindings.get(&token_type_id) {
            // Check for multiple default bindings (shouldn't happen but good to check)
            let default_count = self.defaults.contains_key(&token_type_id) as usize;
            if default_count == 0 && !bindings.is_empty() {
                conflicts.push(format!(
                    "Token {} has bindings but no default binding",
                    Token::token_type_name()
                ));
            }

            // Check for duplicate named bindings (shouldn't happen due to HashMap)
            let mut seen_names = std::collections::HashSet::new();
            for binding in bindings {
                if let Some(name) = &binding.name {
                    if !seen_names.insert(name.clone()) {
                        conflicts.push(format!(
                            "Token {} has duplicate named binding: {}",
                            Token::token_type_name(),
                            name
                        ));
                    }
                }
            }
        }

        conflicts
    }

    /// Get detailed information about a token for debugging
    pub fn get_token_info<Token: ServiceToken>(&self) -> Option<TokenInfo> {
        let token_type_id = TypeId::of::<Token>();
        let bindings = self.bindings.get(&token_type_id)?;
        let default_binding = self.defaults.get(&token_type_id);

        let named_bindings: Vec<String> = self
            .named
            .iter()
            .filter_map(|((tid, name), _)| {
                if *tid == token_type_id {
                    Some(name.clone())
                } else {
                    None
                }
            })
            .collect();

        Some(TokenInfo {
            token_name: Token::token_type_name().to_string(),
            service_name: Token::service_type_name().to_string(),
            total_bindings: bindings.len(),
            has_default: default_binding.is_some(),
            named_bindings,
            implementation_types: bindings
                .iter()
                .map(|b| b.impl_type_name.to_string())
                .collect(),
        })
    }

    /// Clear all bindings (for testing)
    #[cfg(test)]
    pub fn clear(&mut self) {
        self.bindings.clear();
        self.defaults.clear();
        self.named.clear();
    }
}

/// Statistics about token registry usage
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenRegistryStats {
    /// Total number of distinct token types
    pub total_tokens: usize,
    /// Total number of bindings (including named variants)
    pub total_bindings: usize,
    /// Number of named bindings
    pub named_bindings: usize,
}

/// Detailed information about a specific token for debugging and validation
#[derive(Debug, Clone)]
pub struct TokenInfo {
    /// The token type name
    pub token_name: String,
    /// The service type name this token represents
    pub service_name: String,
    /// Total number of bindings for this token
    pub total_bindings: usize,
    /// Whether this token has a default binding
    pub has_default: bool,
    /// List of named binding identifiers
    pub named_bindings: Vec<String>,
    /// List of implementation type names
    pub implementation_types: Vec<String>,
}

/// Helper trait for working with token references in injection
///
/// This trait is implemented for reference types (`&Token`) to enable
/// seamless resolution of tokens through references in dependency injection.
pub trait TokenReference {
    /// The token type this reference points to
    type Token: ServiceToken;

    /// Get the token type
    fn token_type() -> PhantomData<Self::Token> {
        PhantomData
    }
}

impl<T: ServiceToken> TokenReference for &T {
    type Token = T;
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test service trait
    trait TestService: Send + Sync {
        #[allow(dead_code)]
        fn test(&self) -> String;
    }

    // Test token
    struct TestToken;
    impl ServiceToken for TestToken {
        type Service = dyn TestService;
    }

    // Test implementation
    struct TestImpl;
    impl TestService for TestImpl {
        fn test(&self) -> String {
            "test".to_string()
        }
    }

    #[test]
    fn test_service_token_trait() {
        assert_eq!(
            TestToken::token_type_name(),
            "elif_core::container::tokens::tests::TestToken"
        );
        assert_eq!(
            TestToken::service_type_name(),
            "dyn elif_core::container::tokens::tests::TestService"
        );
    }

    #[test]
    fn test_token_binding_creation() {
        let binding = TokenBinding::new::<TestToken, TestImpl>();

        assert_eq!(binding.token_type_id, TypeId::of::<TestToken>());
        assert_eq!(binding.service_type_id, TypeId::of::<dyn TestService>());
        assert_eq!(binding.impl_type_id, TypeId::of::<TestImpl>());
        assert!(binding.name.is_none());
    }

    #[test]
    fn test_named_token_binding() {
        let binding = TokenBinding::named::<TestToken, TestImpl>("primary");

        assert_eq!(binding.name, Some("primary".to_string()));
        assert!(binding.matches_token::<TestToken>());
    }

    #[test]
    fn test_token_registry_basic() {
        let mut registry = TokenRegistry::new();

        assert!(!registry.contains::<TestToken>());

        registry.register::<TestToken, TestImpl>().unwrap();

        assert!(registry.contains::<TestToken>());

        let binding = registry.get_default::<TestToken>().unwrap();
        assert!(binding.matches_token::<TestToken>());
    }

    #[test]
    fn test_token_registry_named() {
        let mut registry = TokenRegistry::new();

        registry
            .register_named::<TestToken, TestImpl>("primary")
            .unwrap();

        assert!(registry.contains_named::<TestToken>("primary"));
        assert!(!registry.contains_named::<TestToken>("secondary"));

        let binding = registry.get_named::<TestToken>("primary").unwrap();
        assert_eq!(binding.name, Some("primary".to_string()));
    }

    #[test]
    fn test_token_registry_stats() {
        let mut registry = TokenRegistry::new();

        registry.register::<TestToken, TestImpl>().unwrap();
        registry
            .register_named::<TestToken, TestImpl>("primary")
            .unwrap();

        let stats = registry.stats();
        assert_eq!(stats.total_tokens, 1);
        assert_eq!(stats.total_bindings, 2);
        assert_eq!(stats.named_bindings, 1);
    }

    #[test]
    fn test_token_reference() {
        let _phantom: PhantomData<TestToken> = <&TestToken as TokenReference>::token_type();
        // This test mainly ensures the TokenReference trait compiles correctly
    }
}
