//! Integration tests for the token-based dependency injection system
//!
//! Tests the complete flow from token definition to service resolution,
//! validating that the ServiceToken trait, TokenRegistry, and IoC container
//! integration work correctly.

use elif_core::container::{
    tokens::TokenBinding, tokens::TokenRegistry, IocContainer, ServiceToken,
};

/// Test service trait
trait EmailService: Send + Sync {
    fn send(&self, to: &str, subject: &str, body: &str) -> Result<(), String>;
    fn get_provider(&self) -> &str;
}

/// SMTP implementation of EmailService
#[derive(Default)]
struct SmtpEmailService;

impl EmailService for SmtpEmailService {
    fn send(&self, to: &str, subject: &str, body: &str) -> Result<(), String> {
        println!("SMTP: Sending to {} - {}: {}", to, subject, body);
        Ok(())
    }

    fn get_provider(&self) -> &str {
        "smtp"
    }
}

/// SendGrid implementation of EmailService
#[derive(Default)]
struct SendGridEmailService;

impl EmailService for SendGridEmailService {
    fn send(&self, to: &str, subject: &str, body: &str) -> Result<(), String> {
        println!("SendGrid: Sending to {} - {}: {}", to, subject, body);
        Ok(())
    }

    fn get_provider(&self) -> &str {
        "sendgrid"
    }
}

/// Email notification token
struct EmailNotificationToken;

impl ServiceToken for EmailNotificationToken {
    type Service = dyn EmailService;
}

/// Marketing email token (for testing multiple tokens for same service)
struct MarketingEmailToken;

impl ServiceToken for MarketingEmailToken {
    type Service = dyn EmailService;
}

/// Test database service trait
trait DatabaseService: Send + Sync {
    fn query(&self, sql: &str) -> Result<Vec<String>, String>;
}

/// Postgres implementation
#[derive(Default)]
struct PostgresService;

impl DatabaseService for PostgresService {
    fn query(&self, sql: &str) -> Result<Vec<String>, String> {
        Ok(vec![format!("postgres result for: {}", sql)])
    }
}

/// Database token
struct DatabaseToken;

impl ServiceToken for DatabaseToken {
    type Service = dyn DatabaseService;
}

#[test]
fn test_service_token_trait_metadata() {
    // Test that the token traits provide correct metadata
    assert_eq!(
        EmailNotificationToken::token_type_name(),
        "token_injection_tests::EmailNotificationToken"
    );
    assert_eq!(
        EmailNotificationToken::service_type_name(),
        "dyn token_injection_tests::EmailService"
    );

    assert_eq!(
        DatabaseToken::token_type_name(),
        "token_injection_tests::DatabaseToken"
    );
    assert_eq!(
        DatabaseToken::service_type_name(),
        "dyn token_injection_tests::DatabaseService"
    );
}

#[test]
fn test_token_registry_basic_operations() {
    let mut registry = TokenRegistry::new();

    // Initially no tokens registered
    assert!(!registry.contains::<EmailNotificationToken>());
    assert!(!registry.contains::<DatabaseToken>());

    // Register tokens
    registry
        .register::<EmailNotificationToken, SmtpEmailService>()
        .unwrap();
    registry
        .register::<DatabaseToken, PostgresService>()
        .unwrap();

    // Now tokens should be registered
    assert!(registry.contains::<EmailNotificationToken>());
    assert!(registry.contains::<DatabaseToken>());

    // Check bindings
    let email_binding = registry.get_default::<EmailNotificationToken>().unwrap();
    assert!(email_binding.matches_token::<EmailNotificationToken>());
    assert_eq!(
        email_binding.token_type_name,
        "token_injection_tests::EmailNotificationToken"
    );
    assert_eq!(
        email_binding.impl_type_name,
        "token_injection_tests::SmtpEmailService"
    );

    let db_binding = registry.get_default::<DatabaseToken>().unwrap();
    assert!(db_binding.matches_token::<DatabaseToken>());
    assert_eq!(
        db_binding.impl_type_name,
        "token_injection_tests::PostgresService"
    );
}

#[test]
fn test_named_token_registration() {
    let mut registry = TokenRegistry::new();

    // Register multiple implementations for the same token
    registry
        .register::<EmailNotificationToken, SmtpEmailService>()
        .unwrap();
    registry
        .register_named::<EmailNotificationToken, SendGridEmailService>("sendgrid")
        .unwrap();
    registry
        .register_named::<EmailNotificationToken, SmtpEmailService>("smtp_backup")
        .unwrap();

    // Check default binding
    assert!(registry.contains::<EmailNotificationToken>());
    let default_binding = registry.get_default::<EmailNotificationToken>().unwrap();
    assert_eq!(
        default_binding.impl_type_name,
        "token_injection_tests::SmtpEmailService"
    );

    // Check named bindings
    assert!(registry.contains_named::<EmailNotificationToken>("sendgrid"));
    assert!(registry.contains_named::<EmailNotificationToken>("smtp_backup"));
    assert!(!registry.contains_named::<EmailNotificationToken>("nonexistent"));

    let sendgrid_binding = registry
        .get_named::<EmailNotificationToken>("sendgrid")
        .unwrap();
    assert_eq!(
        sendgrid_binding.impl_type_name,
        "token_injection_tests::SendGridEmailService"
    );
    assert_eq!(sendgrid_binding.name, Some("sendgrid".to_string()));

    let smtp_backup_binding = registry
        .get_named::<EmailNotificationToken>("smtp_backup")
        .unwrap();
    assert_eq!(
        smtp_backup_binding.impl_type_name,
        "token_injection_tests::SmtpEmailService"
    );
    assert_eq!(smtp_backup_binding.name, Some("smtp_backup".to_string()));
}

#[test]
fn test_token_registry_stats() {
    let mut registry = TokenRegistry::new();

    let initial_stats = registry.stats();
    assert_eq!(initial_stats.total_tokens, 0);
    assert_eq!(initial_stats.total_bindings, 0);
    assert_eq!(initial_stats.named_bindings, 0);

    // Register some tokens
    registry
        .register::<EmailNotificationToken, SmtpEmailService>()
        .unwrap();
    registry
        .register_named::<EmailNotificationToken, SendGridEmailService>("sendgrid")
        .unwrap();
    registry
        .register::<DatabaseToken, PostgresService>()
        .unwrap();

    let stats = registry.stats();
    assert_eq!(stats.total_tokens, 2); // EmailNotificationToken and DatabaseToken
    assert_eq!(stats.total_bindings, 3); // Default email, named email, default database
    assert_eq!(stats.named_bindings, 1); // Only the sendgrid binding
}

#[test]
fn test_multiple_tokens_same_service() {
    let mut registry = TokenRegistry::new();

    // Register different tokens for the same service trait
    registry
        .register::<EmailNotificationToken, SmtpEmailService>()
        .unwrap();
    registry
        .register::<MarketingEmailToken, SendGridEmailService>()
        .unwrap();

    // Both tokens should be registered
    assert!(registry.contains::<EmailNotificationToken>());
    assert!(registry.contains::<MarketingEmailToken>());

    // Each should have different implementations
    let notification_binding = registry.get_default::<EmailNotificationToken>().unwrap();
    let marketing_binding = registry.get_default::<MarketingEmailToken>().unwrap();

    assert_eq!(
        notification_binding.impl_type_name,
        "token_injection_tests::SmtpEmailService"
    );
    assert_eq!(
        marketing_binding.impl_type_name,
        "token_injection_tests::SendGridEmailService"
    );
}

#[test]
fn test_ioc_container_token_binding() {
    let mut container = IocContainer::new();

    // Initially no tokens registered
    assert!(!container.contains_token::<EmailNotificationToken>());

    // Bind tokens
    container
        .bind_token::<EmailNotificationToken, SmtpEmailService>()
        .unwrap();
    container
        .bind_token::<DatabaseToken, PostgresService>()
        .unwrap();

    // Now tokens should be registered
    assert!(container.contains_token::<EmailNotificationToken>());
    assert!(container.contains_token::<DatabaseToken>());

    // Check token stats
    let stats = container.token_stats();
    assert_eq!(stats.total_tokens, 2);
    assert_eq!(stats.total_bindings, 2);
    assert_eq!(stats.named_bindings, 0);
}

#[test]
fn test_ioc_container_named_token_binding() {
    let mut container = IocContainer::new();

    // Bind named tokens
    container
        .bind_token_named::<EmailNotificationToken, SmtpEmailService>("smtp")
        .unwrap();
    container
        .bind_token_named::<EmailNotificationToken, SendGridEmailService>("sendgrid")
        .unwrap();

    // Check named token registration
    assert!(container.contains_token_named::<EmailNotificationToken>("smtp"));
    assert!(container.contains_token_named::<EmailNotificationToken>("sendgrid"));
    assert!(!container.contains_token_named::<EmailNotificationToken>("nonexistent"));

    // Check token stats
    let stats = container.token_stats();
    assert_eq!(stats.total_tokens, 1); // Only EmailNotificationToken type
    assert_eq!(stats.total_bindings, 2); // Two named bindings
    assert_eq!(stats.named_bindings, 2); // Both are named
}

#[test]
fn test_cannot_bind_tokens_after_build() {
    let mut container = IocContainer::new();

    // Build the container
    container.build().unwrap();

    // Now binding should fail
    let result = container.bind_token::<EmailNotificationToken, SmtpEmailService>();
    assert!(result.is_err());

    let error = result.unwrap_err().to_string();
    assert!(error.contains("Cannot bind tokens after container is built"));
}

// Note: Full resolution testing requires completing the trait object resolution
// implementation in the IoC container, which is marked as a placeholder.
// The following tests demonstrate the intended behavior but will currently
// return "trait object resolution not yet fully implemented" errors.

#[test]
#[ignore = "Requires full trait object resolution implementation"]
fn test_token_resolution() {
    let mut container = IocContainer::new();

    // Bind and build
    container
        .bind_token::<EmailNotificationToken, SmtpEmailService>()
        .unwrap();
    container.build().unwrap();

    // Resolve by token
    let service = container
        .resolve_by_token::<EmailNotificationToken>()
        .unwrap();
    assert_eq!(service.get_provider(), "smtp");

    // Test the service works
    let result = service.send("test@example.com", "Test", "Hello world!");
    assert!(result.is_ok());
}

#[test]
#[ignore = "Requires full trait object resolution implementation"]
fn test_named_token_resolution() {
    let mut container = IocContainer::new();

    // Bind multiple implementations
    container
        .bind_token_named::<EmailNotificationToken, SmtpEmailService>("smtp")
        .unwrap();
    container
        .bind_token_named::<EmailNotificationToken, SendGridEmailService>("sendgrid")
        .unwrap();
    container.build().unwrap();

    // Resolve different implementations
    let smtp_service = container
        .resolve_by_token_named::<EmailNotificationToken>("smtp")
        .unwrap();
    assert_eq!(smtp_service.get_provider(), "smtp");

    let sendgrid_service = container
        .resolve_by_token_named::<EmailNotificationToken>("sendgrid")
        .unwrap();
    assert_eq!(sendgrid_service.get_provider(), "sendgrid");
}

#[test]
#[ignore = "Requires full trait object resolution implementation"]
fn test_try_resolve_tokens() {
    let mut container = IocContainer::new();
    container.build().unwrap();

    // Should return None for unregistered token
    let result = container.try_resolve_by_token::<EmailNotificationToken>();
    assert!(result.is_none());

    // Should return None for unregistered named token
    let result = container.try_resolve_by_token_named::<EmailNotificationToken>("nonexistent");
    assert!(result.is_none());
}

#[test]
fn test_token_validation() {
    let mut registry = TokenRegistry::new();

    // Valid registration should succeed
    let result = registry.register::<EmailNotificationToken, SmtpEmailService>();
    assert!(result.is_ok());

    // Test that the validation logic exists (even if basic)
    let result = registry.register::<DatabaseToken, PostgresService>();
    assert!(result.is_ok());
}

/// Integration test showing the complete token-based DI flow
/// This demonstrates the intended usage pattern once trait resolution is complete
#[test]
#[ignore = "Requires full trait object resolution implementation"]
fn test_complete_token_workflow() {
    let mut container = IocContainer::new();

    // 1. Bind services to tokens
    container
        .bind_token::<EmailNotificationToken, SmtpEmailService>()
        .unwrap();
    container
        .bind_token::<DatabaseToken, PostgresService>()
        .unwrap();
    container
        .bind_token_named::<MarketingEmailToken, SendGridEmailService>("marketing")
        .unwrap();

    // 2. Build the container
    container.build().unwrap();

    // 3. Resolve services by token
    let notification_service = container
        .resolve_by_token::<EmailNotificationToken>()
        .unwrap();
    let database = container.resolve_by_token::<DatabaseToken>().unwrap();
    let marketing_service = container
        .resolve_by_token_named::<MarketingEmailToken>("marketing")
        .unwrap();

    // 4. Use the services
    notification_service
        .send("user@example.com", "Welcome", "Hello!")
        .unwrap();
    let results = database.query("SELECT * FROM users").unwrap();
    assert!(!results.is_empty());
    marketing_service
        .send("leads@example.com", "Promo", "Special offer!")
        .unwrap();

    // 5. Verify different implementations
    assert_eq!(notification_service.get_provider(), "smtp");
    assert_eq!(marketing_service.get_provider(), "sendgrid");
}

#[test]
fn test_enhanced_token_validation() {
    // Test the enhanced validation logic
    let result =
        TokenBinding::validate_implementation::<EmailNotificationToken, SmtpEmailService>();
    assert!(result.is_ok());

    // Test invalid type names (this would be difficult to create in practice)
    // The validation checks for empty type names, but Rust's type system prevents this

    // Test naming pattern validation
    let result = TokenBinding::validate_implementation::<DatabaseToken, PostgresService>();
    assert!(result.is_ok());
}

#[test]
fn test_registry_validation() {
    let mut registry = TokenRegistry::new();

    // Empty registry should have no errors
    let errors = registry.validate_all_bindings();
    assert!(errors.is_empty());

    // Add some bindings
    registry
        .register::<EmailNotificationToken, SmtpEmailService>()
        .unwrap();
    registry
        .register_named::<EmailNotificationToken, SendGridEmailService>("sendgrid")
        .unwrap();

    // Should still have no validation errors
    let errors = registry.validate_all_bindings();
    assert!(errors.is_empty(), "Validation errors found: {:?}", errors);

    // Check for binding conflicts
    let conflicts = registry.has_binding_conflicts::<EmailNotificationToken>();
    assert!(
        conflicts.is_empty(),
        "Binding conflicts found: {:?}",
        conflicts
    );
}

#[test]
fn test_token_info_debugging() {
    let mut registry = TokenRegistry::new();

    // Initially no token info available
    let info = registry.get_token_info::<EmailNotificationToken>();
    assert!(info.is_none());

    // Register some bindings
    registry
        .register::<EmailNotificationToken, SmtpEmailService>()
        .unwrap();
    registry
        .register_named::<EmailNotificationToken, SendGridEmailService>("sendgrid")
        .unwrap();

    // Now token info should be available
    let info = registry.get_token_info::<EmailNotificationToken>().unwrap();
    assert_eq!(
        info.token_name,
        "token_injection_tests::EmailNotificationToken"
    );
    assert!(info.service_name.contains("EmailService"));
    assert_eq!(info.total_bindings, 2);
    assert!(info.has_default);
    assert_eq!(info.named_bindings, vec!["sendgrid"]);
    assert_eq!(info.implementation_types.len(), 2);
    assert!(info
        .implementation_types
        .contains(&"token_injection_tests::SmtpEmailService".to_string()));
    assert!(info
        .implementation_types
        .contains(&"token_injection_tests::SendGridEmailService".to_string()));
}
