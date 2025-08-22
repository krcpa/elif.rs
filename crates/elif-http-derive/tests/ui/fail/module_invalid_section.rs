//! Test invalid module section names

use elif_http_derive::module;

pub struct UserService;
pub struct UserController;

// Invalid section name should cause error
#[module(
    invalid_section: [UserService],
    controllers: [UserController]
)]
pub struct InvalidSectionModule;

fn main() {}