use elif::prelude::*;

#[elif::bootstrap(AppModule, invalid_param = "test")]
async fn main() -> Result<(), HttpError> {
    // This should fail because invalid_param is not a recognized parameter
}