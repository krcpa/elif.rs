//! Worker implementation for processing jobs from the queue

use crate::{JobEntry, JobResult, Queue, QueueBackend, QueueConfig, QueueError, QueueResult};
use async_trait::async_trait;
use futures::future::BoxFuture;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, Semaphore};
use tokio::time::{interval, timeout, Duration};
use tracing::{debug, error, info, warn};

/// Job handler function type
pub type JobHandler = Arc<dyn Fn(JobEntry) -> BoxFuture<'static, JobResult<()>> + Send + Sync>;

/// Worker registry for managing job handlers
pub struct WorkerRegistry {
    handlers: RwLock<HashMap<String, JobHandler>>,
}

impl WorkerRegistry {
    /// Create a new worker registry
    pub fn new() -> Self {
        Self {
            handlers: RwLock::new(HashMap::new()),
        }
    }

    /// Register a job handler for a specific job type
    pub async fn register<T: crate::Job + JobTypeProvider + 'static>(
        &self,
        handler: impl JobProcessor<T> + 'static,
    ) {
        let job_type = T::default_job_type();
        let handler: JobHandler = Arc::new(move |entry: JobEntry| {
            let handler = handler.clone();
            Box::pin(async move { handler.process(entry).await })
        });

        self.handlers
            .write()
            .await
            .insert(job_type.to_string(), handler);
        info!("Registered job handler for type: {}", job_type);
    }

    /// Get a handler for a job type
    pub async fn get_handler(&self, job_type: &str) -> Option<JobHandler> {
        self.handlers.read().await.get(job_type).cloned()
    }

    /// List all registered job types
    pub async fn list_job_types(&self) -> Vec<String> {
        self.handlers.read().await.keys().cloned().collect()
    }
}

impl Default for WorkerRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Trait for processing specific job types
#[async_trait]
pub trait JobProcessor<T: crate::Job>: Clone + Send + Sync {
    /// Process a job entry
    async fn process(&self, entry: JobEntry) -> JobResult<()>;
}

/// Convenience implementation for closure-based processors
#[async_trait]
impl<T, F, Fut> JobProcessor<T> for F
where
    T: crate::Job + 'static,
    F: Fn(T) -> Fut + Clone + Send + Sync + 'static,
    Fut: std::future::Future<Output = JobResult<()>> + Send + 'static,
{
    async fn process(&self, entry: JobEntry) -> JobResult<()> {
        let job: T = serde_json::from_value(entry.payload.clone())?;
        self(job).await
    }
}

/// Worker for processing jobs from a queue
pub struct Worker<B: QueueBackend> {
    queue: Arc<Queue<B>>,
    registry: Arc<WorkerRegistry>,
    config: QueueConfig,
    concurrency_limiter: Arc<Semaphore>,
}

impl<B: QueueBackend + 'static> Worker<B> {
    /// Create a new worker
    pub fn new(queue: Queue<B>, registry: WorkerRegistry, config: QueueConfig) -> Self {
        let concurrency_limiter = Arc::new(Semaphore::new(*config.get_max_workers()));

        Self {
            queue: Arc::new(queue),
            registry: Arc::new(registry),
            config,
            concurrency_limiter,
        }
    }

    /// Start processing jobs
    pub async fn start(&self) -> QueueResult<()> {
        info!(
            "Starting worker with {} max concurrent jobs",
            self.config.get_max_workers()
        );

        let mut poll_interval = interval(*self.config.get_poll_interval());

        loop {
            poll_interval.tick().await;

            // Try to get a job from the queue
            match self.queue.dequeue().await {
                Ok(Some(job_entry)) => {
                    let permit = match self.concurrency_limiter.clone().try_acquire_owned() {
                        Ok(permit) => permit,
                        Err(_) => {
                            // No available worker slots, continue polling
                            debug!("No available worker slots, skipping job processing");
                            continue;
                        }
                    };

                    let queue = self.queue.clone();
                    let registry = self.registry.clone();
                    let job_timeout = *self.config.get_default_timeout();

                    // Process job in background
                    tokio::spawn(async move {
                        let _permit = permit; // Hold permit until job is done

                        let job_id = job_entry.id();
                        let job_type = job_entry.job_type().to_string();

                        debug!("Processing job {} of type {}", job_id, job_type);

                        let result = if let Some(handler) = registry.get_handler(&job_type).await {
                            // Execute with timeout
                            match timeout(job_timeout, handler(job_entry)).await {
                                Ok(result) => result,
                                Err(_) => {
                                    error!("Job {} timed out after {:?}", job_id, job_timeout);
                                    Err(Box::new(QueueError::Timeout)
                                        as Box<dyn std::error::Error + Send + Sync>)
                                }
                            }
                        } else {
                            error!("No handler registered for job type: {}", job_type);
                            Err(Box::new(QueueError::Configuration(format!(
                                "No handler for job type: {}",
                                job_type
                            )))
                                as Box<dyn std::error::Error + Send + Sync>)
                        };

                        // Complete the job and log outcome
                        match &result {
                            Ok(_) => {
                                info!("Job {} completed successfully", job_id);
                                if let Err(e) = queue.complete(job_id, result).await {
                                    error!("Failed to mark job {} as completed: {}", job_id, e);
                                }
                            }
                            Err(e) => {
                                warn!("Job {} failed: {}", job_id, e);
                                if let Err(e2) = queue.complete(job_id, result).await {
                                    error!("Failed to mark job {} as completed: {}", job_id, e2);
                                }
                            }
                        }
                    });
                }
                Ok(None) => {
                    // No jobs available, continue polling
                    continue;
                }
                Err(e) => {
                    error!("Failed to dequeue job: {}", e);
                    // Brief pause before retrying
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            }
        }
    }

    /// Start processing with graceful shutdown
    pub async fn start_with_shutdown(
        &self,
        mut shutdown: tokio::sync::mpsc::Receiver<()>,
    ) -> QueueResult<()> {
        info!("Starting worker with graceful shutdown support");

        let mut poll_interval = interval(*self.config.get_poll_interval());

        let shutting_down = false;

        loop {
            tokio::select! {
                _ = poll_interval.tick(), if !shutting_down => {
                    // Try to get a job from the queue
                    match self.queue.dequeue().await {
                        Ok(Some(job_entry)) => {
                            let permit = match self.concurrency_limiter.clone().try_acquire_owned() {
                                Ok(permit) => permit,
                                Err(_) => {
                                    debug!("No available worker slots, skipping job processing");
                                    continue;
                                }
                            };

                            let queue = self.queue.clone();
                            let registry = self.registry.clone();
                            let job_timeout = *self.config.get_default_timeout();

                            // Process job in background
                            tokio::spawn(async move {
                                let _permit = permit;

                                let job_id = job_entry.id();
                                let job_type = job_entry.job_type().to_string();

                                debug!("Processing job {} of type {}", job_id, job_type);

                                let result = if let Some(handler) = registry.get_handler(&job_type).await {
                                    match timeout(job_timeout, handler(job_entry)).await {
                                        Ok(result) => result,
                                        Err(_) => {
                                            error!("Job {} timed out after {:?}", job_id, job_timeout);
                                            Err(Box::new(QueueError::Timeout) as Box<dyn std::error::Error + Send + Sync>)
                                        }
                                    }
                                } else {
                                    error!("No handler registered for job type: {}", job_type);
                                    Err(Box::new(QueueError::Configuration(
                                        format!("No handler for job type: {}", job_type)
                                    )) as Box<dyn std::error::Error + Send + Sync>)
                                };

                                // Complete the job and log outcome
                                match &result {
                                    Ok(_) => {
                                        info!("Job {} completed successfully", job_id);
                                        if let Err(e) = queue.complete(job_id, result).await {
                                            error!("Failed to mark job {} as completed: {}", job_id, e);
                                        }
                                    }
                                    Err(e) => {
                                        warn!("Job {} failed: {}", job_id, e);
                                        if let Err(e2) = queue.complete(job_id, result).await {
                                            error!("Failed to mark job {} as completed: {}", job_id, e2);
                                        }
                                    }
                                }
                            });
                        }
                        Ok(None) => {
                            // No jobs available
                            continue;
                        }
                        Err(e) => {
                            error!("Failed to dequeue job: {}", e);
                            tokio::time::sleep(Duration::from_secs(1)).await;
                        }
                    }
                }

                _ = shutdown.recv() => {
                    info!("Shutdown signal received, stopping new job processing");

                    // Wait for active jobs to complete
                    let active_jobs = *self.config.get_max_workers() - self.concurrency_limiter.available_permits();
                    info!("Waiting for {} active jobs to complete", active_jobs);
                    while self.concurrency_limiter.available_permits() < *self.config.get_max_workers() {
                        tokio::time::sleep(Duration::from_millis(100)).await;
                    }

                    info!("Worker shutdown complete");
                    break;
                }
            }
        }

        Ok(())
    }

    /// Get worker statistics
    pub async fn stats(&self) -> QueueResult<WorkerStats> {
        let queue_stats = self.queue.stats().await?;
        let available_slots = self.concurrency_limiter.available_permits();
        let active_jobs = *self.config.get_max_workers() - available_slots;
        let job_types = self.registry.list_job_types().await;

        Ok(WorkerStats {
            queue_stats,
            max_workers: *self.config.get_max_workers(),
            active_workers: active_jobs,
            available_workers: available_slots,
            registered_job_types: job_types,
        })
    }
}

/// Worker statistics
#[derive(Debug, Clone)]
pub struct WorkerStats {
    pub queue_stats: crate::QueueStats,
    pub max_workers: usize,
    pub active_workers: usize,
    pub available_workers: usize,
    pub registered_job_types: Vec<String>,
}

/// Extension trait for Job to provide default job type
pub trait JobTypeProvider {
    fn default_job_type() -> &'static str;
}

// Blanket implementation for jobs that implement the main Job trait
impl<T> JobTypeProvider for T
where
    T: crate::Job + Default,
{
    fn default_job_type() -> &'static str {
        T::default().job_type()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::QueueConfigBuilder;
    use crate::{backends::MemoryBackend, Priority};
    use serde::{Deserialize, Serialize};
    use std::sync::atomic::{AtomicU32, Ordering};

    #[derive(Debug, Clone, Serialize, Deserialize, Default)]
    struct TestJob {
        id: u32,
        message: String,
    }

    #[async_trait]
    impl crate::Job for TestJob {
        async fn execute(&self) -> JobResult<()> {
            Ok(())
        }

        fn job_type(&self) -> &'static str {
            "test"
        }
    }

    #[derive(Clone)]
    struct TestJobProcessor {
        counter: Arc<AtomicU32>,
    }

    impl TestJobProcessor {
        fn new() -> Self {
            Self {
                counter: Arc::new(AtomicU32::new(0)),
            }
        }

        fn get_count(&self) -> u32 {
            self.counter.load(Ordering::Relaxed)
        }
    }

    #[async_trait]
    impl JobProcessor<TestJob> for TestJobProcessor {
        async fn process(&self, _entry: JobEntry) -> JobResult<()> {
            self.counter.fetch_add(1, Ordering::Relaxed);
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_worker_registry() {
        let registry = WorkerRegistry::new();
        let processor = TestJobProcessor::new();

        registry.register::<TestJob>(processor.clone()).await;

        let job_types = registry.list_job_types().await;
        assert_eq!(job_types, vec!["test"]);

        let handler = registry.get_handler("test").await;
        assert!(handler.is_some());

        let no_handler = registry.get_handler("nonexistent").await;
        assert!(no_handler.is_none());
    }

    #[tokio::test]
    async fn test_job_processing() {
        let backend = MemoryBackend::new(
            QueueConfigBuilder::testing()
                .build()
                .expect("Failed to build config"),
        );
        let queue = Queue::new(backend);
        let registry = WorkerRegistry::new();
        let processor = TestJobProcessor::new();

        registry.register::<TestJob>(processor.clone()).await;

        // Enqueue a test job
        let job = TestJob {
            id: 1,
            message: "test message".to_string(),
        };
        let job_id = queue.enqueue(job, Some(Priority::Normal)).await.unwrap();

        // Process job manually
        let job_entry = queue.dequeue().await.unwrap().unwrap();
        let handler = registry.get_handler("test").await.unwrap();
        let result = handler(job_entry).await;

        assert!(result.is_ok());
        assert_eq!(processor.get_count(), 1);

        // Complete the job
        queue.complete(job_id, result).await.unwrap();

        // Verify job was completed
        let stats = queue.stats().await.unwrap();
        assert_eq!(stats.completed_jobs, 1);
    }

    #[tokio::test]
    async fn test_worker_stats() {
        let backend = MemoryBackend::new(
            QueueConfigBuilder::testing()
                .build()
                .expect("Failed to build config"),
        );
        let queue = Queue::new(backend);
        let registry = WorkerRegistry::new();
        let processor = TestJobProcessor::new();
        let config = QueueConfigBuilder::testing()
            .build()
            .expect("Failed to build config");

        registry.register::<TestJob>(processor).await;
        let worker = Worker::new(queue, registry, config);

        let stats = worker.stats().await.unwrap();
        assert_eq!(stats.max_workers, 1); // testing config
        assert_eq!(stats.active_workers, 0);
        assert_eq!(stats.available_workers, 1);
        assert_eq!(stats.registered_job_types, vec!["test"]);
    }
}
