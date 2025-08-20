//! Test that #[group] macro fails with proper error messages on invalid syntax

use elif_http_derive::{group, get};

// This should fail with proper error message
#[group(invalid_syntax_here)]
impl BadGroup {
    #[get("/test")]
    pub fn test() -> String {
        "test".to_string()
    }
}

fn main() {}