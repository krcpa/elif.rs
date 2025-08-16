use serde::{Serialize, Deserialize};
use elif_validation::prelude::*;

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct UpdatePostRequest {
    #[validate(length(max = 255))]
    pub title: Option<String>,
    #[validate(length(max = 255))]
    pub content: Option<String>,
    pub user_id: Option<i32>,
}
