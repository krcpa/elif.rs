use elif::prelude::*;

#[elif::bootstrap(
    AppModule,
    addr = "0.0.0.0:8080"
)]
async fn main() -> Result<(), HttpError> {
    // This should compile successfully with address parameter
}