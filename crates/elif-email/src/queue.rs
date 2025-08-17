//! Email queue integration with elif-queue
//!
//! This module provides background email processing capabilities using the elif-queue system.

use crate::{Email, EmailError, EmailProvider, EmailResult};
use async_trait::async_trait;
use elif_queue::{
    Job, JobResult, JobEntry, JobId, Priority, Queue, QueueBackend, 
    QueueResult, QueueError, JobState
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{info, warn, error};
use uuid::Uuid;

/// Email job for background processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailJob {
    /// The email to be sent
    pub email: Email,
    /// Job priority
    pub priority: EmailPriority,
    /// Maximum retry attempts
    pub max_retries: u32,
    /// Job timeout
    pub timeout_seconds: u64,
    /// Provider to use for sending (optional, uses default if None)
    pub provider: Option<String>,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

/// Email priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum EmailPriority {
    /// Low priority (newsletters, bulk emails)
    Low = 0,
    /// Normal priority (user notifications)
    Normal = 1,
    /// High priority (password resets, confirmations)
    High = 2,
    /// Critical priority (security alerts, system notifications)
    Critical = 3,
}

impl Default for EmailPriority {
    fn default() -> Self {
        EmailPriority::Normal
    }
}

impl From<EmailPriority> for Priority {
    fn from(email_priority: EmailPriority) -> Self {
        match email_priority {
            EmailPriority::Low => Priority::Low,
            EmailPriority::Normal => Priority::Normal,
            EmailPriority::High => Priority::High,
            EmailPriority::Critical => Priority::Critical,
        }
    }
}

impl From<Priority> for EmailPriority {
    fn from(priority: Priority) -> Self {
        match priority {
            Priority::Low => EmailPriority::Low,
            Priority::Normal => EmailPriority::Normal,
            Priority::High => EmailPriority::High,
            Priority::Critical => EmailPriority::Critical,
        }
    }
}

impl EmailJob {
    /// Create a new email job
    pub fn new(email: Email) -> Self {
        Self {
            email,
            priority: EmailPriority::Normal,
            max_retries: 3,
            timeout_seconds: 60,
            provider: None,
            metadata: HashMap::new(),
        }
    }
    
    /// Set job priority
    pub fn with_priority(mut self, priority: EmailPriority) -> Self {
        self.priority = priority;
        self
    }
    
    /// Set maximum retry attempts
    pub fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }
    
    /// Set job timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout_seconds = timeout.as_secs();
        self
    }
    
    /// Set specific provider to use
    pub fn with_provider(mut self, provider: String) -> Self {
        self.provider = Some(provider);
        self
    }
    
    /// Add metadata
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }
}

#[async_trait]
impl Job for EmailJob {
    async fn execute(&self) -> JobResult<()> {
        info!("Processing email job for: {}", self.email.id);
        
        // In a real implementation, this would get the email processor from context
        // For now, we'll return a placeholder implementation
        Err("Email job execution not yet implemented - requires EmailJobProcessor".into())
    }
    
    fn job_type(&self) -> &'static str {
        "email"
    }
    
    fn max_retries(&self) -> u32 {
        self.max_retries
    }
    
    fn retry_delay(&self, attempt: u32) -> Duration {
        // Exponential backoff with jitter
        let base_delay = Duration::from_secs(1 << attempt.min(6)); // Cap at 64 seconds
        let jitter = Duration::from_millis(rand::random::<u64>() % 1000);
        base_delay + jitter
    }
    
    fn timeout(&self) -> Duration {
        Duration::from_secs(self.timeout_seconds)
    }
}

/// Email job processor handles the execution of email jobs
pub struct EmailJobProcessor {
    /// Default email provider
    default_provider: Arc<dyn EmailProvider>,
    /// Named email providers
    providers: RwLock<HashMap<String, Arc<dyn EmailProvider>>>,
    /// Delivery tracking
    delivery_tracking: RwLock<HashMap<Uuid, EmailDeliveryStatus>>,
}

/// Email delivery status tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailDeliveryStatus {
    pub email_id: Uuid,
    pub status: DeliveryState,
    pub attempts: Vec<DeliveryAttempt>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeliveryState {
    Queued,
    Processing,
    Sent,
    Failed,
    Bounced,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliveryAttempt {
    pub attempt_number: u32,
    pub attempted_at: chrono::DateTime<chrono::Utc>,
    pub result: DeliveryAttemptResult,
    pub provider_used: String,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeliveryAttemptResult {
    Success(EmailResult),
    Failure(String),
    Timeout,
}

impl EmailJobProcessor {
    /// Create a new email job processor
    pub fn new(default_provider: Arc<dyn EmailProvider>) -> Self {
        Self {
            default_provider,
            providers: RwLock::new(HashMap::new()),
            delivery_tracking: RwLock::new(HashMap::new()),
        }
    }
    
    /// Register a named email provider
    pub async fn register_provider(&self, name: String, provider: Arc<dyn EmailProvider>) {
        self.providers.write().await.insert(name, provider);
    }
    
    /// Process an email job
    pub async fn process(&self, job: &EmailJob) -> JobResult<()> {
        let start_time = chrono::Utc::now();
        info!("Processing email job {} with priority {:?}", job.email.id, job.priority);
        
        // Track delivery start
        {
            let mut tracking = self.delivery_tracking.write().await;
            tracking.insert(job.email.id, EmailDeliveryStatus {
                email_id: job.email.id,
                status: DeliveryState::Processing,
                attempts: Vec::new(),
                created_at: start_time,
                updated_at: start_time,
            });
        }
        
        // Select provider
        let provider = if let Some(ref provider_name) = job.provider {
            self.providers.read().await
                .get(provider_name)
                .cloned()
                .unwrap_or_else(|| {
                    warn!("Provider '{}' not found, using default", provider_name);
                    self.default_provider.clone()
                })
        } else {
            self.default_provider.clone()
        };
        
        // Attempt to send email (with panic safety)  
        let result = tokio::time::timeout(
            Duration::from_secs(job.timeout_seconds),
            async {
                // Wrap the provider call in a task that can handle panics
                let provider_clone = provider.clone();
                let email_clone = job.email.clone();
                match tokio::spawn(async move {
                    provider_clone.send(&email_clone).await
                }).await {
                    Ok(result) => result,
                    Err(join_error) => {
                        if join_error.is_panic() {
                            Err(EmailError::provider("unknown", "Provider panicked during send"))
                        } else {
                            Err(EmailError::provider("unknown", "Provider task was cancelled"))
                        }
                    }
                }
            }
        ).await;
        
        let attempt_result = match result {
            Ok(Ok(email_result)) => {
                info!("Email {} sent successfully via {}", job.email.id, provider.provider_name());
                DeliveryAttemptResult::Success(email_result)
            }
            Ok(Err(email_error)) => {
                error!("Failed to send email {}: {}", job.email.id, email_error);
                DeliveryAttemptResult::Failure(email_error.to_string())
            }
            Err(_) => {
                error!("Email {} send timed out", job.email.id);
                DeliveryAttemptResult::Timeout
            }
        };
        
        // Update delivery tracking
        {
            let mut tracking = self.delivery_tracking.write().await;
            if let Some(status) = tracking.get_mut(&job.email.id) {
                let attempt = DeliveryAttempt {
                    attempt_number: status.attempts.len() as u32 + 1,
                    attempted_at: chrono::Utc::now(),
                    result: attempt_result.clone(),
                    provider_used: provider.provider_name().to_string(),
                    error_message: match &attempt_result {
                        DeliveryAttemptResult::Failure(msg) => Some(msg.clone()),
                        DeliveryAttemptResult::Timeout => Some("Request timed out".to_string()),
                        _ => None,
                    },
                };
                
                status.attempts.push(attempt);
                status.updated_at = chrono::Utc::now();
                
                status.status = match attempt_result {
                    DeliveryAttemptResult::Success(_) => DeliveryState::Sent,
                    _ => DeliveryState::Failed,
                };
            }
        }
        
        // Return result
        match attempt_result {
            DeliveryAttemptResult::Success(_) => Ok(()),
            DeliveryAttemptResult::Failure(msg) => Err(msg.into()),
            DeliveryAttemptResult::Timeout => Err("Email send timed out".into()),
        }
    }
    
    /// Get delivery status for an email
    pub async fn get_delivery_status(&self, email_id: Uuid) -> Option<EmailDeliveryStatus> {
        self.delivery_tracking.read().await.get(&email_id).cloned()
    }
    
    /// Get all delivery statuses
    pub async fn get_all_delivery_statuses(&self) -> Vec<EmailDeliveryStatus> {
        self.delivery_tracking.read().await.values().cloned().collect()
    }
    
    /// Clear old delivery tracking records
    pub async fn cleanup_old_records(&self, older_than: Duration) {
        let cutoff = chrono::Utc::now() - chrono::Duration::from_std(older_than).unwrap();
        let mut tracking = self.delivery_tracking.write().await;
        tracking.retain(|_, status| status.updated_at > cutoff);
    }
}

/// Email queue service - high-level interface for queuing emails
pub struct EmailQueueService<B: QueueBackend> {
    queue: Queue<B>,
    processor: Option<Arc<EmailJobProcessor>>,
}

impl<B: QueueBackend> EmailQueueService<B> {
    /// Create a new email queue service
    pub fn new(backend: B) -> Self {
        Self {
            queue: Queue::new(backend),
            processor: None,
        }
    }
    
    /// Set the email job processor
    pub fn with_processor(mut self, processor: Arc<EmailJobProcessor>) -> Self {
        self.processor = Some(processor);
        self
    }
    
    /// Queue an email for sending
    pub async fn enqueue(&self, email: Email) -> QueueResult<JobId> {
        let job = EmailJob::new(email);
        self.enqueue_job(job).await
    }
    
    /// Queue an email with specific priority
    pub async fn enqueue_with_priority(&self, email: Email, priority: EmailPriority) -> QueueResult<JobId> {
        let job = EmailJob::new(email).with_priority(priority);
        self.enqueue_job(job).await
    }
    
    /// Queue an email for later sending
    pub async fn enqueue_scheduled(&self, email: Email, send_at: chrono::DateTime<chrono::Utc>) -> QueueResult<JobId> {
        let now = chrono::Utc::now();
        let delay = if send_at > now {
            match (send_at - now).to_std() {
                Ok(duration) => {
                    // Cap at maximum reasonable delay (30 days)
                    let max_delay = Duration::from_secs(30 * 24 * 60 * 60);
                    if duration > max_delay {
                        return Err(QueueError::Configuration(
                            format!("Scheduled time is too far in the future (max 30 days): {:?}", duration)
                        ));
                    }
                    duration
                }
                Err(_) => {
                    return Err(QueueError::Configuration(
                        "Invalid scheduled time: duration cannot be converted to std::time::Duration".to_string()
                    ));
                }
            }
        } else {
            Duration::from_secs(0)
        };
        
        let job = EmailJob::new(email);
        self.queue.enqueue_delayed(job, delay, Some(Priority::Normal)).await
    }
    
    /// Queue a batch of emails
    pub async fn enqueue_batch(&self, emails: Vec<Email>) -> QueueResult<Vec<JobId>> {
        let mut job_ids = Vec::new();
        for email in emails {
            let job_id = self.enqueue(email).await?;
            job_ids.push(job_id);
        }
        Ok(job_ids)
    }
    
    /// Internal method to queue an EmailJob
    async fn enqueue_job(&self, job: EmailJob) -> QueueResult<JobId> {
        let priority = Priority::from(job.priority);
        self.queue.enqueue(job, Some(priority)).await
    }
    
    /// Get job by ID
    pub async fn get_job(&self, job_id: JobId) -> QueueResult<Option<JobEntry>> {
        self.queue.get_job(job_id).await
    }
    
    /// Get jobs by state
    pub async fn get_jobs_by_state(&self, state: JobState, limit: Option<usize>) -> QueueResult<Vec<JobEntry>> {
        self.queue.get_jobs_by_state(state, limit).await
    }
    
    /// Get queue statistics
    pub async fn stats(&self) -> QueueResult<EmailQueueStats> {
        let queue_stats = self.queue.stats().await?;
        Ok(EmailQueueStats {
            pending: queue_stats.pending_jobs,
            processing: queue_stats.processing_jobs,
            completed: queue_stats.completed_jobs,
            failed: queue_stats.failed_jobs,
            dead_letter: queue_stats.dead_jobs,
            total: queue_stats.total_jobs,
        })
    }
    
    /// Remove a job from the queue
    pub async fn remove_job(&self, job_id: JobId) -> QueueResult<bool> {
        self.queue.remove_job(job_id).await
    }
    
    /// Clear all jobs
    pub async fn clear(&self) -> QueueResult<()> {
        self.queue.clear().await
    }
    
    /// Process jobs (typically called by workers)
    pub async fn process_next_job(&self) -> QueueResult<bool> {
        if let Some(processor) = &self.processor {
            if let Some(job_entry) = self.queue.dequeue().await? {
                let job_id = job_entry.id();

                let result = match serde_json::from_value::<EmailJob>(job_entry.payload().clone()) {
                    Ok(job) => processor.process(&job).await,
                    Err(e) => Err(e.into()),
                };

                // Complete the job in the queue
                self.queue.complete(job_id, result).await?;
                Ok(true)
            } else {
                Ok(false) // No jobs available
            }
        } else {
            Err(QueueError::Configuration("No email processor configured".to_string()))
        }
    }
}

/// Email queue statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailQueueStats {
    pub pending: u64,
    pub processing: u64,
    pub completed: u64,
    pub failed: u64,
    pub dead_letter: u64,
    pub total: u64,
}

/// Email worker for background processing
pub struct EmailWorker<B: QueueBackend> {
    service: Arc<EmailQueueService<B>>,
    shutdown_notify: Arc<tokio::sync::Notify>,
    worker_id: String,
    config: EmailWorkerConfig,
}

/// Email worker configuration
#[derive(Debug, Clone)]
pub struct EmailWorkerConfig {
    /// How long to sleep between job checks
    pub poll_interval: Duration,
    /// Maximum number of jobs to process per batch
    pub batch_size: usize,
    /// Worker shutdown timeout
    pub shutdown_timeout: Duration,
    /// Enable verbose logging
    pub verbose: bool,
}

impl Default for EmailWorkerConfig {
    fn default() -> Self {
        Self {
            poll_interval: Duration::from_secs(1),
            batch_size: 10,
            shutdown_timeout: Duration::from_secs(30),
            verbose: false,
        }
    }
}

impl<B: QueueBackend + 'static> EmailWorker<B> {
    /// Create a new email worker
    pub fn new(service: Arc<EmailQueueService<B>>) -> Self {
        Self {
            service,
            shutdown_notify: Arc::new(tokio::sync::Notify::new()),
            worker_id: Uuid::new_v4().to_string(),
            config: EmailWorkerConfig::default(),
        }
    }
    
    /// Create a worker with configuration
    pub fn with_config(mut self, config: EmailWorkerConfig) -> Self {
        self.config = config;
        self
    }
    
    /// Start the worker
    pub async fn start(&self) -> tokio::task::JoinHandle<()> {
        let service = self.service.clone();
        let shutdown_notify = self.shutdown_notify.clone();
        let worker_id = self.worker_id.clone();
        let config = self.config.clone();
        
        tokio::spawn(async move {
            info!("Email worker {} started", worker_id);
            
            loop {
                let mut processed_count = 0;
                
                // Process up to batch_size jobs
                for _ in 0..config.batch_size {
                    match service.process_next_job().await {
                        Ok(true) => processed_count += 1,
                        Ok(false) => break, // No more jobs
                        Err(e) => {
                            error!("Error processing job in worker {}: {}", worker_id, e);
                            break;
                        }
                    }
                }
                
                if config.verbose && processed_count > 0 {
                    info!("Worker {} processed {} jobs", worker_id, processed_count);
                }
                
                // Wait for either shutdown signal or poll interval
                tokio::select! {
                    _ = shutdown_notify.notified() => {
                        info!("Received shutdown signal for worker {}", worker_id);
                        break;
                    }
                    _ = tokio::time::sleep(config.poll_interval) => {
                        // Continue to next iteration
                    }
                }
            }
            
            info!("Email worker {} stopped", worker_id);
        })
    }
    
    /// Stop the worker gracefully
    pub fn stop(&self) {
        info!("Stopping email worker {}", self.worker_id);
        self.shutdown_notify.notify_one();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use elif_queue::{MemoryBackend, QueueConfig};
    
    #[test]
    fn test_email_priority_conversion() {
        assert_eq!(Priority::from(EmailPriority::Low), Priority::Low);
        assert_eq!(Priority::from(EmailPriority::Normal), Priority::Normal);
        assert_eq!(Priority::from(EmailPriority::High), Priority::High);
        assert_eq!(Priority::from(EmailPriority::Critical), Priority::Critical);
        
        assert_eq!(EmailPriority::from(Priority::Low), EmailPriority::Low);
        assert_eq!(EmailPriority::from(Priority::Normal), EmailPriority::Normal);
        assert_eq!(EmailPriority::from(Priority::High), EmailPriority::High);
        assert_eq!(EmailPriority::from(Priority::Critical), EmailPriority::Critical);
    }
    
    #[test]
    fn test_email_job_builder() {
        let email = Email::new()
            .from("test@example.com")
            .to("user@example.com")
            .subject("Test")
            .text_body("Hello");
            
        let job = EmailJob::new(email)
            .with_priority(EmailPriority::High)
            .with_max_retries(5)
            .with_timeout(Duration::from_secs(120))
            .with_provider("sendgrid".to_string())
            .with_metadata("campaign".to_string(), "welcome".to_string());
            
        assert_eq!(job.priority, EmailPriority::High);
        assert_eq!(job.max_retries, 5);
        assert_eq!(job.timeout_seconds, 120);
        assert_eq!(job.provider.as_ref().unwrap(), "sendgrid");
        assert_eq!(job.metadata.get("campaign").unwrap(), "welcome");
    }
    
    #[tokio::test]
    async fn test_email_queue_service() {
        let backend = MemoryBackend::new(QueueConfig::default());
        let service = EmailQueueService::new(backend);
        
        let email = Email::new()
            .from("test@example.com")
            .to("user@example.com")
            .subject("Test")
            .text_body("Hello");
        
        let job_id = service.enqueue(email).await.unwrap();
        assert!(!job_id.to_string().is_empty());
        
        let stats = service.stats().await.unwrap();
        assert_eq!(stats.total, 1);
        assert_eq!(stats.pending, 1);
    }
    
    #[tokio::test]
    async fn test_process_next_job_with_processor() {
        use crate::providers::MockEmailProvider;
        
        let backend = MemoryBackend::new(QueueConfig::default());
        let provider = Arc::new(MockEmailProvider::new());
        let processor = Arc::new(EmailJobProcessor::new(provider));
        let service = EmailQueueService::new(backend).with_processor(processor);
        
        let email = Email::new()
            .from("test@example.com")
            .to("user@example.com")
            .subject("Test")
            .text_body("Hello");
        
        service.enqueue(email).await.unwrap();
        
        // Process job should work with processor configured
        let processed = service.process_next_job().await.unwrap();
        assert!(processed);
        
        // No more jobs to process
        let processed_again = service.process_next_job().await.unwrap();
        assert!(!processed_again);
    }
    
    #[tokio::test]
    async fn test_process_next_job_without_processor() {
        let backend = MemoryBackend::new(QueueConfig::default());
        let service = EmailQueueService::new(backend);
        
        let email = Email::new()
            .from("test@example.com")
            .to("user@example.com")
            .subject("Test")
            .text_body("Hello");
        
        service.enqueue(email).await.unwrap();
        
        // Should fail without processor
        let result = service.process_next_job().await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No email processor configured"));
    }
    
    #[tokio::test]
    async fn test_enqueue_scheduled_future_date() {
        let backend = MemoryBackend::new(QueueConfig::default());
        let service = EmailQueueService::new(backend);
        
        let email = Email::new()
            .from("test@example.com")
            .to("user@example.com")
            .subject("Scheduled Test")
            .text_body("Hello");
        
        let future_time = chrono::Utc::now() + chrono::Duration::hours(1);
        let job_id = service.enqueue_scheduled(email, future_time).await.unwrap();
        assert!(!job_id.to_string().is_empty());
    }
    
    #[tokio::test]
    async fn test_enqueue_scheduled_too_far_future() {
        let backend = MemoryBackend::new(QueueConfig::default());
        let service = EmailQueueService::new(backend);
        
        let email = Email::new()
            .from("test@example.com")
            .to("user@example.com")
            .subject("Too Far Future Test")
            .text_body("Hello");
        
        let too_far_future = chrono::Utc::now() + chrono::Duration::days(31);
        let result = service.enqueue_scheduled(email, too_far_future).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("too far in the future"));
    }
    
    #[tokio::test]
    async fn test_enqueue_scheduled_past_date() {
        let backend = MemoryBackend::new(QueueConfig::default());
        let service = EmailQueueService::new(backend);
        
        let email = Email::new()
            .from("test@example.com")
            .to("user@example.com")
            .subject("Past Date Test")
            .text_body("Hello");
        
        let past_time = chrono::Utc::now() - chrono::Duration::hours(1);
        let job_id = service.enqueue_scheduled(email, past_time).await.unwrap();
        assert!(!job_id.to_string().is_empty());
    }
    
    #[tokio::test]
    async fn test_email_worker_graceful_shutdown() {
        use crate::providers::MockEmailProvider;
        
        let backend = MemoryBackend::new(QueueConfig::default());
        let provider = Arc::new(MockEmailProvider::new());
        let processor = Arc::new(EmailJobProcessor::new(provider));
        let service = Arc::new(EmailQueueService::new(backend).with_processor(processor));
        
        let worker = EmailWorker::new(service.clone()).with_config(EmailWorkerConfig {
            poll_interval: Duration::from_millis(100),
            batch_size: 1,
            shutdown_timeout: Duration::from_secs(5),
            verbose: true,
        });
        
        let handle = worker.start().await;
        
        // Give worker time to start
        tokio::time::sleep(Duration::from_millis(50)).await;
        
        // Stop worker - should be immediate with new graceful shutdown
        let start_time = std::time::Instant::now();
        worker.stop();
        
        // Wait for worker to finish
        tokio::time::timeout(Duration::from_secs(2), handle).await.unwrap().unwrap();
        
        let shutdown_time = start_time.elapsed();
        // Should shutdown much faster than poll_interval with graceful shutdown
        assert!(shutdown_time < Duration::from_millis(200), 
                "Shutdown took {:?}, expected < 200ms", shutdown_time);
    }
    
    #[tokio::test] 
    async fn test_email_job_processor_panic_safety() {
        use crate::providers::PanickingEmailProvider;
        
        let provider = Arc::new(PanickingEmailProvider::new());
        let processor = EmailJobProcessor::new(provider);
        
        let job = EmailJob::new(Email::new()
            .from("test@example.com")
            .to("user@example.com")
            .subject("Panic Test")
            .text_body("This should handle panics"));
        
        // Should handle panic gracefully and return error
        let result = processor.process(&job).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("panicked"));
    }
}