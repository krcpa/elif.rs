//! UI tests for procedural macros using trybuild
//!
//! These tests verify that:
//! 1. Valid macro usage compiles successfully
//! 2. Invalid macro usage produces meaningful error messages

#[test]
fn ui_pass_tests() {
    let t = trybuild::TestCases::new();
    t.pass("tests/ui/pass/*.rs");
}

#[test]
fn ui_fail_tests() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/fail/*.rs");
}
