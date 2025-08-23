//! UI test for Epic 3 module descriptor generation - should fail compilation

use elif_http_derive::module;

/// Test trait
pub trait EmailService: Send + Sync {
    fn send_email(&self, to: &str) -> Result<(), String>;
}

/// Module with invalid provider definition - trait without implementation
#[module(
    providers: [
        EmailService // Missing => Implementation  
    ],
    controllers: [],
    imports: [],
    exports: []
)]
pub struct InvalidModule;

fn main() {
    let _descriptor = InvalidModule::module_descriptor();
}