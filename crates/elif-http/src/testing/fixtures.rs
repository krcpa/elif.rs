//! Test fixtures and utilities

use crate::{ElifRequest, ElifResponse, HttpConfig, HttpResult};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct TestUser {
    pub id: u32,
    pub name: String,
    pub email: String,
}

impl TestUser {
    pub fn new(id: u32, name: &str, email: &str) -> Self {
        Self {
            id,
            name: name.to_string(),
            email: email.to_string(),
        }
    }

    pub fn alice() -> Self {
        Self::new(1, "Alice", "alice@example.com")
    }

    pub fn bob() -> Self {
        Self::new(2, "Bob", "bob@example.com")
    }
}

#[derive(Deserialize)]
pub struct TestQuery {
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

/// Create a test HTTP configuration
pub fn test_http_config() -> HttpConfig {
    let mut config = HttpConfig::default();
    config.request_timeout_secs = 5;
    config.health_check_path = "/test-health".to_string();
    config
}

/// Sample handler for testing
pub async fn test_handler(_req: ElifRequest) -> HttpResult<ElifResponse> {
    Ok(ElifResponse::ok().text("test response"))
}

/// Sample JSON handler for testing
pub async fn test_json_handler(_req: ElifRequest) -> HttpResult<ElifResponse> {
    let user = TestUser::alice();
    ElifResponse::ok().json(&user)
}

/// Sample error handler for testing
pub async fn test_error_handler(_req: ElifRequest) -> HttpResult<ElifResponse> {
    Err(crate::errors::HttpError::bad_request("Test error"))
}
