use elif::prelude::*;

#[elif::bootstrap(AppModule)]
fn main() -> Result<(), HttpError> {
    // This should fail because the function is not async
}