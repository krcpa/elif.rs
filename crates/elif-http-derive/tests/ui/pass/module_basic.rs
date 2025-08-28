//! Basic module usage - should compile successfully

use elif_http_derive::module;

// Mock services
pub trait UserService: Send + Sync {
    fn get_user(&self, id: u32) -> String;
}

pub trait EmailService: Send + Sync {
    fn send_email(&self, to: &str, subject: &str) -> bool;
}

#[derive(Default)]
pub struct MockUserService;
#[derive(Default)]
pub struct SmtpEmailService;
#[derive(Default)]
pub struct UserController;
#[derive(Default)]
pub struct PostController;

impl UserService for MockUserService {
    fn get_user(&self, id: u32) -> String {
        format!("User {}", id)
    }
}

impl EmailService for SmtpEmailService {
    fn send_email(&self, to: &str, subject: &str) -> bool {
        true
    }
}

// Basic module with providers and controllers
#[module(
    providers: [MockUserService],
    controllers: [UserController]
)]
pub struct BasicModule;

// Module with multiple providers
#[module(
    providers: [
        MockUserService,
        SmtpEmailService
    ],
    controllers: [UserController, PostController]
)]
pub struct MultipleProvidersModule;

// Module with single provider
#[module(
    providers: [
        SmtpEmailService
    ],
    controllers: [UserController]
)]
pub struct SingleProviderModule;

// Module with imports and exports
#[module(
    imports: [BasicModule],
    providers: [SmtpEmailService],
    exports: [SmtpEmailService]
)]
pub struct ImportExportModule;

// Application composition tests will be added in Epic 4 (Runtime Integration)
// fn test_composition() {
//     let _app = module_composition! {
//         modules: [BasicModule, TraitMappingModule]
//     };
// }

fn main() {}