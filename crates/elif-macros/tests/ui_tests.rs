//! UI tests for bootstrap macro
//!
//! These tests ensure the macro produces appropriate compile-time errors
//! for invalid usage patterns.

#[test]
fn ui() {
    let t = trybuild::TestCases::new();
    
    // Test error cases
    t.compile_fail("tests/ui/bootstrap_non_async.rs");
    t.compile_fail("tests/ui/bootstrap_non_result.rs");
    t.compile_fail("tests/ui/bootstrap_invalid_param.rs");
    
    // Test parsing cases (these will fail compilation but should parse correctly)
    t.compile_fail("tests/ui/bootstrap_parse_basic.rs");
    t.compile_fail("tests/ui/bootstrap_parse_full.rs");
}