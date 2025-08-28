//! Test simplified module provider syntax

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

#[derive(Default)]
pub struct MockUserService;
#[derive(Default)]
pub struct SmtpEmailService;
#[derive(Default)]
pub struct RedisCacheService;
#[derive(Default)]
pub struct MockEmailService;
#[derive(Default)]
pub struct UserController;
#[derive(Default)]
pub struct PostController;

impl EmailService for SmtpEmailService {}
impl CacheService for RedisCacheService {}
impl EmailService for MockEmailService {}

// Test simplified provider syntax
#[module(
    providers: [
        MockUserService,
        SmtpEmailService,                    // Simple provider
        RedisCacheService          // Another simple provider
    ],
    controllers: [UserController, PostController]
)]
pub struct SimplifiedModule;

// Test mixed provider types
#[module(
    providers: [
        MockUserService,
        SmtpEmailService,                    // Simple form
        RedisCacheService               // Another simple form
    ]
)]
pub struct MixedSyntaxModule;

// Test application composition
fn test_app_composition() {
    let _app = app! {
        modules: [SimplifiedModule, MixedSyntaxModule],
        overrides: [
            MockEmailService        // Override service
        ]
    };
}

fn main() {}