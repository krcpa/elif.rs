//! Factory states for applying common model variations

use std::collections::HashMap;
use serde_json::{json, Value};
use chrono::{DateTime, Utc};
use crate::error::OrmResult;
use super::traits::FactoryState;
use super::fake_data::*;

/// Common user states
#[derive(Debug, Clone)]
pub struct ActiveState;

#[async_trait::async_trait]
impl<T> FactoryState<T> for ActiveState {
    async fn apply(&self, attributes: &mut HashMap<String, Value>) -> OrmResult<()> {
        attributes.insert("status".to_string(), json!("active"));
        attributes.insert("is_active".to_string(), json!(true));
        Ok(())
    }
    
    fn state_name(&self) -> &'static str {
        "Active"
    }
}

#[derive(Debug, Clone)]
pub struct InactiveState;

#[async_trait::async_trait]
impl<T> FactoryState<T> for InactiveState {
    async fn apply(&self, attributes: &mut HashMap<String, Value>) -> OrmResult<()> {
        attributes.insert("status".to_string(), json!("inactive"));
        attributes.insert("is_active".to_string(), json!(false));
        Ok(())
    }
    
    fn state_name(&self) -> &'static str {
        "Inactive"
    }
}

#[derive(Debug, Clone)]
pub struct PendingState;

#[async_trait::async_trait]
impl<T> FactoryState<T> for PendingState {
    async fn apply(&self, attributes: &mut HashMap<String, Value>) -> OrmResult<()> {
        attributes.insert("status".to_string(), json!("pending"));
        attributes.insert("is_verified".to_string(), json!(false));
        attributes.insert("verified_at".to_string(), Value::Null);
        Ok(())
    }
    
    fn state_name(&self) -> &'static str {
        "Pending"
    }
}

#[derive(Debug, Clone)]
pub struct VerifiedState;

#[async_trait::async_trait]
impl<T> FactoryState<T> for VerifiedState {
    async fn apply(&self, attributes: &mut HashMap<String, Value>) -> OrmResult<()> {
        attributes.insert("status".to_string(), json!("verified"));
        attributes.insert("is_verified".to_string(), json!(true));
        attributes.insert("verified_at".to_string(), json!(fake_datetime().to_rfc3339()));
        Ok(())
    }
    
    fn state_name(&self) -> &'static str {
        "Verified"
    }
}

/// Admin user state
#[derive(Debug, Clone)]
pub struct AdminState;

#[async_trait::async_trait]
impl<T> FactoryState<T> for AdminState {
    async fn apply(&self, attributes: &mut HashMap<String, Value>) -> OrmResult<()> {
        attributes.insert("role".to_string(), json!("admin"));
        attributes.insert("is_admin".to_string(), json!(true));
        attributes.insert("permissions".to_string(), json!(["read", "write", "delete", "admin"]));
        Ok(())
    }
    
    fn state_name(&self) -> &'static str {
        "Admin"
    }
}

/// Moderator user state
#[derive(Debug, Clone)]
pub struct ModeratorState;

#[async_trait::async_trait]
impl<T> FactoryState<T> for ModeratorState {
    async fn apply(&self, attributes: &mut HashMap<String, Value>) -> OrmResult<()> {
        attributes.insert("role".to_string(), json!("moderator"));
        attributes.insert("is_admin".to_string(), json!(false));
        attributes.insert("permissions".to_string(), json!(["read", "write", "moderate"]));
        Ok(())
    }
    
    fn state_name(&self) -> &'static str {
        "Moderator"
    }
}

/// Suspended user state
#[derive(Debug, Clone)]
pub struct SuspendedState;

#[async_trait::async_trait]
impl<T> FactoryState<T> for SuspendedState {
    async fn apply(&self, attributes: &mut HashMap<String, Value>) -> OrmResult<()> {
        attributes.insert("status".to_string(), json!("suspended"));
        attributes.insert("is_active".to_string(), json!(false));
        attributes.insert("suspended_at".to_string(), json!(fake_datetime().to_rfc3339()));
        attributes.insert("suspension_reason".to_string(), json!("Policy violation"));
        Ok(())
    }
    
    fn state_name(&self) -> &'static str {
        "Suspended"
    }
}

/// Published content state
#[derive(Debug, Clone)]
pub struct PublishedState;

#[async_trait::async_trait]
impl<T> FactoryState<T> for PublishedState {
    async fn apply(&self, attributes: &mut HashMap<String, Value>) -> OrmResult<()> {
        attributes.insert("status".to_string(), json!("published"));
        attributes.insert("is_published".to_string(), json!(true));
        attributes.insert("published_at".to_string(), json!(fake_datetime().to_rfc3339()));
        Ok(())
    }
    
    fn state_name(&self) -> &'static str {
        "Published"
    }
}

/// Draft content state
#[derive(Debug, Clone)]
pub struct DraftState;

#[async_trait::async_trait]
impl<T> FactoryState<T> for DraftState {
    async fn apply(&self, attributes: &mut HashMap<String, Value>) -> OrmResult<()> {
        attributes.insert("status".to_string(), json!("draft"));
        attributes.insert("is_published".to_string(), json!(false));
        attributes.insert("published_at".to_string(), Value::Null);
        Ok(())
    }
    
    fn state_name(&self) -> &'static str {
        "Draft"
    }
}

/// Archived content state
#[derive(Debug, Clone)]
pub struct ArchivedState;

#[async_trait::async_trait]
impl<T> FactoryState<T> for ArchivedState {
    async fn apply(&self, attributes: &mut HashMap<String, Value>) -> OrmResult<()> {
        attributes.insert("status".to_string(), json!("archived"));
        attributes.insert("is_archived".to_string(), json!(true));
        attributes.insert("archived_at".to_string(), json!(fake_datetime().to_rfc3339()));
        Ok(())
    }
    
    fn state_name(&self) -> &'static str {
        "Archived"
    }
}

/// Premium account state
#[derive(Debug, Clone)]
pub struct PremiumState;

#[async_trait::async_trait]
impl<T> FactoryState<T> for PremiumState {
    async fn apply(&self, attributes: &mut HashMap<String, Value>) -> OrmResult<()> {
        attributes.insert("account_type".to_string(), json!("premium"));
        attributes.insert("is_premium".to_string(), json!(true));
        attributes.insert("premium_expires_at".to_string(), json!(fake_future_datetime().to_rfc3339()));
        Ok(())
    }
    
    fn state_name(&self) -> &'static str {
        "Premium"
    }
}

/// Free account state
#[derive(Debug, Clone)]
pub struct FreeState;

#[async_trait::async_trait]
impl<T> FactoryState<T> for FreeState {
    async fn apply(&self, attributes: &mut HashMap<String, Value>) -> OrmResult<()> {
        attributes.insert("account_type".to_string(), json!("free"));
        attributes.insert("is_premium".to_string(), json!(false));
        attributes.insert("premium_expires_at".to_string(), Value::Null);
        Ok(())
    }
    
    fn state_name(&self) -> &'static str {
        "Free"
    }
}

/// Completed status state
#[derive(Debug, Clone)]
pub struct CompletedState;

#[async_trait::async_trait]
impl<T> FactoryState<T> for CompletedState {
    async fn apply(&self, attributes: &mut HashMap<String, Value>) -> OrmResult<()> {
        attributes.insert("status".to_string(), json!("completed"));
        attributes.insert("is_completed".to_string(), json!(true));
        attributes.insert("completed_at".to_string(), json!(fake_datetime().to_rfc3339()));
        attributes.insert("progress".to_string(), json!(100));
        Ok(())
    }
    
    fn state_name(&self) -> &'static str {
        "Completed"
    }
}

/// In progress status state
#[derive(Debug, Clone)]
pub struct InProgressState;

#[async_trait::async_trait]
impl<T> FactoryState<T> for InProgressState {
    async fn apply(&self, attributes: &mut HashMap<String, Value>) -> OrmResult<()> {
        attributes.insert("status".to_string(), json!("in_progress"));
        attributes.insert("is_completed".to_string(), json!(false));
        attributes.insert("started_at".to_string(), json!(fake_datetime().to_rfc3339()));
        attributes.insert("progress".to_string(), json!(random_range(1, 99)));
        Ok(())
    }
    
    fn state_name(&self) -> &'static str {
        "InProgress"
    }
}

/// Custom state builder for flexible state creation
#[derive(Debug, Clone)]
pub struct CustomState {
    modifications: HashMap<String, Value>,
    name: String,
}

impl CustomState {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            modifications: HashMap::new(),
            name: name.into(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }
    
    pub fn with(mut self, key: impl Into<String>, value: Value) -> Self {
        self.modifications.insert(key.into(), value);
        self
    }
    
    pub fn with_status(self, status: impl Into<String>) -> Self {
        self.with("status", json!(status.into()))
    }
    
    pub fn with_bool_flag(self, flag: impl Into<String>, value: bool) -> Self {
        self.with(flag.into(), json!(value))
    }
    
    pub fn with_timestamp(self, field: impl Into<String>, datetime: DateTime<Utc>) -> Self {
        self.with(field.into(), json!(datetime.to_rfc3339()))
    }
    
    pub fn with_null(self, field: impl Into<String>) -> Self {
        self.with(field.into(), Value::Null)
    }
}

#[async_trait::async_trait]
impl<T> FactoryState<T> for CustomState {
    async fn apply(&self, attributes: &mut HashMap<String, Value>) -> OrmResult<()> {
        for (key, value) in &self.modifications {
            attributes.insert(key.clone(), value.clone());
        }
        Ok(())
    }
    
    fn state_name(&self) -> &'static str {
        // This is a bit of a hack since we need a static string
        // In practice, custom states should implement their own state struct
        "Custom"
    }
}

/// Macro for creating custom states easily
#[macro_export]
macro_rules! factory_state {
    ($name:ident { $($field:ident: $value:expr),* $(,)? }) => {
        #[derive(Debug, Clone)]
        pub struct $name;
        
        #[async_trait::async_trait]
        impl<T> $crate::factory::FactoryState<T> for $name {
            async fn apply(&self, attributes: &mut std::collections::HashMap<String, serde_json::Value>) -> $crate::error::OrmResult<()> {
                $(
                    attributes.insert(stringify!($field).to_string(), serde_json::json!($value));
                )*
                Ok(())
            }
            
            fn state_name(&self) -> &'static str {
                stringify!($name)
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_active_state() {
        let state = ActiveState;
        let mut attributes = HashMap::new();
        
        FactoryState::<()>::apply(&state, &mut attributes).await.unwrap();
        
        assert_eq!(attributes.get("status").unwrap(), &json!("active"));
        assert_eq!(attributes.get("is_active").unwrap(), &json!(true));
        assert_eq!(FactoryState::<()>::state_name(&state), "Active");
    }

    #[tokio::test]
    async fn test_admin_state() {
        let state = AdminState;
        let mut attributes = HashMap::new();
        
        FactoryState::<()>::apply(&state, &mut attributes).await.unwrap();
        
        assert_eq!(attributes.get("role").unwrap(), &json!("admin"));
        assert_eq!(attributes.get("is_admin").unwrap(), &json!(true));
        assert!(attributes.get("permissions").unwrap().is_array());
    }

    #[tokio::test]
    async fn test_verified_state() {
        let state = VerifiedState;
        let mut attributes = HashMap::new();
        
        FactoryState::<()>::apply(&state, &mut attributes).await.unwrap();
        
        assert_eq!(attributes.get("status").unwrap(), &json!("verified"));
        assert_eq!(attributes.get("is_verified").unwrap(), &json!(true));
        assert!(attributes.get("verified_at").unwrap().is_string());
    }

    #[tokio::test]
    async fn test_draft_state() {
        let state = DraftState;
        let mut attributes = HashMap::new();
        
        FactoryState::<()>::apply(&state, &mut attributes).await.unwrap();
        
        assert_eq!(attributes.get("status").unwrap(), &json!("draft"));
        assert_eq!(attributes.get("is_published").unwrap(), &json!(false));
        assert!(attributes.get("published_at").unwrap().is_null());
    }

    #[tokio::test]
    async fn test_custom_state() {
        let state = CustomState::new("test")
            .with("custom_field", json!("custom_value"))
            .with_status("custom_status")
            .with_bool_flag("custom_flag", true)
            .with_null("null_field");
            
        let mut attributes = HashMap::new();
        
        FactoryState::<()>::apply(&state, &mut attributes).await.unwrap();
        
        assert_eq!(attributes.get("custom_field").unwrap(), &json!("custom_value"));
        assert_eq!(attributes.get("status").unwrap(), &json!("custom_status"));
        assert_eq!(attributes.get("custom_flag").unwrap(), &json!(true));
        assert!(attributes.get("null_field").unwrap().is_null());
    }

    #[tokio::test] 
    async fn test_in_progress_state() {
        let state = InProgressState;
        let mut attributes = HashMap::new();
        
        FactoryState::<()>::apply(&state, &mut attributes).await.unwrap();
        
        assert_eq!(attributes.get("status").unwrap(), &json!("in_progress"));
        assert_eq!(attributes.get("is_completed").unwrap(), &json!(false));
        assert!(attributes.get("started_at").unwrap().is_string());
        
        let progress = attributes.get("progress").unwrap().as_i64().unwrap();
        assert!(progress >= 1 && progress <= 99);
    }

    // Test the macro
    factory_state!(TestMacroState {
        test_field: "test_value",
        test_bool: true,
        test_number: 42,
    });

    #[tokio::test]
    async fn test_macro_generated_state() {
        let state = TestMacroState;
        let mut attributes = HashMap::new();
        
        FactoryState::<()>::apply(&state, &mut attributes).await.unwrap();
        
        assert_eq!(attributes.get("test_field").unwrap(), &json!("test_value"));
        assert_eq!(attributes.get("test_bool").unwrap(), &json!(true));
        assert_eq!(attributes.get("test_number").unwrap(), &json!(42));
        assert_eq!(FactoryState::<()>::state_name(&state), "TestMacroState");
    }
}