//! Demo DSL Showcase - Laravel-style module system syntax
//! 
//! This example demonstrates the `demo_module!` macro which provides
//! simplified Laravel-inspired syntax for common module scenarios.

use elif_http_derive::demo_module;

// =============================================================================
// DOMAIN MODELS & SERVICES
// =============================================================================

// User domain
#[derive(Default)]
pub struct User {
    pub id: u32,
    pub name: String,
    pub email: String,
}

#[derive(Default)]
pub struct UserService {
    users: Vec<User>,
}

impl UserService {
    pub fn find_user(&self, id: u32) -> Option<&User> {
        self.users.iter().find(|u| u.id == id)
    }
}

// Email services  
#[derive(Default)]
pub struct EmailService;

impl EmailService {
    pub fn send_welcome_email(&self, user: &User) {
        println!("📧 Sending welcome email to {}", user.email);
    }
}

// Cache service
#[derive(Default)]
pub struct CacheService;

impl CacheService {
    pub fn get<T>(&self, _key: &str) -> Option<T> {
        None // Simplified implementation
    }
    
    pub fn set<T>(&mut self, _key: &str, _value: T) {
        println!("💾 Cached data");
    }
}

// Controllers
#[derive(Default)]
pub struct UserController;

#[derive(Default)]
pub struct PostController;

#[derive(Default)]
pub struct AdminController;

// =============================================================================
// DEMO DSL EXAMPLES
// =============================================================================

fn main() {
    println!("🚀 Demo DSL Showcase - Laravel-style Module System\n");
    
    // Example 1: Simple service registration
    println!("1. Basic Services Module:");
    let services_module = demo_module! {
        services: [UserService, EmailService]
    };
    println!("   Created module with {} providers\n", services_module.providers.len());
    
    // Example 2: Web controllers  
    println!("2. Web Controllers Module:");
    let web_module = demo_module! {
        services: [UserService],
        controllers: [UserController, PostController]
    };
    println!("   Created module with {} controllers\n", web_module.controllers.len());
    
    // Example 3: Full-featured module with middleware
    println!("3. Complete Blog Module:");
    let blog_module = demo_module! {
        services: [
            UserService,
            EmailService, 
            CacheService
        ],
        controllers: [
            UserController,
            PostController,
            AdminController  
        ],
        middleware: [
            "cors",           // Cross-origin resource sharing
            "logging",        // Request/response logging
            "auth",           // Authentication middleware
            "rate_limiting",  // API rate limiting
            "compression"     // Response compression
        ]
    };
    
    println!("   📦 Services: {}", blog_module.providers.len());
    println!("   🎮 Controllers: {}", blog_module.controllers.len());
    // Note: middleware is processed during expansion but not stored in descriptor
    println!("   🔧 Middleware: Applied during module creation");
    println!();
    
    // Example 4: Minimal module
    println!("4. Minimal Module:");
    let minimal = demo_module! {
        services: [CacheService]
    };
    println!("   Created minimal module with {} service\n", minimal.providers.len());
    
    // Example 5: Laravel-style comparison
    println!("5. Laravel vs elif.rs Comparison:");
    println!("   Laravel (PHP):");
    println!("   ```php");
    println!("   class BlogServiceProvider extends ServiceProvider {{");
    println!("       public function register() {{");
    println!("           $this->app->bind(UserService::class);");
    println!("           $this->app->bind(EmailService::class);"); 
    println!("       }}");
    println!("   }}");
    println!("   ```");
    println!();
    println!("   elif.rs (Rust):");
    println!("   ```rust");
    println!("   let module = demo_module! {{");
    println!("       services: [UserService, EmailService]");
    println!("   }};");
    println!("   ```");
    println!();
    
    // Show that modules are composable
    println!("6. Module Composition:");
    println!("   Modules can be used in larger application compositions:");
    println!("   ```rust");
    println!("   module_composition! {{");
    println!("       modules: [BlogModule, AuthModule, ApiModule]");
    println!("   }}");
    println!("   ```");
    println!();
    
    println!("✅ Demo DSL showcase completed!");
    println!();
    println!("Next steps:");
    println!("• Try the full #[module(...)] syntax for production code");
    println!("• Explore trait mappings and dependency injection");
    println!("• Read the migration guide to convert existing IoC code");
}

// =============================================================================
// ADVANCED EXAMPLES
// =============================================================================

/// Example showing when to graduate from demo DSL to full syntax
mod advanced_usage {
    use super::*;
    
    // When you need trait mappings, use the full syntax:
    pub trait Repository: Send + Sync {
        fn save(&self, entity: &str);
    }
    
    #[derive(Default)]
    pub struct SqlRepository;
    
    impl Repository for SqlRepository {
        fn save(&self, entity: &str) {
            println!("💾 Saving {} to SQL database", entity);
        }
    }
    
    // This requires full module syntax (not demo DSL):
    /*
    #[module(
        providers: [
            dyn Repository => SqlRepository,
            UserService
        ],
        controllers: [UserController],
        exports: [dyn Repository]
    )]
    pub struct AdvancedModule;
    */
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn demo_dsl_creates_valid_descriptors() {
        let module = demo_module! {
            services: [UserService, EmailService],
            controllers: [UserController]
        };
        
        assert_eq!(module.name, "DemoDslModule");
        assert_eq!(module.providers.len(), 2);
        assert_eq!(module.controllers.len(), 1);
    }
    
    #[test]
    fn minimal_module_works() {
        let module = demo_module! {
            services: [CacheService]
        };
        
        assert_eq!(module.providers.len(), 1);
        assert_eq!(module.controllers.len(), 0);
    }
}