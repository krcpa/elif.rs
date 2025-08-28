use elif_http_derive::module;

pub trait TestTrait: Send + Sync {}
pub struct TestImpl;
impl TestTrait for TestImpl {}

// This should work according to the docs
#[module(
    providers: [TestTrait => TestImpl]
)]
pub struct TestModule;

fn main() {}