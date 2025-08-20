use crate::event_error::EventError;
use crate::events::ModelObserver;
use crate::observers::ObserverManager;

pub struct ModelLifecycle {
    observer_manager: ObserverManager,
}

impl ModelLifecycle {
    pub fn new() -> Self {
        Self {
            observer_manager: ObserverManager::new(),
        }
    }

    pub fn register_observer<T: 'static>(&mut self, observer: Box<dyn ModelObserver<T>>) {
        self.observer_manager.register_for_model(observer);
    }

    pub fn register_global_observer<T: 'static>(&mut self, observer: Box<dyn ModelObserver<T>>) {
        self.observer_manager.register_global(observer);
    }

    pub async fn trigger_create_flow<T: 'static>(&self, model: &mut T) -> Result<(), EventError> {
        if let Some(registry) = self.observer_manager.get_registry_for::<T>() {
            // creating -> saving -> saved -> created
            registry.trigger_creating(model).await?;
            registry.trigger_saving(model).await?;
            registry.trigger_saved(model).await?;
            registry.trigger_created(model).await?;
        }
        Ok(())
    }

    pub async fn trigger_update_flow<T: 'static>(&self, model: &mut T, original: &T) -> Result<(), EventError> {
        if let Some(registry) = self.observer_manager.get_registry_for::<T>() {
            // updating -> saving -> saved -> updated
            registry.trigger_updating(model, original).await?;
            registry.trigger_saving(model).await?;
            registry.trigger_saved(model).await?;
            registry.trigger_updated(model, original).await?;
        }
        Ok(())
    }

    pub async fn trigger_delete_flow<T: 'static>(&self, model: &T) -> Result<(), EventError> {
        if let Some(registry) = self.observer_manager.get_registry_for::<T>() {
            // deleting -> deleted
            registry.trigger_deleting(model).await?;
            registry.trigger_deleted(model).await?;
        }
        Ok(())
    }

    pub fn has_observers_for<T: 'static>(&self) -> bool {
        self.observer_manager.has_observers_for::<T>()
    }
}

impl Default for ModelLifecycle {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use async_trait::async_trait;

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
    struct LifecycleTracker {
        events: Arc<Mutex<Vec<String>>>,
    }

    impl LifecycleTracker {
        fn new() -> Self {
            Self {
                events: Arc::new(Mutex::new(Vec::new())),
            }
        }

        fn track(&self, event: &str) {
            self.events.lock().unwrap().push(event.to_string());
        }

        fn get_events(&self) -> Vec<String> {
            self.events.lock().unwrap().clone()
        }

        #[allow(dead_code)]
        fn clear(&self) {
            self.events.lock().unwrap().clear();
        }
    }

    #[derive(Clone)]
    struct LifecycleObserver {
        tracker: LifecycleTracker,
    }

    impl LifecycleObserver {
        fn new(tracker: LifecycleTracker) -> Self {
            Self { tracker }
        }
    }

    #[async_trait]
    impl ModelObserver<TestUser> for LifecycleObserver {
        async fn creating(&self, model: &mut TestUser) -> Result<(), EventError> {
            self.tracker.track(&format!("creating: {}", model.name));
            // Simulate email normalization
            model.email = model.email.to_lowercase();
            Ok(())
        }

        async fn created(&self, model: &TestUser) -> Result<(), EventError> {
            self.tracker.track(&format!("created: {}", model.name));
            Ok(())
        }

        async fn updating(&self, model: &mut TestUser, original: &TestUser) -> Result<(), EventError> {
            self.tracker.track(&format!("updating: {} -> {}", original.name, model.name));
            Ok(())
        }

        async fn updated(&self, model: &TestUser, original: &TestUser) -> Result<(), EventError> {
            self.tracker.track(&format!("updated: {} -> {}", original.name, model.name));
            Ok(())
        }

        async fn saving(&self, model: &mut TestUser) -> Result<(), EventError> {
            self.tracker.track(&format!("saving: {}", model.name));
            Ok(())
        }

        async fn saved(&self, model: &TestUser) -> Result<(), EventError> {
            self.tracker.track(&format!("saved: {}", model.name));
            Ok(())
        }

        async fn deleting(&self, model: &TestUser) -> Result<(), EventError> {
            self.tracker.track(&format!("deleting: {}", model.name));
            Ok(())
        }

        async fn deleted(&self, model: &TestUser) -> Result<(), EventError> {
            self.tracker.track(&format!("deleted: {}", model.name));
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_model_lifecycle_create_flow() {
        let tracker = LifecycleTracker::new();
        let observer = LifecycleObserver::new(tracker.clone());
        
        let mut lifecycle = ModelLifecycle::new();
        lifecycle.register_observer::<TestUser>(Box::new(observer));
        
        let mut user = TestUser {
            name: "John Doe".to_string(),
            email: "JOHN@EXAMPLE.COM".to_string(),
            ..Default::default()
        };
        
        // Simulate create flow: creating -> saving -> saved -> created
        let result = lifecycle.trigger_create_flow(&mut user).await;
        assert!(result.is_ok());
        
        let events = tracker.get_events();
        assert_eq!(events.len(), 4);
        assert_eq!(events[0], "creating: John Doe");
        assert_eq!(events[1], "saving: John Doe");
        assert_eq!(events[2], "saved: John Doe");
        assert_eq!(events[3], "created: John Doe");
        
        // Check email was normalized
        assert_eq!(user.email, "john@example.com");
    }

    #[tokio::test]
    async fn test_model_lifecycle_update_flow() {
        let tracker = LifecycleTracker::new();
        let observer = LifecycleObserver::new(tracker.clone());
        
        let mut lifecycle = ModelLifecycle::new();
        lifecycle.register_observer::<TestUser>(Box::new(observer));
        
        let original = TestUser::default();
        let mut updated = TestUser {
            name: "Updated User".to_string(),
            ..original.clone()
        };
        
        // Simulate update flow: updating -> saving -> saved -> updated
        let result = lifecycle.trigger_update_flow(&mut updated, &original).await;
        assert!(result.is_ok());
        
        let events = tracker.get_events();
        assert_eq!(events.len(), 4);
        assert_eq!(events[0], "updating: Test User -> Updated User");
        assert_eq!(events[1], "saving: Updated User");
        assert_eq!(events[2], "saved: Updated User");
        assert_eq!(events[3], "updated: Test User -> Updated User");
    }

    #[tokio::test]
    async fn test_model_lifecycle_delete_flow() {
        let tracker = LifecycleTracker::new();
        let observer = LifecycleObserver::new(tracker.clone());
        
        let mut lifecycle = ModelLifecycle::new();
        lifecycle.register_observer::<TestUser>(Box::new(observer));
        
        let user = TestUser::default();
        
        // Simulate delete flow: deleting -> deleted
        let result = lifecycle.trigger_delete_flow(&user).await;
        assert!(result.is_ok());
        
        let events = tracker.get_events();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0], "deleting: Test User");
        assert_eq!(events[1], "deleted: Test User");
    }

    #[tokio::test]
    async fn test_model_lifecycle_error_stops_flow() {
        struct FailingObserver;
        
        #[async_trait]
        impl ModelObserver<TestUser> for FailingObserver {
            async fn creating(&self, _model: &mut TestUser) -> Result<(), EventError> {
                Err(EventError::validation("Creation not allowed"))
            }
        }
        
        let mut lifecycle = ModelLifecycle::new();
        lifecycle.register_observer::<TestUser>(Box::new(FailingObserver));
        
        let mut user = TestUser::default();
        
        let result = lifecycle.trigger_create_flow(&mut user).await;
        assert!(result.is_err());
        
        match result.unwrap_err() {
            EventError::Validation { message, .. } => {
                assert_eq!(message, "Creation not allowed");
            }
            _ => panic!("Expected validation error"),
        }
    }

    #[tokio::test]
    async fn test_model_lifecycle_multiple_observers() {
        let tracker1 = LifecycleTracker::new();
        let tracker2 = LifecycleTracker::new();
        
        let observer1 = LifecycleObserver::new(tracker1.clone());
        let observer2 = LifecycleObserver::new(tracker2.clone());
        
        let mut lifecycle = ModelLifecycle::new();
        lifecycle.register_observer::<TestUser>(Box::new(observer1));
        lifecycle.register_observer::<TestUser>(Box::new(observer2));
        
        let mut user = TestUser::default();
        let result = lifecycle.trigger_create_flow(&mut user).await;
        assert!(result.is_ok());
        
        // Both observers should have been called
        let events1 = tracker1.get_events();
        let events2 = tracker2.get_events();
        
        assert_eq!(events1.len(), 4);
        assert_eq!(events2.len(), 4);
        
        // Both should have same event sequence
        assert_eq!(events1[0], "creating: Test User");
        assert_eq!(events2[0], "creating: Test User");
    }

    #[tokio::test]
    async fn test_model_lifecycle_observer_modification_persists() {
        struct NormalizingObserver;
        
        #[async_trait]
        impl ModelObserver<TestUser> for NormalizingObserver {
            async fn creating(&self, model: &mut TestUser) -> Result<(), EventError> {
                model.name = model.name.to_uppercase();
                model.email = model.email.to_lowercase();
                Ok(())
            }
        }
        
        let mut lifecycle = ModelLifecycle::new();
        lifecycle.register_observer::<TestUser>(Box::new(NormalizingObserver));
        
        let mut user = TestUser {
            name: "john doe".to_string(),
            email: "JOHN@EXAMPLE.COM".to_string(),
            ..Default::default()
        };
        
        let result = lifecycle.trigger_create_flow(&mut user).await;
        assert!(result.is_ok());
        
        // Check modifications persisted
        assert_eq!(user.name, "JOHN DOE");
        assert_eq!(user.email, "john@example.com");
    }

    #[tokio::test]
    async fn test_model_lifecycle_event_propagation_control() {
        struct PropagationStoppingObserver;
        
        #[async_trait]
        impl ModelObserver<TestUser> for PropagationStoppingObserver {
            async fn creating(&self, _model: &mut TestUser) -> Result<(), EventError> {
                Err(EventError::propagation_stopped("User decided to cancel"))
            }
        }
        
        let mut lifecycle = ModelLifecycle::new();
        lifecycle.register_observer::<TestUser>(Box::new(PropagationStoppingObserver));
        
        let mut user = TestUser::default();
        let result = lifecycle.trigger_create_flow(&mut user).await;
        
        assert!(result.is_err());
        match result.unwrap_err() {
            EventError::PropagationStopped { reason, .. } => {
                assert_eq!(reason, "User decided to cancel");
            }
            _ => panic!("Expected propagation stopped error"),
        }
    }
}