//! Test that #[inject] with empty arguments fails

use elif_http_derive::inject;

// This should fail - inject requires at least one service
#[inject()]
pub struct EmptyInjectController;

fn main() {}