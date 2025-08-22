//! Test cases that should fail compilation with token injection

use elif_http_derive::inject;

// This should fail: empty inject arguments
#[inject()]
struct EmptyInjectController;

fn main() {}