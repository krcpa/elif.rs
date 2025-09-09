// Test that bootstrap macro parsing works correctly
// This test focuses on macro parsing, not compilation
#[elif::bootstrap]
async fn main() -> Result<(), HttpError> {
    // This should parse correctly (zero-boilerplate syntax)
}