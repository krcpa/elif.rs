//! Test simplified syntax without dyn keyword

use elif_http_derive::{module, app};

// Mock services
pub trait EmailService: Send + Sync {
    fn send_email(&self, _to: &str, _subject: &str) -> bool {
        true
    }
}

pub trait CacheService: Send + Sync {
    fn get(&self, _key: &str) -> Option<String> {
        None
    }
}

pub struct MockUserService;
pub struct SmtpEmailService;
pub struct RedisCacheService;
pub struct MockEmailService;
pub struct UserController;
pub struct PostController;

impl EmailService for SmtpEmailService {}
impl CacheService for RedisCacheService {}
impl EmailService for MockEmailService {}

// Test simplified trait mapping syntax without dyn
#[module(
    providers: [
        MockUserService,
        EmailService => SmtpEmailService,                    // No dyn needed!
        CacheService => RedisCacheService @ "redis"          // Named without dyn!
    ],
    controllers: [UserController, PostController]
)]
pub struct SimplifiedModule;

// Test mixed syntax (both dyn and without are supported)
#[module(
    providers: [
        MockUserService,
        EmailService => SmtpEmailService,                    // Simple form
        dyn CacheService => RedisCacheService               // Explicit dyn still works
    ]
)]
pub struct MixedSyntaxModule;

// Test application composition with simplified syntax
fn test_app_composition() {
    let _app = app! {
        modules: [SimplifiedModule, MixedSyntaxModule],
        overrides: [
            EmailService => MockEmailService @ "test"        // Simple override syntax
        ]
    };
}

fn main() {}