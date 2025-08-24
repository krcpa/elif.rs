// ========== Original Elif.rs Command Modules ==========

pub mod create_simple;
pub use create_simple as create;
pub mod add;
pub mod inspect;
pub mod dev;
pub mod check;
pub mod doctor;
pub mod test;
pub mod db;
pub mod migrate;
pub mod build;
pub mod optimize;
pub mod deploy;
pub mod info;
pub mod status;
pub mod version;

// ========== Legacy modules (disabled to fix compilation) ==========
// These modules contain unused code and will be re-enabled as needed
// pub mod new;
// pub mod generate;
// pub mod route;
// pub mod model;
// pub mod resource;
// pub mod make;
// pub mod serve;
// pub mod queue;
// pub mod interactive_setup;
// pub mod database;
// pub mod email;
// pub mod api_version;
// pub mod auth;
// pub mod map;
// pub mod openapi;