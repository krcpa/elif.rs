//! Test that HTTP method macros on structs fail

use elif_http_derive::get;

// This should fail - HTTP method macros should only work on functions
#[get("/test")]
pub struct BadStruct;

fn main() {}