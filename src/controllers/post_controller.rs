use elif_http::prelude::*;
use elif_core::ServiceContainer;
use crate::models::post::Post;
use crate::requests::post::{ CreatePostRequest, UpdatePostRequest };
use crate::resources::post::{ PostResource, PostCollection };
use std::sync::Arc;

#[controller]
pub struct PostController {
    container: Arc<ServiceContainer>,
}

impl PostController {
    pub fn new(container: Arc<ServiceContainer>) -> Self {
        Self { container }
    }

    // <<<ELIF:BEGIN agent-editable:post-index>>>
    pub async fn index(&self, request: Request) -> Result<Response, HttpError> {
        let query = Post::query();
        
        let posts = query.paginate(request.per_page()).await
            .map_err(|e| HttpError::internal_server_error(format!("Database error: {}", e)))?;
        
        Ok(Response::json(PostCollection::new(posts)))
    }
    // <<<ELIF:END agent-editable:post-index>>>

    // <<<ELIF:BEGIN agent-editable:post-show>>>
    pub async fn show(&self, request: Request) -> Result<Response, HttpError> {
        let id = request.path_param("id")
            .map_err(|_| HttpError::bad_request("Invalid ID parameter"))?;
        
        let post = Post::find(&id).await
            .map_err(|e| HttpError::internal_server_error(format!("Database error: {}", e)))?
            .ok_or_else(|| HttpError::not_found("Post not found"))?;
        
        Ok(Response::json(PostResource::new(post)))
    }
    // <<<ELIF:END agent-editable:post-show>>>

    // <<<ELIF:BEGIN agent-editable:post-store>>>
    pub async fn store(&self, mut request: Request) -> Result<Response, HttpError> {
        
        let data: CreatePostRequest = request.validate_json()
            .map_err(|e| HttpError::unprocessable_entity(format!("Validation error: {}", e)))?;
        
        let post = Post {
            title: data.title,
            content: data.content,
            user_id: data.user_id,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        let saved_post = post.save().await
            .map_err(|e| HttpError::internal_server_error(format!("Database error: {}", e)))?;
        
        Ok(Response::json(PostResource::new(saved_post)).status(201))
    }
    // <<<ELIF:END agent-editable:post-store>>>

    // <<<ELIF:BEGIN agent-editable:post-update>>>
    pub async fn update(&self, mut request: Request) -> Result<Response, HttpError> {
        let id = request.path_param("id")
            .map_err(|_| HttpError::bad_request("Invalid ID parameter"))?;
        
        
        let mut post = Post::find(&id).await
            .map_err(|e| HttpError::internal_server_error(format!("Database error: {}", e)))?
            .ok_or_else(|| HttpError::not_found("Post not found"))?;
        
        
        let data: UpdatePostRequest = request.validate_json()
            .map_err(|e| HttpError::unprocessable_entity(format!("Validation error: {}", e)))?;
        
        // Update fields
        title = data.title;
        content = data.content;
        user_id = data.user_id;
        post.updated_at = chrono::Utc::now();

        let updated_post = post.save().await
            .map_err(|e| HttpError::internal_server_error(format!("Database error: {}", e)))?;
        
        Ok(Response::json(PostResource::new(updated_post)))
    }
    // <<<ELIF:END agent-editable:post-update>>>

    // <<<ELIF:BEGIN agent-editable:post-destroy>>>
    pub async fn destroy(&self, request: Request) -> Result<Response, HttpError> {
        let id = request.path_param("id")
            .map_err(|_| HttpError::bad_request("Invalid ID parameter"))?;
        
        
        let post = Post::find(&id).await
            .map_err(|e| HttpError::internal_server_error(format!("Database error: {}", e)))?
            .ok_or_else(|| HttpError::not_found("Post not found"))?;
        
        
        post.delete().await
            .map_err(|e| HttpError::internal_server_error(format!("Database error: {}", e)))?;
        
        Ok(Response::no_content())
    }
    // <<<ELIF:END agent-editable:post-destroy>>>
}
