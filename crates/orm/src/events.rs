use crate::event_error::EventError;
use async_trait::async_trait;

#[derive(Debug, Clone)]
pub enum ModelEvent<T> {
    Creating(T),
    Created(T),
    Updating(T, T), // (old, new)
    Updated(T, T),
    Saving(T),
    Saved(T),
    Deleting(T),
    Deleted(T),
}

#[async_trait]
pub trait ModelObserver<T>: Send + Sync {
    async fn creating(&self, _model: &mut T) -> Result<(), EventError> {
        Ok(())
    }

    async fn created(&self, _model: &T) -> Result<(), EventError> {
        Ok(())
    }

    async fn updating(&self, _model: &mut T, _original: &T) -> Result<(), EventError> {
        Ok(())
    }

    async fn updated(&self, _model: &T, _original: &T) -> Result<(), EventError> {
        Ok(())
    }

    async fn saving(&self, _model: &mut T) -> Result<(), EventError> {
        Ok(())
    }

    async fn saved(&self, _model: &T) -> Result<(), EventError> {
        Ok(())
    }

    async fn deleting(&self, _model: &T) -> Result<(), EventError> {
        Ok(())
    }

    async fn deleted(&self, _model: &T) -> Result<(), EventError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[derive(Debug, Clone, PartialEq)]
    struct TestUser {
        id: i64,
        name: String,
        email: String,
    }

    impl Default for TestUser {
        fn default() -> Self {
            Self {
                id: 1,
                name: "Test User".to_string(),
                email: "test@example.com".to_string(),
            }
        }
    }

    #[derive(Debug, Clone)]
    struct TestObserver {
        events: Arc<Mutex<Vec<String>>>,
    }

    impl TestObserver {
        fn new() -> Self {
            Self {
                events: Arc::new(Mutex::new(Vec::new())),
            }
        }

        fn get_events(&self) -> Vec<String> {
            self.events.lock().unwrap().clone()
        }
    }

    #[async_trait]
    impl ModelObserver<TestUser> for TestObserver {
        async fn creating(&self, model: &mut TestUser) -> Result<(), EventError> {
            self.events
                .lock()
                .unwrap()
                .push(format!("creating: {}", model.name));
            Ok(())
        }

        async fn created(&self, model: &TestUser) -> Result<(), EventError> {
            self.events
                .lock()
                .unwrap()
                .push(format!("created: {}", model.name));
            Ok(())
        }

        async fn updating(
            &self,
            model: &mut TestUser,
            original: &TestUser,
        ) -> Result<(), EventError> {
            self.events
                .lock()
                .unwrap()
                .push(format!("updating: {} -> {}", original.name, model.name));
            Ok(())
        }

        async fn updated(&self, model: &TestUser, original: &TestUser) -> Result<(), EventError> {
            self.events
                .lock()
                .unwrap()
                .push(format!("updated: {} -> {}", original.name, model.name));
            Ok(())
        }

        async fn saving(&self, model: &mut TestUser) -> Result<(), EventError> {
            self.events
                .lock()
                .unwrap()
                .push(format!("saving: {}", model.name));
            Ok(())
        }

        async fn saved(&self, model: &TestUser) -> Result<(), EventError> {
            self.events
                .lock()
                .unwrap()
                .push(format!("saved: {}", model.name));
            Ok(())
        }

        async fn deleting(&self, model: &TestUser) -> Result<(), EventError> {
            self.events
                .lock()
                .unwrap()
                .push(format!("deleting: {}", model.name));
            Ok(())
        }

        async fn deleted(&self, model: &TestUser) -> Result<(), EventError> {
            self.events
                .lock()
                .unwrap()
                .push(format!("deleted: {}", model.name));
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_model_event_enum_creation() {
        let user = TestUser::default();
        let event = ModelEvent::Creating(user.clone());

        match event {
            ModelEvent::Creating(model) => {
                assert_eq!(model.name, "Test User");
                assert_eq!(model.email, "test@example.com");
            }
            _ => panic!("Expected Creating event"),
        }
    }

    #[tokio::test]
    async fn test_observer_creating_event() {
        let observer = TestObserver::new();
        let mut user = TestUser::default();

        let result = observer.creating(&mut user).await;
        assert!(result.is_ok());

        let events = observer.get_events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], "creating: Test User");
    }

    #[tokio::test]
    async fn test_observer_created_event() {
        let observer = TestObserver::new();
        let user = TestUser::default();

        let result = observer.created(&user).await;
        assert!(result.is_ok());

        let events = observer.get_events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], "created: Test User");
    }

    #[tokio::test]
    async fn test_observer_updating_event() {
        let observer = TestObserver::new();
        let original = TestUser::default();
        let mut updated = TestUser {
            name: "Updated User".to_string(),
            ..original.clone()
        };

        let result = observer.updating(&mut updated, &original).await;
        assert!(result.is_ok());

        let events = observer.get_events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], "updating: Test User -> Updated User");
    }

    #[tokio::test]
    async fn test_observer_updated_event() {
        let observer = TestObserver::new();
        let original = TestUser::default();
        let updated = TestUser {
            name: "Updated User".to_string(),
            ..original.clone()
        };

        let result = observer.updated(&updated, &original).await;
        assert!(result.is_ok());

        let events = observer.get_events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], "updated: Test User -> Updated User");
    }

    #[tokio::test]
    async fn test_observer_saving_event() {
        let observer = TestObserver::new();
        let mut user = TestUser::default();

        let result = observer.saving(&mut user).await;
        assert!(result.is_ok());

        let events = observer.get_events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], "saving: Test User");
    }

    #[tokio::test]
    async fn test_observer_saved_event() {
        let observer = TestObserver::new();
        let user = TestUser::default();

        let result = observer.saved(&user).await;
        assert!(result.is_ok());

        let events = observer.get_events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], "saved: Test User");
    }

    #[tokio::test]
    async fn test_observer_deleting_event() {
        let observer = TestObserver::new();
        let user = TestUser::default();

        let result = observer.deleting(&user).await;
        assert!(result.is_ok());

        let events = observer.get_events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], "deleting: Test User");
    }

    #[tokio::test]
    async fn test_observer_deleted_event() {
        let observer = TestObserver::new();
        let user = TestUser::default();

        let result = observer.deleted(&user).await;
        assert!(result.is_ok());

        let events = observer.get_events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], "deleted: Test User");
    }

    #[tokio::test]
    async fn test_event_error_creation() {
        let error = EventError::validation("Test validation error");

        match error {
            EventError::Validation { message, .. } => {
                assert_eq!(message, "Test validation error");
            }
            _ => panic!("Expected validation error"),
        }
    }

    #[tokio::test]
    async fn test_observer_error_handling() {
        struct FailingObserver;

        #[async_trait]
        impl ModelObserver<TestUser> for FailingObserver {
            async fn creating(&self, _model: &mut TestUser) -> Result<(), EventError> {
                Err(EventError::validation("Email already exists"))
            }
        }

        let observer = FailingObserver;
        let mut user = TestUser::default();

        let result = observer.creating(&mut user).await;
        assert!(result.is_err());

        match result.unwrap_err() {
            EventError::Validation { message, .. } => {
                assert_eq!(message, "Email already exists");
            }
            _ => panic!("Expected validation error"),
        }
    }
}
