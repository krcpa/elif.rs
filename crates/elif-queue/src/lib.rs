//! # elif-queue
//!
//! Background job queue system for the elif.rs framework.
//!
//! ## Features
//!
//! - **Multi-backend support**: Memory and Redis queue backends
//! - **Job persistence**: Reliable job storage and recovery
//! - **Priority queuing**: Support for job priorities and delays
//! - **Async-first**: Built for modern async Rust applications
//! - **Type-safe**: Generic job definitions with serialization support
//! - **Retry logic**: Built-in failure handling and retry mechanisms
//!
//! ## Quick Start
//!
//! ```rust
//! use elif_queue::{Queue, MemoryBackend, QueueConfig, Job, JobResult};
//! use std::time::Duration;
//! use serde::{Serialize, Deserialize};
//! use async_trait::async_trait;
//!
//! #[derive(Debug, Clone, Serialize, Deserialize)]
//! struct EmailJob {
//!     to: String,
//!     subject: String,
//!     body: String,
//! }
//!
//! #[async_trait]
//! impl Job for EmailJob {
//!     async fn execute(&self) -> JobResult<()> {
//!         // Send email logic here
//!         println!("Sending email to: {}", self.to);
//!         Ok(())
//!     }
//!
//!     fn job_type(&self) -> &'static str {
//!         "email"
//!     }
//! }
//!
//! # tokio_test::block_on(async {
//! // Create a memory-based queue
//! let queue = Queue::new(MemoryBackend::new(QueueConfig::default()));
//!
//! // Enqueue a job
//! let job = EmailJob {
//!     to: "user@example.com".to_string(),
//!     subject: "Hello".to_string(),
//!     body: "Hello, World!".to_string(),
//! };
//!
//! queue.enqueue(job, None).await.unwrap();
//!
//! // Process jobs
//! if let Some(job_entry) = queue.dequeue().await.unwrap() {
//!     let result = job_entry.execute::<EmailJob>().await;
//!     queue.complete(job_entry.id(), result).await.unwrap();
//! }
//! # });
//! ```

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::time::Duration;
use thiserror::Error;
use uuid::Uuid;

pub mod backends;
pub mod config;
pub mod scheduler;
pub mod worker;

pub use backends::*;
pub use config::*;
pub use scheduler::*;
pub use worker::*;

/// Job execution errors
#[derive(Error, Debug)]
pub enum QueueError {
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Backend error: {0}")]
    Backend(String),

    #[error("Job not found: {0}")]
    JobNotFound(String),

    #[error("Queue configuration error: {0}")]
    Configuration(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Timeout error")]
    Timeout,

    #[error("Job execution failed: {0}")]
    Execution(String),
}

/// Result type for queue operations
pub type QueueResult<T> = Result<T, QueueError>;

/// Result type for job execution
pub type JobResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// Job unique identifier
pub type JobId = Uuid;

/// Job priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default)]
pub enum Priority {
    Low = 0,
    #[default]
    Normal = 1,
    High = 2,
    Critical = 3,
}

/// Job execution state
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum JobState {
    /// Job is waiting to be processed
    Pending,
    /// Job is currently being processed
    Processing,
    /// Job completed successfully
    Completed,
    /// Job failed and will be retried
    Failed,
    /// Job failed permanently after max retries
    Dead,
}

/// Core trait that all jobs must implement
#[async_trait]
pub trait Job: Send + Sync + Serialize + DeserializeOwned {
    /// Execute the job
    async fn execute(&self) -> JobResult<()>;

    /// Get the job type identifier
    fn job_type(&self) -> &'static str;

    /// Get maximum number of retry attempts (default: 3)
    fn max_retries(&self) -> u32 {
        3
    }

    /// Get retry delay (default: exponential backoff starting at 1 second)
    fn retry_delay(&self, attempt: u32) -> Duration {
        Duration::from_secs(1 << attempt.min(6)) // Cap at 64 seconds
    }

    /// Get job timeout (default: 5 minutes)
    fn timeout(&self) -> Duration {
        Duration::from_secs(300)
    }
}

/// Job entry containing job data and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobEntry {
    /// Unique job identifier
    id: JobId,
    /// Job type for routing and processing
    job_type: String,
    /// Serialized job payload
    payload: serde_json::Value,
    /// Job priority
    priority: Priority,
    /// Current job state
    state: JobState,
    /// Number of retry attempts
    attempts: u32,
    /// Maximum retry attempts
    max_retries: u32,
    /// When the job was created
    created_at: DateTime<Utc>,
    /// When the job should be processed (for delayed jobs)
    run_at: DateTime<Utc>,
    /// When the job was last processed
    processed_at: Option<DateTime<Utc>>,
    /// Last error message (if any)
    last_error: Option<String>,
}

impl JobEntry {
    /// Create a new job entry
    pub fn new<T: Job>(
        job: T,
        priority: Option<Priority>,
        delay: Option<Duration>,
    ) -> QueueResult<Self> {
        let now = Utc::now();
        let run_at = match delay {
            Some(d) => {
                now + chrono::Duration::from_std(d).map_err(|e| {
                    QueueError::Configuration(format!("Invalid delay duration: {}", e))
                })?
            }
            None => now,
        };

        let job_type = job.job_type().to_string();
        let max_retries = job.max_retries();
        let payload = serde_json::to_value(job)?;

        Ok(JobEntry {
            id: Uuid::new_v4(),
            job_type,
            payload,
            priority: priority.unwrap_or_default(),
            state: JobState::Pending,
            attempts: 0,
            max_retries,
            created_at: now,
            run_at,
            processed_at: None,
            last_error: None,
        })
    }

    /// Get job ID
    pub fn id(&self) -> JobId {
        self.id
    }

    /// Get job type
    pub fn job_type(&self) -> &str {
        &self.job_type
    }

    /// Get job priority
    pub fn priority(&self) -> Priority {
        self.priority
    }

    /// Get job state
    pub fn state(&self) -> &JobState {
        &self.state
    }

    /// Get number of attempts
    pub fn attempts(&self) -> u32 {
        self.attempts
    }

    /// Get when job should be processed
    pub fn run_at(&self) -> DateTime<Utc> {
        self.run_at
    }

    /// Get job payload
    pub fn payload(&self) -> &serde_json::Value {
        &self.payload
    }

    /// Check if job is ready to be processed
    pub fn is_ready(&self) -> bool {
        matches!(self.state, JobState::Pending | JobState::Failed) && self.run_at <= Utc::now()
    }

    /// Deserialize and execute the job
    pub async fn execute<T: Job>(&self) -> JobResult<()> {
        let job: T = serde_json::from_value(self.payload.clone())?;
        job.execute().await
    }

    /// Mark job as processing
    pub(crate) fn mark_processing(&mut self) {
        self.state = JobState::Processing;
        self.processed_at = Some(Utc::now());
    }

    /// Mark job as completed
    pub(crate) fn mark_completed(&mut self) {
        self.state = JobState::Completed;
    }

    /// Mark job as failed
    pub(crate) fn mark_failed(&mut self, error: String) {
        self.attempts += 1;
        self.last_error = Some(error);

        if self.attempts >= self.max_retries {
            self.state = JobState::Dead;
        } else {
            self.state = JobState::Failed;
            // Set next retry time with exponential backoff
            let delay = Duration::from_secs(1 << self.attempts.min(6));
            // Set next retry time with exponential backoff
            // If the delay is too large for chrono::Duration, cap it at max value
            let chrono_delay = chrono::Duration::from_std(delay).unwrap_or(chrono::Duration::MAX);
            self.run_at = Utc::now() + chrono_delay;
        }
    }

    /// Reset job for retry (used for dead letter queue reprocessing)
    pub(crate) fn reset_for_retry(&mut self) {
        self.attempts = 0;
        self.state = JobState::Pending;
        self.run_at = Utc::now();
        self.last_error = None;
        self.processed_at = None;
    }

    /// Create a new job entry with explicit job type (for scheduled jobs)
    pub(crate) fn new_with_job_type(
        job_type: String,
        payload: serde_json::Value,
        priority: Option<Priority>,
        delay: Option<Duration>,
        max_retries: u32,
    ) -> QueueResult<Self> {
        let now = Utc::now();
        let run_at = delay
            .map(|d| now + chrono::Duration::from_std(d).unwrap())
            .unwrap_or(now);

        Ok(JobEntry {
            id: Uuid::new_v4(),
            job_type,
            payload,
            priority: priority.unwrap_or_default(),
            state: JobState::Pending,
            attempts: 0,
            max_retries,
            created_at: now,
            run_at,
            processed_at: None,
            last_error: None,
        })
    }
}

/// Core queue backend trait that all queue implementations must implement
#[async_trait]
pub trait QueueBackend: Send + Sync {
    /// Enqueue a job
    async fn enqueue(&self, job: JobEntry) -> QueueResult<JobId>;

    /// Dequeue the next available job
    async fn dequeue(&self) -> QueueResult<Option<JobEntry>>;

    /// Mark a job as completed
    async fn complete(&self, job_id: JobId, result: JobResult<()>) -> QueueResult<()>;

    /// Get job by ID
    async fn get_job(&self, job_id: JobId) -> QueueResult<Option<JobEntry>>;

    /// Get jobs by state
    async fn get_jobs_by_state(
        &self,
        state: JobState,
        limit: Option<usize>,
    ) -> QueueResult<Vec<JobEntry>>;

    /// Remove a job from the queue
    async fn remove_job(&self, job_id: JobId) -> QueueResult<bool>;

    /// Clear all jobs from the queue
    async fn clear(&self) -> QueueResult<()>;

    /// Get queue statistics
    async fn stats(&self) -> QueueResult<QueueStats>;

    /// Atomically requeue a job (remove old and enqueue new)
    /// Default implementation is non-atomic for backward compatibility
    async fn requeue_job(&self, job_id: JobId, mut job: JobEntry) -> QueueResult<bool> {
        // Non-atomic fallback implementation
        if self.remove_job(job_id).await? {
            job.reset_for_retry();
            self.enqueue(job).await?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Atomically clear all jobs in a specific state
    /// Default implementation is non-atomic for backward compatibility
    async fn clear_jobs_by_state(&self, state: JobState) -> QueueResult<u64> {
        // Non-atomic fallback implementation
        let jobs = self.get_jobs_by_state(state, None).await?;
        let count = jobs.len() as u64;

        for job in jobs {
            self.remove_job(job.id()).await?;
        }

        Ok(count)
    }
}

/// Queue statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct QueueStats {
    pub pending_jobs: u64,
    pub processing_jobs: u64,
    pub completed_jobs: u64,
    pub failed_jobs: u64,
    pub dead_jobs: u64,
    pub total_jobs: u64,
}

/// High-level queue interface
pub struct Queue<B: QueueBackend> {
    backend: B,
}

impl<B: QueueBackend> Queue<B> {
    /// Create a new queue instance with the given backend
    pub fn new(backend: B) -> Self {
        Self { backend }
    }

    /// Enqueue a job
    pub async fn enqueue<T: Job>(&self, job: T, priority: Option<Priority>) -> QueueResult<JobId> {
        let entry = JobEntry::new(job, priority, None)?;
        self.backend.enqueue(entry).await
    }

    /// Enqueue a delayed job
    pub async fn enqueue_delayed<T: Job>(
        &self,
        job: T,
        delay: Duration,
        priority: Option<Priority>,
    ) -> QueueResult<JobId> {
        let entry = JobEntry::new(job, priority, Some(delay))?;
        self.backend.enqueue(entry).await
    }

    /// Dequeue the next available job
    pub async fn dequeue(&self) -> QueueResult<Option<JobEntry>> {
        self.backend.dequeue().await
    }

    /// Mark a job as completed
    pub async fn complete(&self, job_id: JobId, result: JobResult<()>) -> QueueResult<()> {
        self.backend.complete(job_id, result).await
    }

    /// Get job by ID
    pub async fn get_job(&self, job_id: JobId) -> QueueResult<Option<JobEntry>> {
        self.backend.get_job(job_id).await
    }

    /// Get jobs by state
    pub async fn get_jobs_by_state(
        &self,
        state: JobState,
        limit: Option<usize>,
    ) -> QueueResult<Vec<JobEntry>> {
        self.backend.get_jobs_by_state(state, limit).await
    }

    /// Remove a job from the queue
    pub async fn remove_job(&self, job_id: JobId) -> QueueResult<bool> {
        self.backend.remove_job(job_id).await
    }

    /// Clear all jobs from the queue
    pub async fn clear(&self) -> QueueResult<()> {
        self.backend.clear().await
    }

    /// Get queue statistics
    pub async fn stats(&self) -> QueueResult<QueueStats> {
        self.backend.stats().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backends::MemoryBackend;

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct TestJob {
        message: String,
    }

    #[async_trait]
    impl Job for TestJob {
        async fn execute(&self) -> JobResult<()> {
            println!("Executing job: {}", self.message);
            Ok(())
        }

        fn job_type(&self) -> &'static str {
            "test"
        }
    }

    #[tokio::test]
    async fn test_job_entry_creation() {
        let job = TestJob {
            message: "Hello, World!".to_string(),
        };

        let entry = JobEntry::new(job, Some(Priority::High), None).unwrap();
        assert_eq!(entry.job_type(), "test");
        assert_eq!(entry.priority(), Priority::High);
        assert_eq!(entry.state(), &JobState::Pending);
        assert_eq!(entry.attempts(), 0);
        assert!(entry.is_ready());
    }

    #[tokio::test]
    async fn test_delayed_job() {
        let job = TestJob {
            message: "Delayed job".to_string(),
        };

        let delay = Duration::from_secs(60);
        let entry = JobEntry::new(job, None, Some(delay)).unwrap();

        // Delayed job should not be ready immediately
        assert!(!entry.is_ready());
        assert!(entry.run_at() > Utc::now());
    }

    #[tokio::test]
    async fn test_duration_conversion_error_handling() {
        use std::time::Duration as StdDuration;

        let job = TestJob {
            message: "test".to_string(),
        };

        // Test with maximum possible duration that would overflow chrono::Duration
        let max_delay = StdDuration::MAX;
        let result = JobEntry::new(job.clone(), None, Some(max_delay));

        // This should fail with a Configuration error instead of panicking
        assert!(result.is_err());
        if let Err(QueueError::Configuration(msg)) = result {
            assert!(msg.contains("Invalid delay duration"));
        } else {
            panic!("Expected Configuration error for invalid delay duration");
        }

        // Test that mark_failed doesn't panic with very large retry attempts
        let entry = JobEntry::new(job, None, None).unwrap();
        let mut job_entry = entry;

        // Set attempts to a very high value to test exponential backoff overflow
        job_entry.attempts = 100; // This would cause 2^100 seconds delay, way beyond chrono limits

        // This should not panic, but gracefully handle the overflow
        job_entry.mark_failed("test error".to_string());

        // Job should still be properly handled
        assert_eq!(job_entry.state, JobState::Dead); // Should exceed max_retries and be dead
    }

    #[tokio::test]
    async fn test_queue_basic_operations() {
        let backend = MemoryBackend::new(QueueConfig::default());
        let queue = Queue::new(backend);

        let job = TestJob {
            message: "Test job".to_string(),
        };

        // Enqueue job
        let job_id = queue.enqueue(job, Some(Priority::Normal)).await.unwrap();

        // Dequeue job
        let job_entry = queue.dequeue().await.unwrap().unwrap();
        assert_eq!(job_entry.id(), job_id);
        assert_eq!(job_entry.job_type(), "test");

        // Complete job
        let result = job_entry.execute::<TestJob>().await;
        queue.complete(job_id, result).await.unwrap();

        // Verify stats
        let stats = queue.stats().await.unwrap();
        assert_eq!(stats.total_jobs, 1);
        assert_eq!(stats.completed_jobs, 1);
    }
}
