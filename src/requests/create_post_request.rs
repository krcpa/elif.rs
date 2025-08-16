use serde::{Serialize, Deserialize};
use elif_validation::prelude::*;

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct CreatePostRequest {
    #[validate(required)]
    #[validate(length(min = 1, max = 255))]
    pub title: String,
    #[validate(required)]
    #[validate(length(min = 1, max = 255))]
    pub content: String,
    #[validate(required)]
    pub user_id: i32,
}
