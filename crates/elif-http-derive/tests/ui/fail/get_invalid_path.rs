//! Test that GET macro with invalid path argument fails

use elif_http_derive::get;

// This should fail - HTTP method macros expect string literals when args provided
#[get(invalid_path)]
pub async fn bad_handler() -> String {
    "Bad".to_string()
}

fn main() {}