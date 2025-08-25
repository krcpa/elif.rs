// Example demonstrating IoC Phase 2: Constructor Injection & Auto-wiring

use crate::container::autowiring::{DependencyResolver, Injectable};
use crate::container::binding::ServiceBinder;
use crate::container::descriptor::ServiceId;
use crate::container::ioc_container::IocContainer;
use crate::errors::CoreError;
use std::sync::Arc;

/// Example: E-commerce Order Processing Service
///
/// This demonstrates how the new autowiring system works with:
/// - Constructor injection
/// - Automatic dependency resolution  
/// - Optional dependencies
/// - Complex service graphs

// Domain services
#[derive(Default)]
pub struct SqliteUserRepository {
    connection_string: String,
}

impl SqliteUserRepository {
    pub fn find_user(&self, id: u32) -> Option<User> {
        println!(
            "Finding user {} from SQLite at {}",
            id, self.connection_string
        );
        Some(User {
            id,
            name: format!("User {}", id),
            email: format!("user{}@example.com", id),
        })
    }
}

#[derive(Default)]
pub struct PostgresProductRepository;

impl PostgresProductRepository {
    pub fn find_product(&self, id: u32) -> Option<Product> {
        println!("Finding product {} from Postgres", id);
        Some(Product {
            id,
            name: format!("Product {}", id),
            price: 29.99,
        })
    }
}

#[derive(Default)]
pub struct SmtpEmailService {
    smtp_server: String,
}

impl SmtpEmailService {
    pub fn send_email(&self, to: &str, subject: &str, body: &str) -> Result<(), String> {
        println!(
            "Sending email via {} to {}: {} - {}",
            self.smtp_server, to, subject, body
        );
        Ok(())
    }
}

#[derive(Default)]
pub struct PaymentProcessor;

impl PaymentProcessor {
    pub fn process_payment(&self, amount: f64) -> Result<String, String> {
        println!("Processing payment of ${:.2}", amount);
        Ok(format!("payment_id_{}", (amount * 100.0) as u32))
    }
}

// Optional services
#[derive(Default)]
pub struct MetricsCollector;

impl MetricsCollector {
    pub fn record_metric(&self, name: &str, value: f64) {
        println!("Recording metric: {} = {}", name, value);
    }
}

// Domain models
#[derive(Debug)]
pub struct User {
    pub id: u32,
    pub name: String,
    pub email: String,
}

#[derive(Debug)]
pub struct Product {
    pub id: u32,
    pub name: String,
    pub price: f64,
}

#[derive(Debug)]
pub struct Order {
    pub user: User,
    pub product: Product,
    pub payment_id: String,
}

// Main business service with complex dependencies
pub struct OrderService {
    user_repo: Arc<SqliteUserRepository>,
    product_repo: Arc<PostgresProductRepository>,
    email_service: Arc<SmtpEmailService>,
    payment_processor: Arc<PaymentProcessor>,
    metrics: Option<Arc<MetricsCollector>>, // Optional dependency
}

impl OrderService {
    pub fn new(
        user_repo: Arc<SqliteUserRepository>,
        product_repo: Arc<PostgresProductRepository>,
        email_service: Arc<SmtpEmailService>,
        payment_processor: Arc<PaymentProcessor>,
        metrics: Option<Arc<MetricsCollector>>,
    ) -> Self {
        Self {
            user_repo,
            product_repo,
            email_service,
            payment_processor,
            metrics,
        }
    }

    pub fn create_order(&self, user_id: u32, product_id: u32) -> Result<Order, String> {
        // Record metrics if available
        if let Some(metrics) = &self.metrics {
            metrics.record_metric("order.created", 1.0);
        }

        // Resolve dependencies and create order
        let user = self.user_repo.find_user(user_id).ok_or("User not found")?;

        let product = self
            .product_repo
            .find_product(product_id)
            .ok_or("Product not found")?;

        // Process payment
        let payment_id = self.payment_processor.process_payment(product.price)?;

        // Send confirmation email
        self.email_service.send_email(
            &user.email,
            "Order Confirmation",
            &format!("Your order for {} has been confirmed!", product.name),
        )?;

        Ok(Order {
            user,
            product,
            payment_id,
        })
    }
}

// Injectable implementation for OrderService
impl Injectable for OrderService {
    fn dependencies() -> Vec<ServiceId> {
        vec![
            ServiceId::of::<SqliteUserRepository>(),
            ServiceId::of::<PostgresProductRepository>(),
            ServiceId::of::<SmtpEmailService>(),
            ServiceId::of::<PaymentProcessor>(),
            ServiceId::of::<MetricsCollector>(), // Optional dependency
        ]
    }

    fn create<R: DependencyResolver>(resolver: &R) -> Result<Self, CoreError> {
        let user_repo = resolver.resolve::<SqliteUserRepository>()?;
        let product_repo = resolver.resolve::<PostgresProductRepository>()?;
        let email_service = resolver.resolve::<SmtpEmailService>()?;
        let payment_processor = resolver.resolve::<PaymentProcessor>()?;
        let metrics = resolver.try_resolve::<MetricsCollector>(); // Optional

        Ok(OrderService::new(
            user_repo,
            product_repo,
            email_service,
            payment_processor,
            metrics,
        ))
    }
}

/// Example usage demonstrating the autowiring system
pub fn run_autowiring_example() -> Result<(), CoreError> {
    println!("🚀 IoC Phase 2: Constructor Injection & Auto-wiring Example");
    println!("============================================================");

    // Create and configure the IoC container
    let mut container = IocContainer::new();

    // Register all services
    container
        .bind::<SqliteUserRepository, SqliteUserRepository>()
        .bind::<PostgresProductRepository, PostgresProductRepository>()
        .bind::<SmtpEmailService, SmtpEmailService>()
        .bind::<PaymentProcessor, PaymentProcessor>()
        .bind::<MetricsCollector, MetricsCollector>() // Optional
        .bind_injectable::<OrderService>(); // Auto-wiring enabled

    // Build the container (validates dependencies)
    container.build()?;

    println!(
        "\n📦 Container built successfully with {} services",
        container.service_count()
    );

    // Resolve the order service - all dependencies automatically injected!
    let order_service = container.resolve_injectable::<OrderService>()?;

    println!("\n🎯 OrderService resolved with auto-wiring!");
    println!("   Dependencies automatically injected:");
    println!("   - SqliteUserRepository");
    println!("   - PostgresProductRepository");
    println!("   - SmtpEmailService");
    println!("   - PaymentProcessor");
    println!("   - MetricsCollector (optional)");

    // Use the service
    println!("\n📋 Creating order...");
    let order = order_service
        .create_order(1, 101)
        .map_err(|e| CoreError::InvalidServiceDescriptor { message: e })?;

    println!("\n✅ Order created successfully!");
    println!("   User: {} ({})", order.user.name, order.user.email);
    println!(
        "   Product: {} - ${:.2}",
        order.product.name, order.product.price
    );
    println!("   Payment ID: {}", order.payment_id);

    // Demonstrate optional dependency handling
    println!("\n🔧 Testing without optional dependency...");
    let mut container_minimal = IocContainer::new();
    container_minimal
        .bind::<SqliteUserRepository, SqliteUserRepository>()
        .bind::<PostgresProductRepository, PostgresProductRepository>()
        .bind::<SmtpEmailService, SmtpEmailService>()
        .bind::<PaymentProcessor, PaymentProcessor>();
    // Note: No MetricsCollector registered
    // Note: Not using bind_injectable for now due to dependency resolution issues

    container_minimal.build()?;

    // Manually create OrderService to test optional dependency
    let user_repo = container_minimal.resolve::<SqliteUserRepository>()?;
    let product_repo = container_minimal.resolve::<PostgresProductRepository>()?;
    let email_service = container_minimal.resolve::<SmtpEmailService>()?;
    let payment_processor = container_minimal.resolve::<PaymentProcessor>()?;
    let metrics = container_minimal.try_resolve::<MetricsCollector>();

    let order_service_minimal = OrderService::new(
        user_repo,
        product_repo,
        email_service,
        payment_processor,
        metrics,
    );
    println!("✅ OrderService created even without optional MetricsCollector!");

    let _order2 = order_service_minimal
        .create_order(2, 102)
        .map_err(|e| CoreError::InvalidServiceDescriptor { message: e })?;
    println!("✅ Order processing works without metrics service");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_autowiring_example() {
        run_autowiring_example().expect("Autowiring example should work");
    }

    #[test]
    fn test_dependency_resolution() {
        let deps = OrderService::dependencies();
        assert_eq!(deps.len(), 5);
        assert!(deps.contains(&ServiceId::of::<SqliteUserRepository>()));
        assert!(deps.contains(&ServiceId::of::<PostgresProductRepository>()));
        assert!(deps.contains(&ServiceId::of::<SmtpEmailService>()));
        assert!(deps.contains(&ServiceId::of::<PaymentProcessor>()));
        assert!(deps.contains(&ServiceId::of::<MetricsCollector>()));
    }

    #[test]
    fn test_optional_dependency_handling() {
        let mut container = IocContainer::new();

        // Register only required dependencies
        container
            .bind::<SqliteUserRepository, SqliteUserRepository>()
            .bind::<PostgresProductRepository, PostgresProductRepository>()
            .bind::<SmtpEmailService, SmtpEmailService>()
            .bind::<PaymentProcessor, PaymentProcessor>();

        container.build().unwrap();

        // Manually resolve dependencies and create service to test optional handling
        let user_repo = container.resolve::<SqliteUserRepository>().unwrap();
        let product_repo = container.resolve::<PostgresProductRepository>().unwrap();
        let email_service = container.resolve::<SmtpEmailService>().unwrap();
        let payment_processor = container.resolve::<PaymentProcessor>().unwrap();
        let metrics = container.try_resolve::<MetricsCollector>(); // Should be None

        assert!(
            metrics.is_none(),
            "MetricsCollector should not be available"
        );

        let order_service = OrderService::new(
            user_repo,
            product_repo,
            email_service,
            payment_processor,
            metrics,
        );
        let order = order_service.create_order(1, 101).unwrap();

        assert_eq!(order.user.id, 1);
        assert_eq!(order.product.id, 101);
        assert!(!order.payment_id.is_empty());
    }

    #[test]
    fn test_complex_dependency_graph() {
        let mut container = IocContainer::new();

        // Register all dependencies including optional ones
        container
            .bind::<SqliteUserRepository, SqliteUserRepository>()
            .bind::<PostgresProductRepository, PostgresProductRepository>()
            .bind::<SmtpEmailService, SmtpEmailService>()
            .bind::<PaymentProcessor, PaymentProcessor>()
            .bind::<MetricsCollector, MetricsCollector>()
            .bind_injectable::<OrderService>();

        container.build().unwrap();

        // Validate that all services can be resolved
        assert!(container.resolve::<SqliteUserRepository>().is_ok());
        assert!(container.resolve::<PostgresProductRepository>().is_ok());
        assert!(container.resolve::<SmtpEmailService>().is_ok());
        assert!(container.resolve::<PaymentProcessor>().is_ok());
        assert!(container.resolve::<MetricsCollector>().is_ok());
        assert!(container.resolve_injectable::<OrderService>().is_ok());
    }
}
