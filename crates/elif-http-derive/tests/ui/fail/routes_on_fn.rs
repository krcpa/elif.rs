//! Test that #[routes] macro fails when applied to functions instead of impl blocks

use elif_http_derive::routes;

#[routes]
fn not_an_impl_block() {
    // This should fail compilation
}

fn main() {}