# Validation

Use `elif-validation` to validate fields and entire payloads with async validators. Compose field-level checks and cross-field rules.

Defining a validator
```rust
use elif_validation::{ValidateField, ValidateRequest, Validate, ValidationErrors, ValidationError};
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;

struct RegisterValidator;

#[async_trait]
impl ValidateField for RegisterValidator {
    async fn validate_field(&self, field: &str, value: &Value) -> Result<(), ValidationErrors> {
        if field == "email" && value.as_str().map(|s| !s.contains('@')).unwrap_or(true) {
            return Err(ValidationErrors::from_error(ValidationError::new("email", "Invalid email")));
        }
        Ok(())
    }
}

#[async_trait]
impl ValidateRequest for RegisterValidator {
    async fn validate_request(&self, data: &HashMap<String, Value>) -> Result<(), ValidationErrors> {
        if data.get("password").is_some() && data.get("password_confirmation").is_none() {
            return Err(ValidationErrors::from_error(ValidationError::new("password_confirmation", "Required")));
        }
        Ok(())
    }
}

// Blanket impl provides Validate for types implementing both trait halves
impl RegisterValidator {}
```

Using in a controller
```rust
use elif_http::{ElifRequest, ElifResponse, HttpResult};
use elif_http::response::response;
use serde_json::json;

pub async fn register(req: ElifRequest) -> HttpResult<ElifResponse> {
    let data: serde_json::Value = req.json()?;
    let map: std::collections::HashMap<String, serde_json::Value> = 
        serde_json::from_value(data.clone()).unwrap_or_default();

    if let Err(errors) = RegisterValidator.validate(&map).await {
        return response().validation_error(errors.to_json()).send();
    }

    response().json(json!({"ok": true})).send()
}
```
