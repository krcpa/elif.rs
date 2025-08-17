//! Queue command modules for job processing and scheduling
//!
//! This module contains the queue-related commands split into logical modules:
//! - `queue_work`: Work processing functionality for background job execution
//! - `queue_status`: Status monitoring functionality for queue inspection
//! - `queue_scheduler`: Scheduled commands and daemon functionality for cron-like jobs

pub mod queue_work;
pub mod queue_status;
pub mod queue_scheduler;

// Re-export the main types and functions for backward compatibility
pub use queue_work::{QueueWorkArgs, QueueWorkCommand, work};
pub use queue_status::{QueueStatusArgs, QueueStatusCommand, status};
pub use queue_scheduler::{ScheduleRunArgs, ScheduleRunCommand, schedule_run};