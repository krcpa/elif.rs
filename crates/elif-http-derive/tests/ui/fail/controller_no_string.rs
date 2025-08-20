//! Test that controller macro without string argument fails

use elif_http_derive::controller;

// This should fail - controller macro requires a string literal
#[controller(invalid_path)]
pub struct BadController;

fn main() {}