// Re-export all email functionality from submodules
pub mod analytics;
pub mod core;
pub mod providers;
pub mod queue;
pub mod templates;
pub mod testing;

// Re-export all public items
pub use analytics::*;
pub use core::*;
pub use providers::*;
pub use queue::*;
pub use templates::*;
pub use testing::*;