//! Test that controller macro without arguments fails

use elif_http_derive::controller;

// This should fail - controller macro requires a path argument
#[controller]
pub struct BadController;

fn main() {}