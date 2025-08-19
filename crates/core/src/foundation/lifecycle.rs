use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Application lifecycle states
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LifecycleState {
    Created,
    Initializing,
    Running,
    Stopping,
    Stopped,
    Failed,
}

/// Simple lifecycle manager for the framework
#[derive(Debug)]
pub struct LifecycleManager {
    state: Arc<AtomicBool>,
}

impl LifecycleManager {
    /// Create a new lifecycle manager
    pub fn new() -> Self {
        Self {
            state: Arc::new(AtomicBool::new(false)),
        }
    }
    
    /// Check if the lifecycle manager is running
    pub fn is_running(&self) -> bool {
        self.state.load(Ordering::SeqCst)
    }
    
    /// Set running state
    pub fn set_running(&self, running: bool) {
        self.state.store(running, Ordering::SeqCst);
    }
}

impl Default for LifecycleManager {
    fn default() -> Self {
        Self::new()
    }
}