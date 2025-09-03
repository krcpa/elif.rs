//! Test that is_app: true causes a compilation error
//! is_app should only be used as a bare flag without a colon or value

use elif_http_derive::module;

#[derive(Default)]
pub struct TestService;

#[derive(Default)]
pub struct TestController;

// This should fail - is_app should be used without a value
#[module(
    providers: [TestService],
    controllers: [TestController],
    is_app: true
)]
pub struct BadAppModule;

fn main() {}