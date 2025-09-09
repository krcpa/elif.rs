// Test that bootstrap macro parsing works with parameters
#[elif::bootstrap(addr = "0.0.0.0:8080")]
async fn main() -> Result<(), HttpError> {
    // This should parse correctly (auto-discovery with custom address)
}