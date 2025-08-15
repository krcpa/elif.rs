//! Controller Examples - Demonstrations of the elif-http controller system
//! 
//! This module contains example implementations showing how to use the
//! elif-http controller system with the ORM for full CRUD operations.

pub mod user_model;
pub mod user_controller;

pub use user_model::{User, CreateUserRequest, UpdateUserRequest};
pub use user_controller::{UserController, SearchParams, UserStats, setup_user_routes};