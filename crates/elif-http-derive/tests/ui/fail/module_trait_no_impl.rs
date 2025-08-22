//! Test trait provider without implementation

use elif_http_derive::module;

pub trait UserService: Send + Sync {}
pub struct UserController;

// Trait provider without implementation should cause error
#[module(
    providers: [dyn UserService],
    controllers: [UserController]
)]
pub struct TraitNoImplModule;

fn main() {}