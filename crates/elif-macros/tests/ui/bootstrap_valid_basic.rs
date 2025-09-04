use elif::prelude::*;

// This should compile successfully - demonstrating basic usage
#[elif::bootstrap(AppModule)]
async fn main() -> Result<(), HttpError> {
    // This should compile successfully
}