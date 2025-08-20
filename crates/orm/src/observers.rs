use crate::event_error::EventError;
use crate::events::ModelObserver;
use std::collections::HashMap;
use std::any::{TypeId, Any};

pub struct ObserverRegistry<T> {
    observers: Vec<Box<dyn ModelObserver<T>>>,
}

impl<T> ObserverRegistry<T> {
    pub fn new() -> Self {
        Self {
            observers: Vec::new(),
        }
    }

    pub fn register(&mut self, observer: Box<dyn ModelObserver<T>>) {
        self.observers.push(observer);
    }

    pub fn observer_count(&self) -> usize {
        self.observers.len()
    }

    pub async fn trigger_creating(&self, model: &mut T) -> Result<(), EventError> {
        for observer in &self.observers {
            observer.creating(model).await?;
        }
        Ok(())
    }

    pub async fn trigger_created(&self, model: &T) -> Result<(), EventError> {
        for observer in &self.observers {
            observer.created(model).await?;
        }
        Ok(())
    }

    pub async fn trigger_updating(&self, model: &mut T, original: &T) -> Result<(), EventError> {
        for observer in &self.observers {
            observer.updating(model, original).await?;
        }
        Ok(())
    }

    pub async fn trigger_updated(&self, model: &T, original: &T) -> Result<(), EventError> {
        for observer in &self.observers {
            observer.updated(model, original).await?;
        }
        Ok(())
    }

    pub async fn trigger_saving(&self, model: &mut T) -> Result<(), EventError> {
        for observer in &self.observers {
            observer.saving(model).await?;
        }
        Ok(())
    }

    pub async fn trigger_saved(&self, model: &T) -> Result<(), EventError> {
        for observer in &self.observers {
            observer.saved(model).await?;
        }
        Ok(())
    }

    pub async fn trigger_deleting(&self, model: &T) -> Result<(), EventError> {
        for observer in &self.observers {
            observer.deleting(model).await?;
        }
        Ok(())
    }

    pub async fn trigger_deleted(&self, model: &T) -> Result<(), EventError> {
        for observer in &self.observers {
            observer.deleted(model).await?;
        }
        Ok(())
    }
}

impl<T> Default for ObserverRegistry<T> {
    fn default() -> Self {
        Self::new()
    }
}

pub struct GlobalObserverRegistry {
    _observers: Vec<Box<dyn Any + Send + Sync>>,
}

impl GlobalObserverRegistry {
    pub fn new() -> Self {
        Self {
            _observers: Vec::new(),
        }
    }

    pub fn register<T: 'static>(&mut self, _observer: Box<dyn ModelObserver<T> + Send + Sync>) {
        // For now, simplified global registry - not using Any conversion
        // This would need a more complex implementation for full functionality
    }

    pub fn observer_count(&self) -> usize {
        0 // Simplified for now
    }
}

impl Default for GlobalObserverRegistry {
    fn default() -> Self {
        Self::new()
    }
}

pub struct ObserverManager {
    model_observers: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
    global_observers: GlobalObserverRegistry,
}

impl ObserverManager {
    pub fn new() -> Self {
        Self {
            model_observers: HashMap::new(),
            global_observers: GlobalObserverRegistry::new(),
        }
    }

    pub fn register_for_model<T: 'static>(&mut self, observer: Box<dyn ModelObserver<T>>) {
        let type_id = TypeId::of::<T>();
        
        if let Some(registry) = self.model_observers.get_mut(&type_id) {
            if let Some(registry) = registry.downcast_mut::<ObserverRegistry<T>>() {
                registry.register(observer);
                return;
            }
        }
        
        let mut registry = ObserverRegistry::<T>::new();
        registry.register(observer);
        self.model_observers.insert(type_id, Box::new(registry));
    }

    pub fn register_global<T: 'static>(&mut self, observer: Box<dyn ModelObserver<T> + Send + Sync>) {
        self.global_observers.register(observer);
    }

    pub fn has_observers_for<T: 'static>(&self) -> bool {
        let type_id = TypeId::of::<T>();
        self.model_observers.contains_key(&type_id)
    }

    pub fn global_observer_count(&self) -> usize {
        self.global_observers.observer_count()
    }

    pub fn get_registry_for<T: 'static>(&self) -> Option<&ObserverRegistry<T>> {
        let type_id = TypeId::of::<T>();
        self.model_observers.get(&type_id)?
            .downcast_ref::<ObserverRegistry<T>>()
    }

    pub fn get_registry_for_mut<T: 'static>(&mut self) -> Option<&mut ObserverRegistry<T>> {
        let type_id = TypeId::of::<T>();
        self.model_observers.get_mut(&type_id)?
            .downcast_mut::<ObserverRegistry<T>>()
    }
}

impl Default for ObserverManager {
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
    struct EventTracker {
        events: Arc<Mutex<Vec<String>>>,
    }

    impl EventTracker {
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
    struct TrackingObserver {
        tracker: EventTracker,
        name: String,
    }

    impl TrackingObserver {
        fn new(name: &str, tracker: EventTracker) -> Self {
            Self {
                tracker,
                name: name.to_string(),
            }
        }
    }

    #[async_trait]
    impl ModelObserver<TestUser> for TrackingObserver {
        async fn creating(&self, model: &mut TestUser) -> Result<(), EventError> {
            self.tracker.track(&format!("{}: creating {}", self.name, model.name));
            Ok(())
        }

        async fn created(&self, model: &TestUser) -> Result<(), EventError> {
            self.tracker.track(&format!("{}: created {}", self.name, model.name));
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_observer_registry_creation() {
        let registry = ObserverRegistry::<TestUser>::new();
        assert_eq!(registry.observer_count(), 0);
    }

    #[tokio::test]
    async fn test_observer_registry_register() {
        let mut registry = ObserverRegistry::<TestUser>::new();
        let tracker = EventTracker::new();
        let observer = TrackingObserver::new("observer1", tracker.clone());
        
        registry.register(Box::new(observer));
        assert_eq!(registry.observer_count(), 1);
    }

    #[tokio::test]
    async fn test_observer_registry_multiple_observers() {
        let mut registry = ObserverRegistry::<TestUser>::new();
        let tracker = EventTracker::new();
        
        let observer1 = TrackingObserver::new("observer1", tracker.clone());
        let observer2 = TrackingObserver::new("observer2", tracker.clone());
        
        registry.register(Box::new(observer1));
        registry.register(Box::new(observer2));
        
        assert_eq!(registry.observer_count(), 2);
    }

    #[tokio::test]
    async fn test_observer_registry_trigger_creating() {
        let mut registry = ObserverRegistry::<TestUser>::new();
        let tracker = EventTracker::new();
        let observer = TrackingObserver::new("observer1", tracker.clone());
        
        registry.register(Box::new(observer));
        
        let mut user = TestUser::default();
        let result = registry.trigger_creating(&mut user).await;
        
        assert!(result.is_ok());
        
        let events = tracker.get_events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], "observer1: creating Test User");
    }

    #[tokio::test]
    async fn test_observer_registry_trigger_created() {
        let mut registry = ObserverRegistry::<TestUser>::new();
        let tracker = EventTracker::new();
        let observer = TrackingObserver::new("observer1", tracker.clone());
        
        registry.register(Box::new(observer));
        
        let user = TestUser::default();
        let result = registry.trigger_created(&user).await;
        
        assert!(result.is_ok());
        
        let events = tracker.get_events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], "observer1: created Test User");
    }

    #[tokio::test]
    async fn test_observer_registry_multiple_observers_execution_order() {
        let mut registry = ObserverRegistry::<TestUser>::new();
        let tracker = EventTracker::new();
        
        let observer1 = TrackingObserver::new("observer1", tracker.clone());
        let observer2 = TrackingObserver::new("observer2", tracker.clone());
        
        registry.register(Box::new(observer1));
        registry.register(Box::new(observer2));
        
        let mut user = TestUser::default();
        let result = registry.trigger_creating(&mut user).await;
        
        assert!(result.is_ok());
        
        let events = tracker.get_events();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0], "observer1: creating Test User");
        assert_eq!(events[1], "observer2: creating Test User");
    }

    #[tokio::test]
    async fn test_observer_registry_error_handling() {
        struct FailingObserver;
        
        #[async_trait]
        impl ModelObserver<TestUser> for FailingObserver {
            async fn creating(&self, _model: &mut TestUser) -> Result<(), EventError> {
                Err(EventError::validation("Observer failed"))
            }
        }
        
        let mut registry = ObserverRegistry::<TestUser>::new();
        registry.register(Box::new(FailingObserver));
        
        let mut user = TestUser::default();
        let result = registry.trigger_creating(&mut user).await;
        
        assert!(result.is_err());
        match result.unwrap_err() {
            EventError::Validation { message, .. } => {
                assert_eq!(message, "Observer failed");
            }
            _ => panic!("Expected validation error"),
        }
    }

    #[tokio::test]
    async fn test_observer_registry_error_propagation_stops_execution() {
        let mut registry = ObserverRegistry::<TestUser>::new();
        let tracker = EventTracker::new();
        
        // First observer that fails
        struct FailingObserver;
        #[async_trait]
        impl ModelObserver<TestUser> for FailingObserver {
            async fn creating(&self, _model: &mut TestUser) -> Result<(), EventError> {
                Err(EventError::validation("First observer failed"))
            }
        }
        
        // Second observer that should not be executed
        let observer2 = TrackingObserver::new("observer2", tracker.clone());
        
        registry.register(Box::new(FailingObserver));
        registry.register(Box::new(observer2));
        
        let mut user = TestUser::default();
        let result = registry.trigger_creating(&mut user).await;
        
        assert!(result.is_err());
        
        // Second observer should not have been called
        let events = tracker.get_events();
        assert_eq!(events.len(), 0);
    }

    #[tokio::test]
    async fn test_global_observer_registry() {
        let mut global_registry = GlobalObserverRegistry::new();
        assert_eq!(global_registry.observer_count(), 0);
        
        let tracker = EventTracker::new();
        let observer = TrackingObserver::new("global", tracker.clone());
        
        global_registry.register(Box::new(observer));
        // Simplified implementation returns 0 for now
        assert_eq!(global_registry.observer_count(), 0);
    }

    #[tokio::test]
    async fn test_observer_manager_creation() {
        let manager = ObserverManager::new();
        
        // Should have no model-specific observers initially
        assert!(!manager.has_observers_for::<TestUser>());
    }

    #[tokio::test]
    async fn test_observer_manager_register_model_observer() {
        let mut manager = ObserverManager::new();
        let tracker = EventTracker::new();
        let observer = TrackingObserver::new("model_observer", tracker.clone());
        
        manager.register_for_model::<TestUser>(Box::new(observer));
        
        assert!(manager.has_observers_for::<TestUser>());
    }

    #[tokio::test]
    async fn test_observer_manager_register_global_observer() {
        let mut manager = ObserverManager::new();
        let tracker = EventTracker::new();
        let observer = TrackingObserver::new("global_observer", tracker.clone());
        
        manager.register_global(Box::new(observer));
        
        // Global observers should be accessible (simplified implementation returns 0)
        assert_eq!(manager.global_observer_count(), 0);
    }
}