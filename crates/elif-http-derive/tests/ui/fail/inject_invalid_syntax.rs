//! Test that #[inject] with invalid syntax fails

use elif_http_derive::inject;

// This should fail - missing colon
#[inject(user_service UserService)]
pub struct InvalidSyntaxController;

fn main() {}