use elif::prelude::*;

#[elif::bootstrap(AppModule)]
async fn main() {
    // This should fail because the function doesn't return Result
}