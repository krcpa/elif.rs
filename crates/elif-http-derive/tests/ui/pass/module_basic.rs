//! Basic module usage - should compile successfully

use elif_http_derive::module;

// Mock services
pub trait UserService: Send + Sync {
    fn get_user(&self, id: u32) -> String;
}

pub trait EmailService: Send + Sync {
    fn send_email(&self, to: &str, subject: &str) -> bool;
}

pub struct MockUserService;
pub struct SmtpEmailService;
pub struct UserController;
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

// Module with trait mappings
#[module(
    providers: [
        MockUserService,
        dyn EmailService => SmtpEmailService
    ],
    controllers: [UserController, PostController]
)]
pub struct TraitMappingModule;

// Module with named trait mappings
#[module(
    providers: [
        dyn EmailService => SmtpEmailService @ "smtp"
    ],
    controllers: [UserController]
)]
pub struct NamedMappingModule;

// Module with imports and exports
#[module(
    imports: [BasicModule],
    providers: [dyn EmailService => SmtpEmailService],
    exports: [dyn EmailService]
)]
pub struct ImportExportModule;

// Application composition tests will be added in Epic 4 (Runtime Integration)
// fn test_composition() {
//     let _app = module_composition! {
//         modules: [BasicModule, TraitMappingModule]
//     };
// }

fn main() {}