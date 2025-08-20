//! Test that #[resource] macro fails when no path is provided

use elif_http_derive::resource;

#[resource]
fn missing_path() -> String {
    "Should fail".to_string()
}

fn main() {}