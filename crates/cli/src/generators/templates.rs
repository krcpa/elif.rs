pub const MODEL_TEMPLATE: &str = r#"use elif_orm::prelude::*;
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
{{#if has_uuid}}
use uuid::Uuid;
{{/if}}

#[derive(Model, Debug, Clone, Serialize, Deserialize, PartialEq)]
#[table_name = "{{snake_case table_name}}"]
pub struct {{pascal_case name}} {
    {{#each fields}}
    {{#if pk}}
    #[primary_key]
    {{/if}}
    {{#if index}}
    #[index]
    {{/if}}
    pub {{snake_case name}}: {{field_type}},
    {{/each}}
    {{#if timestamps}}
    
    #[timestamp]
    pub created_at: DateTime<Utc>,
    
    #[timestamp]
    pub updated_at: DateTime<Utc>,
    {{/if}}
    {{#if soft_delete}}
    
    #[soft_delete]
    pub deleted_at: Option<DateTime<Utc>>,
    {{/if}}
}

impl {{pascal_case name}} {
    // <<<ELIF:BEGIN agent-editable:{{snake_case name}}-model-methods>>>
    
    // Add your custom model methods here
    
    // <<<ELIF:END agent-editable:{{snake_case name}}-model-methods>>>
}

{{#if relationships}}
// Relationship definitions
impl {{pascal_case name}} {
    {{#each relationships}}
    {{#if (eq type "belongs_to")}}
    pub async fn {{snake_case related_model}}(&self) -> Result<{{pascal_case related_model}}, ModelError> {
        {{pascal_case ../name}}::query()
            .belongs_to::<{{pascal_case related_model}}>("{{snake_case related_model}}_id")
            .load_for(self)
            .await
    }
    {{/if}}
    
    {{#if (eq type "has_one")}}
    pub async fn {{snake_case related_model}}(&self) -> Result<Option<{{pascal_case related_model}}>, ModelError> {
        {{pascal_case related_model}}::query()
            .where_eq("{{snake_case ../name}}_id", &self.id)
            .first()
            .await
    }
    {{/if}}
    
    {{#if (eq type "has_many")}}
    pub async fn {{pluralize (snake_case related_model)}}(&self) -> Result<Vec<{{pascal_case related_model}}>, ModelError> {
        {{pascal_case related_model}}::query()
            .where_eq("{{snake_case ../name}}_id", &self.id)
            .load()
            .await
    }
    {{/if}}
    
    {{#if (eq type "belongs_to_many")}}
    pub async fn {{pluralize (snake_case related_model)}}(&self) -> Result<Vec<{{pascal_case related_model}}>, ModelError> {
        {{pascal_case ../name}}::query()
            .belongs_to_many::<{{pascal_case related_model}}>("{{snake_case ../name}}_{{pluralize (snake_case related_model)}}")
            .load_for(self)
            .await
    }
    {{/if}}
    {{/each}}
}
{{/if}}

#[cfg(test)]
mod tests {
    use super::*;
    use elif_testing::prelude::*;

    // <<<ELIF:BEGIN agent-editable:{{snake_case name}}-model-tests>>>
    
    #[test_database]
    async fn test_{{snake_case name}}_creation() {
        let {{snake_case name}} = {{pascal_case name}} {
            {{#each fields}}
            {{#unless pk}}
            {{snake_case name}}: {{#if (eq field_type "String")}}"test".to_string(){{else}}Default::default(){{/if}},
            {{/unless}}
            {{/each}}
            {{#if timestamps}}
            created_at: Utc::now(),
            updated_at: Utc::now(),
            {{/if}}
            {{#if soft_delete}}
            deleted_at: None,
            {{/if}}
        };

        let saved_{{snake_case name}} = {{snake_case name}}.save().await.unwrap();
        assert_eq!(saved_{{snake_case name}}.{{#each fields}}{{#if pk}}{{snake_case name}}{{/if}}{{/each}}, {{snake_case name}}.{{#each fields}}{{#if pk}}{{snake_case name}}{{/if}}{{/each}});
    }
    
    // <<<ELIF:END agent-editable:{{snake_case name}}-model-tests>>>
}
"#;

pub const CONTROLLER_TEMPLATE: &str = r#"use elif_http::prelude::*;
use elif_core::ServiceContainer;
use crate::models::{{snake_case name}}::{{pascal_case name}};
{{#if validation}}
use crate::requests::{{snake_case name}}::{ Create{{pascal_case name}}Request, Update{{pascal_case name}}Request };
use crate::resources::{{snake_case name}}::{ {{pascal_case name}}Resource, {{pascal_case name}}Collection };
{{/if}}
use std::sync::Arc;

#[controller]
pub struct {{pascal_case name}}Controller {
    container: Arc<ServiceContainer>,
}

impl {{pascal_case name}}Controller {
    pub fn new(container: Arc<ServiceContainer>) -> Self {
        Self { container }
    }

    // <<<ELIF:BEGIN agent-editable:{{snake_case name}}-index>>>
    pub async fn index(&self, request: Request) -> Result<Response, HttpError> {
        let query = {{pascal_case name}}::query(){{#if relationships}}{{#each relationships}}{{#if (eq type "belongs_to")}}
            .with("{{snake_case related_model}}"){{/if}}{{/each}}{{/if}};
        
        let {{pluralize (snake_case name)}} = query.paginate(request.per_page()).await
            .map_err(|e| HttpError::internal_server_error(format!("Database error: {}", e)))?;
        
        {{#if validation}}
        Ok(Response::json({{pascal_case name}}Collection::new({{pluralize (snake_case name)}})))
        {{else}}
        Ok(Response::json({{pluralize (snake_case name)}}))
        {{/if}}
    }
    // <<<ELIF:END agent-editable:{{snake_case name}}-index>>>

    // <<<ELIF:BEGIN agent-editable:{{snake_case name}}-show>>>
    pub async fn show(&self, request: Request) -> Result<Response, HttpError> {
        let id = request.path_param("id")
            .map_err(|_| HttpError::bad_request("Invalid ID parameter"))?;
        
        let {{snake_case name}} = {{pascal_case name}}::find(&id).await
            .map_err(|e| HttpError::internal_server_error(format!("Database error: {}", e)))?
            .ok_or_else(|| HttpError::not_found("{{pascal_case name}} not found"))?;
        
        {{#if validation}}
        Ok(Response::json({{pascal_case name}}Resource::new({{snake_case name}})))
        {{else}}
        Ok(Response::json({{snake_case name}}))
        {{/if}}
    }
    // <<<ELIF:END agent-editable:{{snake_case name}}-show>>>

    // <<<ELIF:BEGIN agent-editable:{{snake_case name}}-store>>>
    pub async fn store(&self, mut request: Request) -> Result<Response, HttpError> {
        {{#if auth}}
        let user = request.require_user()
            .map_err(|_| HttpError::unauthorized("Authentication required"))?;
        
        {{#if policy}}
        user.can("create", {{pascal_case name}}::class())
            .map_err(|_| HttpError::forbidden("Not authorized to create {{pluralize (lower name)}}"))?;
        {{/if}}
        {{/if}}
        
        {{#if validation}}
        let data: Create{{pascal_case name}}Request = request.validate_json()
            .map_err(|e| HttpError::unprocessable_entity(format!("Validation error: {}", e)))?;
        {{else}}
        let data: {{pascal_case name}} = request.json().await
            .map_err(|e| HttpError::bad_request(format!("Invalid JSON: {}", e)))?;
        {{/if}}
        
        let {{snake_case name}} = {{pascal_case name}} {
            {{#each fields}}
            {{#unless pk}}
            {{snake_case name}}: {{#if validation}}data.{{snake_case name}}{{else}}data.{{snake_case name}}{{/if}},
            {{/unless}}
            {{/each}}
            {{#if timestamps}}
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            {{/if}}
            {{#if soft_delete}}
            deleted_at: None,
            {{/if}}
        };

        let saved_{{snake_case name}} = {{snake_case name}}.save().await
            .map_err(|e| HttpError::internal_server_error(format!("Database error: {}", e)))?;
        
        {{#if validation}}
        Ok(Response::json({{pascal_case name}}Resource::new(saved_{{snake_case name}})).status(201))
        {{else}}
        Ok(Response::json(saved_{{snake_case name}}).status(201))
        {{/if}}
    }
    // <<<ELIF:END agent-editable:{{snake_case name}}-store>>>

    // <<<ELIF:BEGIN agent-editable:{{snake_case name}}-update>>>
    pub async fn update(&self, mut request: Request) -> Result<Response, HttpError> {
        let id = request.path_param("id")
            .map_err(|_| HttpError::bad_request("Invalid ID parameter"))?;
        
        {{#if auth}}
        let user = request.require_user()
            .map_err(|_| HttpError::unauthorized("Authentication required"))?;
        {{/if}}
        
        let mut {{snake_case name}} = {{pascal_case name}}::find(&id).await
            .map_err(|e| HttpError::internal_server_error(format!("Database error: {}", e)))?
            .ok_or_else(|| HttpError::not_found("{{pascal_case name}} not found"))?;
        
        {{#if policy}}
        user.can("update", &{{snake_case name}})
            .map_err(|_| HttpError::forbidden("Not authorized to update this {{lower name}}"))?;
        {{/if}}
        
        {{#if validation}}
        let data: Update{{pascal_case name}}Request = request.validate_json()
            .map_err(|e| HttpError::unprocessable_entity(format!("Validation error: {}", e)))?;
        {{else}}
        let data: {{pascal_case name}} = request.json().await
            .map_err(|e| HttpError::bad_request(format!("Invalid JSON: {}", e)))?;
        {{/if}}
        
        // Update fields
        {{#each fields}}
        {{#unless pk}}
        {{#unless (eq name "created_at")}}
        {{snake_case name}} = {{#if validation}}data.{{snake_case name}}{{else}}data.{{snake_case name}}{{/if}};
        {{/unless}}
        {{/unless}}
        {{/each}}
        {{#if timestamps}}
        {{snake_case name}}.updated_at = chrono::Utc::now();
        {{/if}}

        let updated_{{snake_case name}} = {{snake_case name}}.save().await
            .map_err(|e| HttpError::internal_server_error(format!("Database error: {}", e)))?;
        
        {{#if validation}}
        Ok(Response::json({{pascal_case name}}Resource::new(updated_{{snake_case name}})))
        {{else}}
        Ok(Response::json(updated_{{snake_case name}}))
        {{/if}}
    }
    // <<<ELIF:END agent-editable:{{snake_case name}}-update>>>

    // <<<ELIF:BEGIN agent-editable:{{snake_case name}}-destroy>>>
    pub async fn destroy(&self, request: Request) -> Result<Response, HttpError> {
        let id = request.path_param("id")
            .map_err(|_| HttpError::bad_request("Invalid ID parameter"))?;
        
        {{#if auth}}
        let user = request.require_user()
            .map_err(|_| HttpError::unauthorized("Authentication required"))?;
        {{/if}}
        
        let {{snake_case name}} = {{pascal_case name}}::find(&id).await
            .map_err(|e| HttpError::internal_server_error(format!("Database error: {}", e)))?
            .ok_or_else(|| HttpError::not_found("{{pascal_case name}} not found"))?;
        
        {{#if policy}}
        user.can("delete", &{{snake_case name}})
            .map_err(|_| HttpError::forbidden("Not authorized to delete this {{lower name}}"))?;
        {{/if}}
        
        {{#if soft_delete}}
        {{snake_case name}}.soft_delete().await
        {{else}}
        {{snake_case name}}.delete().await
        {{/if}}
            .map_err(|e| HttpError::internal_server_error(format!("Database error: {}", e)))?;
        
        Ok(Response::no_content())
    }
    // <<<ELIF:END agent-editable:{{snake_case name}}-destroy>>>
}
"#;

pub const MIGRATION_TEMPLATE: &str = r#"-- Migration: {{timestamp}}_create_{{pluralize (snake_case name)}}_table.sql
-- Created at: {{created_at}}

-- Up
CREATE TABLE {{pluralize (snake_case name)}} (
    {{#each fields}}
    {{snake_case name}} {{sql_type field_type}}{{#if pk}} PRIMARY KEY{{/if}}{{#if required}} NOT NULL{{/if}}{{#if default}} DEFAULT {{default}}{{/if}},
    {{/each}}
    {{#if timestamps}}
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    {{/if}}
    {{#if soft_delete}}
    deleted_at TIMESTAMPTZ,
    {{/if}}
);

{{#if indexes}}
-- Indexes
{{#each indexes}}
CREATE {{#if unique}}UNIQUE {{/if}}INDEX {{index_name}} ON {{table_name}} ({{columns}});
{{/each}}
{{/if}}

{{#if foreign_keys}}
-- Foreign key constraints
{{#each foreign_keys}}
ALTER TABLE {{../table_name}} ADD CONSTRAINT {{constraint_name}} 
    FOREIGN KEY ({{column}}) REFERENCES {{referenced_table}} ({{referenced_column}})
    {{#if on_delete}}ON DELETE {{on_delete}}{{/if}}
    {{#if on_update}}ON UPDATE {{on_update}}{{/if}};
{{/each}}
{{/if}}

-- Down
{{#if foreign_keys}}
-- Drop foreign key constraints
{{#each foreign_keys}}
ALTER TABLE {{../table_name}} DROP CONSTRAINT IF EXISTS {{constraint_name}};
{{/each}}
{{/if}}

{{#if indexes}}
-- Drop indexes
{{#each indexes}}
DROP INDEX IF EXISTS {{index_name}};
{{/each}}
{{/if}}

DROP TABLE IF EXISTS {{table_name}};
"#;

pub const TEST_TEMPLATE: &str = r#"use elif_testing::prelude::*;
use crate::models::{{snake_case name}}::{{pascal_case name}};
{{#if has_controller}}
use crate::controllers::{{snake_case name}}_controller::{{pascal_case name}}Controller;
{{/if}}

mod {{snake_case name}}_tests {
    use super::*;

    // <<<ELIF:BEGIN agent-editable:{{snake_case name}}-model-tests>>>
    
    #[test_database]
    async fn test_create_{{snake_case name}}() -> TestResult<()> {
        let {{snake_case name}} = {{pascal_case name}}Factory::new().create().await?;
        
        assert!(!{{snake_case name}}.{{#each fields}}{{#if pk}}{{snake_case name}}{{/if}}{{/each}}.is_nil());
        {{#each fields}}
        {{#unless pk}}
        {{#unless (eq field_type "DateTime<Utc>")}}
        // Assert {{snake_case name}} field
        {{/unless}}
        {{/unless}}
        {{/each}}
        
        Ok(())
    }

    #[test_database]
    async fn test_{{snake_case name}}_validation() -> TestResult<()> {
        // Test required field validation
        {{#each fields}}
        {{#if required}}
        {{#unless pk}}
        let result = {{pascal_case ../name}}::create({{pascal_case ../name}} {
            {{snake_case name}}: {{#if (eq field_type "String")}}String::new(){{else}}Default::default(){{/if}}, // Invalid empty value
            {{#each ../fields}}
            {{#unless (eq name ../name)}}
            {{#unless pk}}
            {{snake_case name}}: {{#if (eq field_type "String")}}"valid_value".to_string(){{else}}Default::default(){{/if}},
            {{/unless}}
            {{/unless}}
            {{/each}}
            ..Default::default()
        }).await;
        
        assert!(result.is_err());
        {{/unless}}
        {{/if}}
        {{/each}}
        
        Ok(())
    }
    
    {{#if relationships}}
    #[test_database]
    async fn test_{{snake_case name}}_relationships() -> TestResult<()> {
        let {{snake_case name}} = {{pascal_case name}}Factory::new().create().await?;
        
        {{#each relationships}}
        {{#if (eq type "has_many")}}
        // Test has_many relationship
        let {{pluralize (snake_case related_model)}} = {{pascal_case related_model}}Factory::new()
            .for_{{snake_case ../name}}({{snake_case ../name}}.id)
            .count(3)
            .create().await?;
            
        let loaded_{{pluralize (snake_case related_model)}} = {{snake_case ../name}}.{{pluralize (snake_case related_model)}}().await?;
        assert_eq!(loaded_{{pluralize (snake_case related_model)}}.len(), 3);
        {{/if}}
        
        {{#if (eq type "belongs_to")}}
        // Test belongs_to relationship  
        let {{snake_case related_model}} = {{pascal_case related_model}}Factory::new().create().await?;
        let {{snake_case ../name}} = {{pascal_case ../name}}Factory::new()
            .{{snake_case related_model}}_id({{snake_case related_model}}.id)
            .create().await?;
            
        let loaded_{{snake_case related_model}} = {{snake_case ../name}}.{{snake_case related_model}}().await?;
        assert_eq!(loaded_{{snake_case related_model}}.id, {{snake_case related_model}}.id);
        {{/if}}
        {{/each}}
        
        Ok(())
    }
    {{/if}}
    
    // <<<ELIF:END agent-editable:{{snake_case name}}-model-tests>>>
}

{{#if has_controller}}
mod {{snake_case name}}_controller_tests {
    use super::*;

    // <<<ELIF:BEGIN agent-editable:{{snake_case name}}-controller-tests>>>
    
    #[test_database]
    async fn test_{{snake_case name}}_index() -> TestResult<()> {
        let {{pluralize (snake_case name)}} = {{pascal_case name}}Factory::new().count(5).create().await?;
        
        let response = TestClient::new()
            .get("/api/{{pluralize (snake_case name)}}")
            .send()
            .await?;
            
        response.assert_status(200)
               .assert_json_length("data", 5);
        
        Ok(())
    }

    #[test_database] 
    async fn test_{{snake_case name}}_show() -> TestResult<()> {
        let {{snake_case name}} = {{pascal_case name}}Factory::new().create().await?;
        
        let response = TestClient::new()
            .get(&format!("/api/{{pluralize (snake_case name)}}/{}", {{snake_case name}}.id))
            .send()
            .await?;
            
        response.assert_status(200)
               .assert_json_contains(json!({"id": {{snake_case name}}.id}));
        
        Ok(())
    }

    #[test_database]
    async fn test_{{snake_case name}}_store() -> TestResult<()> {
        let {{snake_case name}}_data = json!({
            {{#each fields}}
            {{#unless pk}}
            {{#unless (eq field_type "DateTime<Utc>")}}
            "{{snake_case name}}": {{#if (eq field_type "String")}}"test_value"{{else}}{{#if (eq field_type "i32")}}42{{else}}{{#if (eq field_type "bool")}}true{{else}}null{{/if}}{{/if}}{{/if}},
            {{/unless}}
            {{/unless}}
            {{/each}}
        });
        
        let response = TestClient::new()
            .post("/api/{{pluralize (snake_case name)}}")
            .json(&{{snake_case name}}_data)
            .send()
            .await?;
            
        response.assert_status(201);
        
        // Verify created in database
        assert_database_has("{{pluralize (snake_case name)}}", |{{snake_case name}}: {{pascal_case name}}| {
            {{#each fields}}
            {{#unless pk}}
            {{#if (eq field_type "String")}}
            {{snake_case name}}.{{snake_case name}} == "test_value"{{#unless @last}} &&{{/unless}}
            {{/if}}
            {{/unless}}
            {{/each}}
        }).await?;
        
        Ok(())
    }

    #[test_database]
    async fn test_{{snake_case name}}_update() -> TestResult<()> {
        let {{snake_case name}} = {{pascal_case name}}Factory::new().create().await?;
        let update_data = json!({
            {{#each fields}}
            {{#unless pk}}
            {{#unless (eq name "created_at")}}
            {{#unless (eq field_type "DateTime<Utc>")}}
            "{{snake_case name}}": {{#if (eq field_type "String")}}"updated_value"{{else}}{{#if (eq field_type "i32")}}100{{else}}{{#if (eq field_type "bool")}}false{{else}}null{{/if}}{{/if}}{{/if}},
            {{/unless}}
            {{/unless}}
            {{/unless}}
            {{/each}}
        });
        
        let response = TestClient::new()
            .patch(&format!("/api/{{pluralize (snake_case name)}}/{}", {{snake_case name}}.id))
            .json(&update_data)
            .send()
            .await?;
            
        response.assert_status(200);
        
        Ok(())
    }

    #[test_database]
    async fn test_{{snake_case name}}_destroy() -> TestResult<()> {
        let {{snake_case name}} = {{pascal_case name}}Factory::new().create().await?;
        
        let response = TestClient::new()
            .delete(&format!("/api/{{pluralize (snake_case name)}}/{}", {{snake_case name}}.id))
            .send()
            .await?;
            
        response.assert_status(204);
        
        // Verify deleted from database
        {{#if soft_delete}}
        assert_database_missing("{{pluralize (snake_case name)}}", |{{snake_case name}}: {{pascal_case name}}| {
            {{snake_case name}}.id == {{snake_case name}}.id && {{snake_case name}}.deleted_at.is_some()
        }).await?;
        {{else}}
        assert_database_missing("{{pluralize (snake_case name)}}", |{{snake_case name}}: {{pascal_case name}}| {
            {{snake_case name}}.id == {{snake_case name}}.id
        }).await?;
        {{/if}}
        
        Ok(())
    }
    
    // <<<ELIF:END agent-editable:{{snake_case name}}-controller-tests>>>
}
{{/if}}

// Test factory for {{pascal_case name}}
#[factory]
pub struct {{pascal_case name}}Factory {
    {{#each fields}}
    {{#unless pk}}
    pub {{snake_case name}}: {{field_type}},
    {{/unless}}
    {{/each}}
    {{#if timestamps}}
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    {{/if}}
    {{#if soft_delete}}
    pub deleted_at: Option<DateTime<Utc>>,
    {{/if}}
}

impl {{pascal_case name}}Factory {
    {{#each relationships}}
    {{#if (eq type "belongs_to")}}
    pub fn {{snake_case related_model}}_id(mut self, {{snake_case related_model}}_id: {{pascal_case ../fields.0.field_type}}) -> Self {
        self.{{snake_case related_model}}_id = {{snake_case related_model}}_id;
        self
    }
    
    pub fn for_{{snake_case related_model}}(self, {{snake_case related_model}}: {{pascal_case related_model}}) -> Self {
        self.{{snake_case related_model}}_id({{snake_case related_model}}.id)
    }
    {{/if}}
    {{/each}}
}
"#;

pub const POLICY_TEMPLATE: &str = r#"use elif_auth::prelude::*;
use crate::models::{{snake_case name}}::{{pascal_case name}};

pub struct {{pascal_case name}}Policy;

impl {{pascal_case name}}Policy {
    // <<<ELIF:BEGIN agent-editable:{{snake_case name}}-policy-view-any>>>
    pub fn view_any(&self, user: &UserContext) -> bool {
        // Define who can view any {{pluralize (lower name)}}
        user.is_authenticated()
    }
    // <<<ELIF:END agent-editable:{{snake_case name}}-policy-view-any>>>

    // <<<ELIF:BEGIN agent-editable:{{snake_case name}}-policy-view>>>
    pub fn view(&self, user: &UserContext, {{snake_case name}}: &{{pascal_case name}}) -> bool {
        // Define who can view a specific {{lower name}}
        {{#if user_owned}}
        user.id() == Some({{snake_case name}}.user_id) || user.has_role("admin")
        {{else}}
        user.is_authenticated()
        {{/if}}
    }
    // <<<ELIF:END agent-editable:{{snake_case name}}-policy-view>>>

    // <<<ELIF:BEGIN agent-editable:{{snake_case name}}-policy-create>>>
    pub fn create(&self, user: &UserContext) -> bool {
        // Define who can create {{pluralize (lower name)}}
        {{#if user_owned}}
        user.is_authenticated()
        {{else}}
        user.has_permission("create:{{pluralize (snake_case name)}}")
        {{/if}}
    }
    // <<<ELIF:END agent-editable:{{snake_case name}}-policy-create>>>

    // <<<ELIF:BEGIN agent-editable:{{snake_case name}}-policy-update>>>
    pub fn update(&self, user: &UserContext, {{snake_case name}}: &{{pascal_case name}}) -> bool {
        // Define who can update a specific {{lower name}}
        {{#if user_owned}}
        user.id() == Some({{snake_case name}}.user_id) || user.has_role("admin")
        {{else}}
        user.has_permission("update:{{pluralize (snake_case name)}}")
        {{/if}}
    }
    // <<<ELIF:END agent-editable:{{snake_case name}}-policy-update>>>

    // <<<ELIF:BEGIN agent-editable:{{snake_case name}}-policy-delete>>>
    pub fn delete(&self, user: &UserContext, {{snake_case name}}: &{{pascal_case name}}) -> bool {
        // Define who can delete a specific {{lower name}}
        {{#if user_owned}}
        user.id() == Some({{snake_case name}}.user_id) || user.has_role("admin")
        {{else}}
        user.has_permission("delete:{{pluralize (snake_case name)}}")
        {{/if}}
    }
    // <<<ELIF:END agent-editable:{{snake_case name}}-policy-delete-->

    // <<<ELIF:BEGIN agent-editable:{{snake_case name}}-policy-restore>>>
    {{#if soft_delete}}
    pub fn restore(&self, user: &UserContext, {{snake_case name}}: &{{pascal_case name}}) -> bool {
        // Define who can restore a deleted {{lower name}}
        user.has_permission("restore:{{pluralize (snake_case name)}}")
    }
    {{/if}}
    // <<<ELIF:END agent-editable:{{snake_case name}}-policy-restore-->

    // <<<ELIF:BEGIN agent-editable:{{snake_case name}}-policy-force-delete>>>
    {{#if soft_delete}}
    pub fn force_delete(&self, user: &UserContext, {{snake_case name}}: &{{pascal_case name}}) -> bool {
        // Define who can permanently delete a {{lower name}}
        user.has_role("admin")
    }
    {{/if}}
    // <<<ELIF:END agent-editable:{{snake_case name}}-policy-force-delete-->
}

#[cfg(test)]
mod tests {
    use super::*;
    use elif_testing::prelude::*;

    // <<<ELIF:BEGIN agent-editable:{{snake_case name}}-policy-tests>>>
    
    #[test]
    fn test_view_any_policy() {
        let policy = {{pascal_case name}}Policy;
        
        let authenticated_user = UserContext::new(1, "user@example.com", vec![], vec![]);
        assert!(policy.view_any(&authenticated_user));
        
        let anonymous_user = UserContext::anonymous();
        assert!(!policy.view_any(&anonymous_user));
    }

    #[test]
    fn test_view_policy() {
        let policy = {{pascal_case name}}Policy;
        let {{snake_case name}} = {{pascal_case name}}Factory::new().build();
        
        {{#if user_owned}}
        let owner = UserContext::new({{snake_case name}}.user_id, "owner@example.com", vec![], vec![]);
        assert!(policy.view(&owner, &{{snake_case name}}));
        
        let other_user = UserContext::new(999, "other@example.com", vec![], vec![]);
        assert!(!policy.view(&other_user, &{{snake_case name}}));
        
        let admin = UserContext::new(2, "admin@example.com", vec!["admin".to_string()], vec![]);
        assert!(policy.view(&admin, &{{snake_case name}}));
        {{else}}
        let authenticated_user = UserContext::new(1, "user@example.com", vec![], vec![]);
        assert!(policy.view(&authenticated_user, &{{snake_case name}}));
        {{/if}}
    }

    #[test]
    fn test_create_policy() {
        let policy = {{pascal_case name}}Policy;
        
        {{#if user_owned}}
        let authenticated_user = UserContext::new(1, "user@example.com", vec![], vec![]);
        assert!(policy.create(&authenticated_user));
        
        let anonymous_user = UserContext::anonymous();
        assert!(!policy.create(&anonymous_user));
        {{else}}
        let user_with_permission = UserContext::new(1, "user@example.com", vec![], vec!["create:{{pluralize (snake_case name)}}".to_string()]);
        assert!(policy.create(&user_with_permission));
        
        let user_without_permission = UserContext::new(2, "user2@example.com", vec![], vec![]);
        assert!(!policy.create(&user_without_permission));
        {{/if}}
    }
    
    // <<<ELIF:END agent-editable:{{snake_case name}}-policy-tests>>>
}
"#;