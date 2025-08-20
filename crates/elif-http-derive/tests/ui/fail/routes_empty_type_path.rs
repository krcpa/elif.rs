//! Test that #[routes] macro fails gracefully with empty type paths

use elif_http_derive::routes;

// This should fail with a proper error message
#[routes]
impl <> {
    pub fn dummy() {}
}

fn main() {}