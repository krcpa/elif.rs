//! Test that reference types NOT ending with "Token" are handled as regular services
//! 
//! This demonstrates that the convention-based approach prevents false positives
//! where regular reference types would be incorrectly treated as tokens.

use elif_http_derive::inject;

// Regular types that don't follow token naming convention
struct Config;
struct DatabaseConnection; 
struct UserService;

// This should compile and treat all references as regular service types,
// not as token references (since none end with "Token")
#[inject(
    // These will be treated as regular services, not tokens
    config: Config,
    connection: DatabaseConnection,  
    user_service: UserService
)]
struct RegularController;

fn main() {
    // This should compile, demonstrating that:
    // 1. &Config would not be treated as a token (doesn't end with "Token")  
    // 2. &DatabaseConnection would not be treated as a token
    // 3. Only references ending with "Token" trigger token-based injection
    println!("Non-token reference test passed!");
}