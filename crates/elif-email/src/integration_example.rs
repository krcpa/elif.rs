//! Integration example showing complete email queue functionality
//!
//! This demonstrates how all the pieces work together:
//! - EmailJob for background processing
//! - EmailJobProcessor for handling email delivery
//! - EmailQueueService for high-level queue management
//! - EmailWorker for background processing

use crate::{
    Email, EmailJob, EmailJobProcessor, EmailQueueService, EmailWorker, EmailWorkerConfig,
    EmailPriority, EmailProvider
};
use async_trait::async_trait;
use elif_queue::{MemoryBackend, QueueConfig};
use std::sync::Arc;
use std::time::Duration;
use tracing::info;

/// Mock email provider for testing
pub struct MockEmailProvider {
    pub name: String,
    pub should_fail: bool,
    pub delay: Duration,
}

impl MockEmailProvider {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            should_fail: false,
            delay: Duration::from_millis(100),
        }
    }
    
    pub fn with_failure(mut self) -> Self {
        self.should_fail = true;
        self
    }
    
    pub fn with_delay(mut self, delay: Duration) -> Self {
        self.delay = delay;
        self
    }
}

#[async_trait]
impl crate::EmailProvider for MockEmailProvider {
    async fn send(&self, email: &Email) -> Result<crate::EmailResult, crate::EmailError> {
        info!("MockEmailProvider '{}' processing email: {}", self.name, email.id);
        
        // Simulate processing delay
        tokio::time::sleep(self.delay).await;
        
        if self.should_fail {
            return Err(crate::EmailError::provider("mock", "Mock provider configured to fail"));
        }
        
        Ok(crate::EmailResult {
            email_id: email.id,
            message_id: format!("mock-{}-{}", self.name, uuid::Uuid::new_v4()),
            sent_at: chrono::Utc::now(),
            provider: self.name.clone(),
        })
    }
    
    async fn validate_config(&self) -> Result<(), crate::EmailError> {
        Ok(())
    }
    
    fn provider_name(&self) -> &'static str {
        "mock"
    }
}

/// Complete integration example
pub async fn run_integration_example() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing for demo
    tracing_subscriber::fmt::init();
    
    info!("Starting email queue integration example");
    
    // 1. Set up email providers
    let default_provider = Arc::new(MockEmailProvider::new("primary"));
    let backup_provider = Arc::new(MockEmailProvider::new("backup"));
    
    // 2. Create email job processor with providers
    let processor = Arc::new(EmailJobProcessor::new(default_provider.clone()));
    processor.register_provider("backup".to_string(), backup_provider).await;
    
    // 3. Set up queue backend and service
    let backend = MemoryBackend::new(QueueConfig::default());
    let queue_service = Arc::new(
        EmailQueueService::new(backend)
            .with_processor(processor.clone())
    );
    
    // 4. Create some test emails
    let emails = vec![
        Email::new()
            .from("system@example.com")
            .to("user1@example.com")
            .subject("Welcome!")
            .text_body("Welcome to our service!"),
            
        Email::new()
            .from("alerts@example.com")
            .to("admin@example.com")
            .subject("System Alert")
            .text_body("Critical system alert!"),
            
        Email::new()
            .from("newsletter@example.com")
            .to("subscriber@example.com")
            .subject("Monthly Newsletter")
            .html_body("<h1>Newsletter</h1><p>Monthly updates...</p>"),
    ];
    
    // 5. Queue emails with different priorities
    info!("Queuing emails with different priorities");
    let job_ids = vec![
        queue_service.enqueue_with_priority(emails[0].clone(), EmailPriority::Normal).await?,
        queue_service.enqueue_with_priority(emails[1].clone(), EmailPriority::Critical).await?,
        queue_service.enqueue_with_priority(emails[2].clone(), EmailPriority::Low).await?,
    ];
    
    // 6. Schedule an email for later
    let scheduled_email = Email::new()
        .from("reminders@example.com")
        .to("user@example.com")
        .subject("Scheduled Reminder")
        .text_body("This email was scheduled!");
        
    let send_at = chrono::Utc::now() + chrono::Duration::seconds(2);
    let scheduled_job_id = queue_service.enqueue_scheduled(scheduled_email, send_at).await?;
    info!("Scheduled email job: {}", scheduled_job_id);
    
    // 7. Queue a batch of emails
    let batch_emails: Vec<Email> = (0..5).map(|i| {
        Email::new()
            .from("batch@example.com")
            .to(format!("user{}@example.com", i))
            .subject("Batch Email")
            .text_body(format!("Batch email #{}", i))
    }).collect();
    
    let batch_job_ids = queue_service.enqueue_batch(batch_emails).await?;
    info!("Queued {} batch emails", batch_job_ids.len());
    
    // 8. Check queue statistics
    let stats = queue_service.stats().await?;
    info!("Queue stats: pending={}, total={}", stats.pending, stats.total);
    
    // 9. Start email worker for background processing
    let worker_config = EmailWorkerConfig {
        poll_interval: Duration::from_millis(500),
        batch_size: 3,
        shutdown_timeout: Duration::from_secs(10),
        verbose: true,
    };
    
    let worker = EmailWorker::new(queue_service.clone()).with_config(worker_config);
    let worker_handle = worker.start().await;
    
    info!("Email worker started, processing jobs...");
    
    // 10. Wait for some processing to happen
    tokio::time::sleep(Duration::from_secs(5)).await;
    
    // 11. Check delivery status
    for job_id in &job_ids {
        if let Ok(Some(job_entry)) = queue_service.get_job(*job_id).await {
            info!("Job {} status: {:?}", job_id, job_entry.state());
        }
    }
    
    // 12. Check final statistics
    let final_stats = queue_service.stats().await?;
    info!("Final stats: completed={}, failed={}", final_stats.completed, final_stats.failed);
    
    // 13. Check delivery tracking
    let delivery_statuses = processor.get_all_delivery_statuses().await;
    info!("Tracked {} email deliveries", delivery_statuses.len());
    
    for status in &delivery_statuses {
        info!("Email {} delivery status: {:?}, attempts: {}", 
            status.email_id, status.status, status.attempts.len());
    }
    
    // 14. Stop the worker
    worker.stop().await;
    worker_handle.abort();
    
    info!("Email queue integration example completed successfully!");
    
    Ok(())
}

/// Demonstrate retry logic and failure handling
pub async fn run_failure_handling_example() -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting failure handling example");
    
    // Create a provider that will fail
    let failing_provider = Arc::new(MockEmailProvider::new("failing").with_failure());
    let processor = Arc::new(EmailJobProcessor::new(failing_provider));
    
    let backend = MemoryBackend::new(QueueConfig::default());
    let queue_service = Arc::new(
        EmailQueueService::new(backend)
            .with_processor(processor.clone())
    );
    
    // Create an email job with custom retry settings
    let email = Email::new()
        .from("test@example.com")
        .to("user@example.com")
        .subject("Test Retry Logic")
        .text_body("This email will fail and retry");
    
    let job_id = queue_service.enqueue_with_priority(email, EmailPriority::High).await?;
    info!("Queued failing email job: {}", job_id);
    
    // Process the job (it will fail and retry)
    let worker_config = EmailWorkerConfig {
        poll_interval: Duration::from_millis(100),
        batch_size: 1,
        shutdown_timeout: Duration::from_secs(5),
        verbose: true,
    };
    
    let worker = EmailWorker::new(queue_service.clone()).with_config(worker_config);
    let worker_handle = worker.start().await;
    
    // Wait for retry attempts
    tokio::time::sleep(Duration::from_secs(8)).await;
    
    // Check final job state
    if let Ok(Some(job_entry)) = queue_service.get_job(job_id).await {
        info!("Final job state: {:?}, attempts: {}", job_entry.state(), job_entry.attempts());
    }
    
    // Check delivery tracking
    let delivery_statuses = processor.get_all_delivery_statuses().await;
    for status in &delivery_statuses {
        info!("Delivery attempts for {}: {}", status.email_id, status.attempts.len());
        for attempt in &status.attempts {
            info!("  Attempt {}: {:?}", attempt.attempt_number, attempt.result);
        }
    }
    
    worker.stop().await;
    worker_handle.abort();
    
    info!("Failure handling example completed");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::EmailProvider;
    
    #[tokio::test]
    async fn test_mock_email_provider() {
        let provider = MockEmailProvider::new("test");
        
        let email = Email::new()
            .from("test@example.com")
            .to("user@example.com")
            .subject("Test")
            .text_body("Hello");
        
        let result = provider.send(&email).await;
        assert!(result.is_ok());
        
        let email_result = result.unwrap();
        assert_eq!(email_result.email_id, email.id);
        assert_eq!(email_result.provider, "test");
    }
    
    #[tokio::test]
    async fn test_failing_provider() {
        let provider = MockEmailProvider::new("failing").with_failure();
        
        let email = Email::new()
            .from("test@example.com")
            .to("user@example.com")
            .subject("Test")
            .text_body("Hello");
        
        let result = provider.send(&email).await;
        assert!(result.is_err());
    }
    
    #[tokio::test]
    async fn test_email_job_processor_setup() {
        let provider = Arc::new(MockEmailProvider::new("primary"));
        let processor = EmailJobProcessor::new(provider.clone());
        
        // Register additional provider
        let backup_provider = Arc::new(MockEmailProvider::new("backup"));
        processor.register_provider("backup".to_string(), backup_provider).await;
        
        // Test that we can create the processor without errors
        // (We can't test the provider directly due to private fields, but creation is sufficient)
    }
}