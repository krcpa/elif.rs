//! Test that controller macro on enum fails

use elif_http_derive::controller;

// This should fail - controller macro should only work on structs
#[controller("/api")]
pub enum BadController {
    Variant,
}

fn main() {}