//! Job scheduling system with cron expression support
//!
//! This module provides advanced job scheduling capabilities including:
//! - Cron expression parsing and validation
//! - Recurring job scheduling
//! - Delayed execution with retry logic
//! - Multiple retry strategies with backoff patterns

use std::time::Duration;
use std::str::FromStr;
use std::sync::Arc;
use chrono::{DateTime, Utc};
use cron::Schedule;
use serde::{Serialize, Deserialize};
use thiserror::Error;
use crate::{Job, JobEntry, Priority, QueueResult, QueueError};

/// Scheduling errors
#[derive(Error, Debug)]
pub enum ScheduleError {
    #[error("Invalid cron expression: {0}")]
    InvalidCron(String),
    
    #[error("Schedule not found: {0}")]
    ScheduleNotFound(String),
    
    #[error("Invalid retry configuration: {0}")]
    InvalidRetryConfig(String),
    
    #[error("Queue error: {0}")]
    Queue(#[from] QueueError),
}

/// Result type for scheduling operations
pub type ScheduleResult<T> = Result<T, ScheduleError>;

/// Cron expression wrapper with validation
#[derive(Debug, Clone, Serialize)]
pub struct CronExpression {
    expression: String,
    #[serde(skip)]
    schedule: Option<Schedule>,
}

impl CronExpression {
    /// Create a new cron expression from a string
    /// 
    /// Supports standard 6-field cron format:
    /// - `0 * * * * *` (every minute)
    /// - `0 0 0 * * *` (daily at midnight)
    /// - `0 0 */6 * * *` (every 6 hours)
    /// - `0 0 9-17 * * 1-5` (weekdays 9-5)
    pub fn new(expression: &str) -> ScheduleResult<Self> {
        let schedule = Schedule::from_str(expression)
            .map_err(|e| ScheduleError::InvalidCron(format!("{}: {}", expression, e)))?;
            
        Ok(CronExpression {
            expression: expression.to_string(),
            schedule: Some(schedule),
        })
    }
    
    /// Get the raw cron expression string
    pub fn expression(&self) -> &str {
        &self.expression
    }
    
    /// Get the next run time after the given datetime
    pub fn next_run_time(&self, after: DateTime<Utc>) -> Option<DateTime<Utc>> {
        self.schedule.as_ref()?.after(&after).next()
    }
    
    /// Get the next N run times after the given datetime
    pub fn next_run_times(&self, after: DateTime<Utc>, count: usize) -> Vec<DateTime<Utc>> {
        self.schedule.as_ref()
            .map(|s| s.after(&after).take(count).collect())
            .unwrap_or_default()
    }
    
    /// Check if this schedule should run at the given time
    pub fn should_run(&self, at: DateTime<Utc>) -> bool {
        if let Some(next) = self.next_run_time(at - chrono::Duration::seconds(1)) {
            (next - at).num_seconds().abs() < 30 // 30 second window
        } else {
            false
        }
    }
}

// Custom serialization to handle the non-serializable Schedule field
impl<'de> Deserialize<'de> for CronExpression {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct CronExpressionData {
            expression: String,
        }
        
        let data = CronExpressionData::deserialize(deserializer)?;
        CronExpression::new(&data.expression)
            .map_err(|e| serde::de::Error::custom(e.to_string()))
    }
}

/// Retry strategy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RetryStrategy {
    /// Fixed delay between retries
    Fixed {
        delay: Duration,
        max_attempts: u32,
    },
    /// Exponential backoff with optional jitter
    Exponential {
        initial_delay: Duration,
        multiplier: f64,
        max_delay: Duration,
        max_attempts: u32,
        jitter: bool,
    },
    /// Linear backoff
    Linear {
        initial_delay: Duration,
        increment: Duration,
        max_delay: Duration,
        max_attempts: u32,
    },
    /// Custom retry delays
    Custom {
        delays: Vec<Duration>,
    },
}

impl Default for RetryStrategy {
    fn default() -> Self {
        RetryStrategy::Exponential {
            initial_delay: Duration::from_secs(1),
            multiplier: 2.0,
            max_delay: Duration::from_secs(300), // 5 minutes
            max_attempts: 3,
            jitter: true,
        }
    }
}

impl RetryStrategy {
    /// Calculate retry delay for the given attempt
    pub fn delay_for_attempt(&self, attempt: u32) -> Option<Duration> {
        use rand::Rng;
        
        match self {
            RetryStrategy::Fixed { delay, max_attempts } => {
                if attempt < *max_attempts {
                    Some(*delay)
                } else {
                    None
                }
            }
            RetryStrategy::Exponential {
                initial_delay,
                multiplier,
                max_delay,
                max_attempts,
                jitter,
            } => {
                if attempt >= *max_attempts {
                    return None;
                }
                
                let delay = initial_delay.as_secs_f64() * multiplier.powi(attempt as i32);
                let delay = delay.min(max_delay.as_secs_f64());
                
                let delay = if *jitter {
                    // Add ±25% jitter
                    let mut rng = rand::thread_rng();
                    let jitter_factor = rng.gen_range(0.75..1.25);
                    delay * jitter_factor
                } else {
                    delay
                };
                
                Some(Duration::from_secs_f64(delay))
            }
            RetryStrategy::Linear {
                initial_delay,
                increment,
                max_delay,
                max_attempts,
            } => {
                if attempt >= *max_attempts {
                    return None;
                }
                
                let delay = initial_delay.as_secs() + (increment.as_secs() * attempt as u64);
                let delay = delay.min(max_delay.as_secs());
                Some(Duration::from_secs(delay))
            }
            RetryStrategy::Custom { delays } => {
                delays.get(attempt as usize).copied()
            }
        }
    }
    
    /// Get maximum number of retry attempts
    pub fn max_attempts(&self) -> u32 {
        match self {
            RetryStrategy::Fixed { max_attempts, .. } => *max_attempts,
            RetryStrategy::Exponential { max_attempts, .. } => *max_attempts,
            RetryStrategy::Linear { max_attempts, .. } => *max_attempts,
            RetryStrategy::Custom { delays } => delays.len() as u32,
        }
    }
}

/// Scheduled job configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledJob {
    /// Unique identifier for this scheduled job
    pub id: String,
    /// Cron expression for when to run
    pub cron: CronExpression,
    /// Job type to execute
    pub job_type: String,
    /// Job payload (serialized)
    pub payload: serde_json::Value,
    /// Job priority
    pub priority: Priority,
    /// Retry strategy configuration
    pub retry_strategy: RetryStrategy,
    /// Job timeout
    pub timeout: Duration,
    /// Whether this schedule is active
    pub enabled: bool,
    /// Optional description
    pub description: Option<String>,
    /// Next scheduled run time
    pub next_run: Option<DateTime<Utc>>,
    /// Last execution time
    pub last_run: Option<DateTime<Utc>>,
    /// When this schedule was created
    pub created_at: DateTime<Utc>,
}

impl ScheduledJob {
    /// Create a new scheduled job
    pub fn new<T: Job>(
        id: String,
        cron_expr: &str,
        job: T,
        priority: Option<Priority>,
        retry_strategy: Option<RetryStrategy>,
    ) -> ScheduleResult<Self> {
        let cron = CronExpression::new(cron_expr)?;
        let now = Utc::now();
        let next_run = cron.next_run_time(now);
        
        let job_type = job.job_type().to_string();
        let timeout = job.timeout();
        let payload = serde_json::to_value(job)
            .map_err(|e| ScheduleError::Queue(QueueError::Serialization(e)))?;
        
        Ok(ScheduledJob {
            id,
            cron,
            job_type,
            payload,
            priority: priority.unwrap_or_default(),
            retry_strategy: retry_strategy.unwrap_or_default(),
            timeout,
            enabled: true,
            description: None,
            next_run,
            last_run: None,
            created_at: now,
        })
    }
    
    /// Update next run time
    pub fn update_next_run(&mut self) {
        let after = self.last_run.unwrap_or_else(Utc::now);
        self.next_run = self.cron.next_run_time(after);
    }
    
    /// Check if this job should run now
    pub fn should_run(&self) -> bool {
        if !self.enabled {
            return false;
        }
        
        if let Some(next_run) = self.next_run {
            next_run <= Utc::now()
        } else {
            false
        }
    }
    
    /// Mark as executed
    pub fn mark_executed(&mut self) {
        self.last_run = Some(Utc::now());
        self.update_next_run();
    }
    
    /// Create a job entry for execution
    pub fn create_job_entry(&self) -> QueueResult<JobEntry> {
        JobEntry::new_with_job_type(
            self.job_type.clone(),
            self.payload.clone(),
            Some(self.priority),
            None, // No delay - execute immediately
            self.retry_strategy.max_attempts(),
        )
    }
}

/// Wrapper to implement Job trait for scheduled jobs
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ScheduledJobWrapper {
    job_type: String,
    payload: serde_json::Value,
    max_retries: u32,
    timeout: Duration,
}

#[async_trait::async_trait]
impl Job for ScheduledJobWrapper {
    async fn execute(&self) -> crate::JobResult<()> {
        // This should not be called directly - it's just for creating JobEntry
        Ok(())
    }
    
    fn job_type(&self) -> &'static str {
        // This is a bit of a hack - we need to return a &'static str
        // In practice, the job_type will be used from the JobEntry
        "scheduled_job_wrapper"
    }
    
    fn max_retries(&self) -> u32 {
        self.max_retries
    }
    
    fn timeout(&self) -> Duration {
        self.timeout
    }
}

/// Job scheduler manages recurring jobs and their execution
pub struct JobScheduler<B: crate::QueueBackend> {
    backend: std::sync::Arc<B>,
    schedules: std::sync::Arc<parking_lot::RwLock<std::collections::HashMap<String, ScheduledJob>>>,
    running: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl<B: crate::QueueBackend + 'static> JobScheduler<B> {
    /// Create a new job scheduler
    pub fn new(backend: std::sync::Arc<B>) -> Self {
        Self {
            backend,
            schedules: std::sync::Arc::new(parking_lot::RwLock::new(std::collections::HashMap::new())),
            running: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }
    
    /// Add a scheduled job
    pub fn add_schedule(&self, schedule: ScheduledJob) -> ScheduleResult<()> {
        let mut schedules = self.schedules.write();
        schedules.insert(schedule.id.clone(), schedule);
        Ok(())
    }
    
    /// Remove a scheduled job
    pub fn remove_schedule(&self, id: &str) -> ScheduleResult<bool> {
        let mut schedules = self.schedules.write();
        Ok(schedules.remove(id).is_some())
    }
    
    /// Get a scheduled job by ID
    pub fn get_schedule(&self, id: &str) -> Option<ScheduledJob> {
        let schedules = self.schedules.read();
        schedules.get(id).cloned()
    }
    
    /// List all scheduled jobs
    pub fn list_schedules(&self) -> Vec<ScheduledJob> {
        let schedules = self.schedules.read();
        schedules.values().cloned().collect()
    }
    
    /// Enable or disable a scheduled job
    pub fn set_schedule_enabled(&self, id: &str, enabled: bool) -> ScheduleResult<bool> {
        let mut schedules = self.schedules.write();
        if let Some(schedule) = schedules.get_mut(id) {
            schedule.enabled = enabled;
            Ok(true)
        } else {
            Ok(false)
        }
    }
    
    /// Start the scheduler loop
    pub async fn start(&self) -> ScheduleResult<()> {
        self.running.store(true, std::sync::atomic::Ordering::SeqCst);
        
        let backend = self.backend.clone();
        let schedules = self.schedules.clone();
        let running = self.running.clone();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30)); // Check every 30 seconds
            
            while running.load(std::sync::atomic::Ordering::SeqCst) {
                interval.tick().await;
                
                // Get schedules that should run
                let mut due_schedules = Vec::new();
                {
                    let mut schedules_guard = schedules.write();
                    for schedule in schedules_guard.values_mut() {
                        if schedule.should_run() {
                            schedule.mark_executed();
                            due_schedules.push(schedule.clone());
                        }
                    }
                }
                
                // Enqueue jobs for due schedules
                for schedule in due_schedules {
                    if let Ok(job_entry) = schedule.create_job_entry() {
                        if let Err(e) = backend.enqueue(job_entry).await {
                            tracing::error!("Failed to enqueue scheduled job {}: {}", schedule.id, e);
                        } else {
                            tracing::info!("Enqueued scheduled job: {}", schedule.id);
                        }
                    }
                }
            }
        });
        
        Ok(())
    }
    
    /// Stop the scheduler
    pub fn stop(&self) {
        self.running.store(false, std::sync::atomic::Ordering::SeqCst);
    }
    
    /// Check if scheduler is running
    pub fn is_running(&self) -> bool {
        self.running.load(std::sync::atomic::Ordering::SeqCst)
    }
    
    /// Get dead letter queue entries (jobs that failed permanently)
    pub async fn get_dead_jobs(&self, limit: Option<usize>) -> QueueResult<Vec<crate::JobEntry>> {
        self.backend.get_jobs_by_state(crate::JobState::Dead, limit).await
    }
    
    /// Requeue a dead job (reset attempts and change to Pending state)
    pub async fn requeue_dead_job(&self, job_id: crate::JobId) -> QueueResult<bool> {
        if let Some(job) = self.backend.get_job(job_id).await? {
            if job.state() == &crate::JobState::Dead {
                // Use atomic requeue operation
                self.backend.requeue_job(job_id, job).await
            } else {
                Ok(false)
            }
        } else {
            Ok(false)
        }
    }
    
    /// Clear all dead letter queue entries
    pub async fn clear_dead_jobs(&self) -> QueueResult<u64> {
        let dead_jobs = self.get_dead_jobs(None).await?;
        let count = dead_jobs.len() as u64;
        
        for job in dead_jobs {
            self.backend.remove_job(job.id()).await?;
        }
        
        Ok(count)
    }
}


/// Helper functions for creating common cron expressions
pub mod cron_presets {
    use super::CronExpression;
    
    /// Every minute
    pub fn every_minute() -> CronExpression {
        CronExpression::new("0 * * * * *").expect("Invalid 'every_minute' cron preset")
    }
    
    /// Every 5 minutes
    pub fn every_5_minutes() -> CronExpression {
        CronExpression::new("0 */5 * * * *").unwrap()
    }
    
    /// Every 15 minutes
    pub fn every_15_minutes() -> CronExpression {
        CronExpression::new("0 */15 * * * *").unwrap()
    }
    
    /// Every 30 minutes
    pub fn every_30_minutes() -> CronExpression {
        CronExpression::new("0 */30 * * * *").unwrap()
    }
    
    /// Every hour at minute 0
    pub fn hourly() -> CronExpression {
        CronExpression::new("0 0 * * * *").unwrap()
    }
    
    /// Daily at midnight
    pub fn daily() -> CronExpression {
        CronExpression::new("0 0 0 * * *").unwrap()
    }
    
    /// Weekly on Sunday at midnight
    pub fn weekly() -> CronExpression {
        CronExpression::new("0 0 0 * * SUN").unwrap()
    }
    
    /// Monthly on the 1st at midnight
    pub fn monthly() -> CronExpression {
        CronExpression::new("0 0 0 1 * *").unwrap()
    }
    
    /// Weekdays at 9 AM
    pub fn weekdays_at_9am() -> CronExpression {
        CronExpression::new("0 0 9 * * 1-5").unwrap()
    }
    
    /// Custom cron expression
    pub fn custom(expression: &str) -> Result<CronExpression, super::ScheduleError> {
        CronExpression::new(expression)
    }
}

/// Cancellation token for cooperative job cancellation
#[derive(Debug, Clone)]
pub struct CancellationToken {
    inner: Arc<std::sync::atomic::AtomicBool>,
}

impl CancellationToken {
    /// Create a new cancellation token
    pub fn new() -> Self {
        Self {
            inner: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }
    
    /// Cancel the operation
    pub fn cancel(&self) {
        self.inner.store(true, std::sync::atomic::Ordering::SeqCst);
    }
    
    /// Check if cancellation was requested
    pub fn is_cancelled(&self) -> bool {
        self.inner.load(std::sync::atomic::Ordering::SeqCst)
    }
    
    /// Wait for cancellation signal
    pub async fn wait_for_cancellation(&self) {
        while !self.is_cancelled() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }
    
    /// Create a future that completes when cancelled
    pub async fn cancelled(&self) {
        self.wait_for_cancellation().await;
    }
}

impl Default for CancellationToken {
    fn default() -> Self {
        Self::new()
    }
}

/// Job cancellation manager
#[derive(Debug)]
pub struct JobCancellationManager {
    active_tokens: Arc<parking_lot::RwLock<std::collections::HashMap<crate::JobId, CancellationToken>>>,
}

impl JobCancellationManager {
    /// Create a new cancellation manager
    pub fn new() -> Self {
        Self {
            active_tokens: Arc::new(parking_lot::RwLock::new(std::collections::HashMap::new())),
        }
    }
    
    /// Register a new job with cancellation token
    pub fn register_job(&self, job_id: crate::JobId) -> CancellationToken {
        let token = CancellationToken::new();
        self.active_tokens.write().insert(job_id, token.clone());
        token
    }
    
    /// Cancel a specific job
    pub fn cancel_job(&self, job_id: crate::JobId) -> bool {
        if let Some(token) = self.active_tokens.read().get(&job_id) {
            token.cancel();
            true
        } else {
            false
        }
    }
    
    /// Cancel all active jobs
    pub fn cancel_all(&self) {
        let tokens = self.active_tokens.read();
        for token in tokens.values() {
            token.cancel();
        }
    }
    
    /// Remove completed job from tracking
    pub fn unregister_job(&self, job_id: crate::JobId) {
        self.active_tokens.write().remove(&job_id);
    }
    
    /// Get active job count
    pub fn active_job_count(&self) -> usize {
        self.active_tokens.read().len()
    }
    
    /// List all active job IDs
    pub fn active_jobs(&self) -> Vec<crate::JobId> {
        self.active_tokens.read().keys().cloned().collect()
    }
}

impl Default for JobCancellationManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Extended job trait with cancellation support
#[async_trait::async_trait]
pub trait CancellableJob: Job {
    /// Execute the job with cancellation support
    async fn execute_with_cancellation(&self, token: &CancellationToken) -> crate::JobResult<()>;
}

/// Job execution metrics and statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct JobMetrics {
    /// Total number of jobs scheduled
    pub total_scheduled: u64,
    /// Total number of jobs executed
    pub total_executed: u64,
    /// Total number of successful jobs
    pub successful_jobs: u64,
    /// Total number of failed jobs
    pub failed_jobs: u64,
    /// Total number of retried jobs
    pub retried_jobs: u64,
    /// Total number of timed out jobs
    pub timeout_jobs: u64,
    /// Total number of cancelled jobs
    pub cancelled_jobs: u64,
    /// Average execution time in milliseconds
    pub avg_execution_time_ms: f64,
    /// Min execution time in milliseconds
    pub min_execution_time_ms: u64,
    /// Max execution time in milliseconds
    pub max_execution_time_ms: u64,
    /// Jobs by priority
    pub jobs_by_priority: std::collections::HashMap<String, u64>,
    /// Jobs by type
    pub jobs_by_type: std::collections::HashMap<String, u64>,
    /// Success rate (0.0 - 1.0)
    pub success_rate: f64,
    /// Average retry attempts for failed jobs
    pub avg_retry_attempts: f64,
    /// Last reset timestamp
    pub last_reset: DateTime<Utc>,
}

impl JobMetrics {
    /// Create new empty metrics
    pub fn new() -> Self {
        Self {
            last_reset: Utc::now(),
            ..Default::default()
        }
    }
    
    /// Record a scheduled job
    pub fn record_scheduled(&mut self, job_type: &str, priority: Priority) {
        self.total_scheduled += 1;
        *self.jobs_by_type.entry(job_type.to_string()).or_insert(0) += 1;
        *self.jobs_by_priority.entry(format!("{:?}", priority)).or_insert(0) += 1;
    }
    
    /// Record job execution start
    pub fn record_execution_start(&mut self) {
        self.total_executed += 1;
    }
    
    /// Record successful job completion
    pub fn record_success(&mut self, execution_time_ms: u64) {
        self.successful_jobs += 1;
        self.update_execution_time(execution_time_ms);
        self.update_success_rate();
    }
    
    /// Record job failure
    pub fn record_failure(&mut self, execution_time_ms: u64, retry_attempts: u32) {
        self.failed_jobs += 1;
        self.update_execution_time(execution_time_ms);
        self.update_success_rate();
        self.update_retry_attempts(retry_attempts);
    }
    
    /// Record job retry
    pub fn record_retry(&mut self) {
        self.retried_jobs += 1;
    }
    
    /// Record job timeout
    pub fn record_timeout(&mut self, execution_time_ms: u64) {
        self.timeout_jobs += 1;
        self.update_execution_time(execution_time_ms);
    }
    
    /// Record job cancellation
    pub fn record_cancellation(&mut self, execution_time_ms: u64) {
        self.cancelled_jobs += 1;
        self.update_execution_time(execution_time_ms);
    }
    
    /// Reset all metrics
    pub fn reset(&mut self) {
        *self = Self::new();
    }
    
    /// Update execution time statistics
    fn update_execution_time(&mut self, execution_time_ms: u64) {
        if self.min_execution_time_ms == 0 || execution_time_ms < self.min_execution_time_ms {
            self.min_execution_time_ms = execution_time_ms;
        }
        if execution_time_ms > self.max_execution_time_ms {
            self.max_execution_time_ms = execution_time_ms;
        }
        
        // Update average based on completed jobs, not total executions
        let completed_jobs = self.successful_jobs + self.failed_jobs + self.timeout_jobs + self.cancelled_jobs;
        if completed_jobs == 1 {
            self.avg_execution_time_ms = execution_time_ms as f64;
        } else {
            let total_time = self.avg_execution_time_ms * (completed_jobs - 1) as f64;
            self.avg_execution_time_ms = (total_time + execution_time_ms as f64) / completed_jobs as f64;
        }
    }
    
    /// Update success rate
    fn update_success_rate(&mut self) {
        let total_completed = self.successful_jobs + self.failed_jobs + self.timeout_jobs + self.cancelled_jobs;
        if total_completed > 0 {
            self.success_rate = self.successful_jobs as f64 / total_completed as f64;
        }
    }
    
    /// Update retry attempts average
    fn update_retry_attempts(&mut self, attempts: u32) {
        if self.failed_jobs == 1 {
            self.avg_retry_attempts = attempts as f64;
        } else {
            let total_attempts = self.avg_retry_attempts * (self.failed_jobs - 1) as f64;
            self.avg_retry_attempts = (total_attempts + attempts as f64) / self.failed_jobs as f64;
        }
    }
}

/// Job metrics collector
#[derive(Debug)]
pub struct JobMetricsCollector {
    metrics: Arc<parking_lot::RwLock<JobMetrics>>,
    active_executions: Arc<parking_lot::RwLock<std::collections::HashMap<crate::JobId, std::time::Instant>>>,
}

impl JobMetricsCollector {
    /// Create a new metrics collector
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(parking_lot::RwLock::new(JobMetrics::new())),
            active_executions: Arc::new(parking_lot::RwLock::new(std::collections::HashMap::new())),
        }
    }
    
    /// Record a job being scheduled
    pub fn record_job_scheduled(&self, job_type: &str, priority: Priority) {
        let mut metrics = self.metrics.write();
        metrics.record_scheduled(job_type, priority);
    }
    
    /// Record job execution start
    pub fn record_execution_start(&self, job_id: crate::JobId) {
        let mut metrics = self.metrics.write();
        let mut executions = self.active_executions.write();
        
        metrics.record_execution_start();
        executions.insert(job_id, std::time::Instant::now());
    }
    
    /// Record job completion (success)
    pub fn record_job_success(&self, job_id: crate::JobId) {
        let execution_time = self.get_and_remove_execution_time(job_id);
        let mut metrics = self.metrics.write();
        metrics.record_success(execution_time);
    }
    
    /// Record job failure
    pub fn record_job_failure(&self, job_id: crate::JobId, retry_attempts: u32) {
        let execution_time = self.get_and_remove_execution_time(job_id);
        let mut metrics = self.metrics.write();
        metrics.record_failure(execution_time, retry_attempts);
    }
    
    /// Record job retry
    pub fn record_job_retry(&self, _job_id: crate::JobId) {
        // Don't remove execution time for retries
        let mut metrics = self.metrics.write();
        metrics.record_retry();
    }
    
    /// Record job timeout
    pub fn record_job_timeout(&self, job_id: crate::JobId) {
        let execution_time = self.get_and_remove_execution_time(job_id);
        let mut metrics = self.metrics.write();
        metrics.record_timeout(execution_time);
    }
    
    /// Record job cancellation
    pub fn record_job_cancellation(&self, job_id: crate::JobId) {
        let execution_time = self.get_and_remove_execution_time(job_id);
        let mut metrics = self.metrics.write();
        metrics.record_cancellation(execution_time);
    }
    
    /// Get current metrics snapshot
    pub fn get_metrics(&self) -> JobMetrics {
        self.metrics.read().clone()
    }
    
    /// Reset all metrics
    pub fn reset_metrics(&self) {
        let mut metrics = self.metrics.write();
        let mut executions = self.active_executions.write();
        
        metrics.reset();
        executions.clear();
    }
    
    /// Get and remove execution time for a job
    fn get_and_remove_execution_time(&self, job_id: crate::JobId) -> u64 {
        let mut executions = self.active_executions.write();
        if let Some(start_time) = executions.remove(&job_id) {
            start_time.elapsed().as_millis() as u64
        } else {
            0
        }
    }
    
    /// Get active executions count
    pub fn active_executions_count(&self) -> usize {
        self.active_executions.read().len()
    }
}

impl Default for JobMetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use crate::{MemoryBackend, QueueConfig};
    
    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    struct TestJob {
        message: String,
    }
    
    #[async_trait::async_trait]
    impl Job for TestJob {
        async fn execute(&self) -> crate::JobResult<()> {
            println!("Executing test job: {}", self.message);
            Ok(())
        }
        
        fn job_type(&self) -> &'static str {
            "test_job"
        }
    }
    
    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    struct CancellableTestJob {
        message: String,
        sleep_duration: Duration,
    }
    
    #[async_trait::async_trait]
    impl Job for CancellableTestJob {
        async fn execute(&self) -> crate::JobResult<()> {
            tokio::time::sleep(self.sleep_duration).await;
            Ok(())
        }
        
        fn job_type(&self) -> &'static str {
            "cancellable_test_job"
        }
    }
    
    #[async_trait::async_trait]
    impl CancellableJob for CancellableTestJob {
        async fn execute_with_cancellation(&self, token: &CancellationToken) -> crate::JobResult<()> {
            tokio::select! {
                _ = tokio::time::sleep(self.sleep_duration) => {
                    println!("Job completed: {}", self.message);
                    Ok(())
                }
                _ = token.cancelled() => {
                    println!("Job cancelled: {}", self.message);
                    Err("Job was cancelled".into())
                }
            }
        }
    }
    
    #[test]
    fn test_cron_expression_validation() {
        // Valid cron expressions
        assert!(CronExpression::new("0 0 0 * * *").is_ok()); // Daily at midnight
        assert!(CronExpression::new("0 */5 * * * *").is_ok()); // Every 5 minutes
        assert!(CronExpression::new("0 0 9-17 * * 1-5").is_ok()); // Weekdays 9-5
        
        // Invalid cron expressions
        assert!(CronExpression::new("invalid").is_err());
        assert!(CronExpression::new("* * * * *").is_err()); // 5 fields not supported
    }
    
    #[test]
    fn test_cron_next_run_time() {
        let cron = CronExpression::new("0 0 0 * * *").unwrap(); // Daily at midnight
        let now = Utc::now();
        let next = cron.next_run_time(now);
        
        assert!(next.is_some());
        assert!(next.unwrap() > now);
    }
    
    #[test]
    fn test_cron_presets() {
        // Test preset expressions
        assert!(cron_presets::every_minute().next_run_time(Utc::now()).is_some());
        assert!(cron_presets::hourly().next_run_time(Utc::now()).is_some());
        assert!(cron_presets::daily().next_run_time(Utc::now()).is_some());
        assert!(cron_presets::weekly().next_run_time(Utc::now()).is_some());
        assert!(cron_presets::monthly().next_run_time(Utc::now()).is_some());
        assert!(cron_presets::weekdays_at_9am().next_run_time(Utc::now()).is_some());
    }
    
    #[test]
    fn test_retry_strategy_exponential() {
        let strategy = RetryStrategy::Exponential {
            initial_delay: Duration::from_secs(1),
            multiplier: 2.0,
            max_delay: Duration::from_secs(60),
            max_attempts: 3,
            jitter: false,
        };
        
        assert_eq!(strategy.delay_for_attempt(0), Some(Duration::from_secs(1)));
        assert_eq!(strategy.delay_for_attempt(1), Some(Duration::from_secs(2)));
        assert_eq!(strategy.delay_for_attempt(2), Some(Duration::from_secs(4)));
        assert_eq!(strategy.delay_for_attempt(3), None); // Exceeds max attempts
        assert_eq!(strategy.max_attempts(), 3);
    }
    
    #[test]
    fn test_retry_strategy_linear() {
        let strategy = RetryStrategy::Linear {
            initial_delay: Duration::from_secs(5),
            increment: Duration::from_secs(10),
            max_delay: Duration::from_secs(60),
            max_attempts: 4,
        };
        
        assert_eq!(strategy.delay_for_attempt(0), Some(Duration::from_secs(5)));
        assert_eq!(strategy.delay_for_attempt(1), Some(Duration::from_secs(15)));
        assert_eq!(strategy.delay_for_attempt(2), Some(Duration::from_secs(25)));
        assert_eq!(strategy.delay_for_attempt(3), Some(Duration::from_secs(35)));
        assert_eq!(strategy.delay_for_attempt(4), None); // Exceeds max attempts
        assert_eq!(strategy.max_attempts(), 4);
    }
    
    #[test]
    fn test_retry_strategy_fixed() {
        let strategy = RetryStrategy::Fixed {
            delay: Duration::from_secs(10),
            max_attempts: 2,
        };
        
        assert_eq!(strategy.delay_for_attempt(0), Some(Duration::from_secs(10)));
        assert_eq!(strategy.delay_for_attempt(1), Some(Duration::from_secs(10)));
        assert_eq!(strategy.delay_for_attempt(2), None); // Exceeds max attempts
        assert_eq!(strategy.max_attempts(), 2);
    }
    
    #[test]
    fn test_retry_strategy_custom() {
        let strategy = RetryStrategy::Custom {
            delays: vec![
                Duration::from_secs(1),
                Duration::from_secs(5),
                Duration::from_secs(30),
            ],
        };
        
        assert_eq!(strategy.delay_for_attempt(0), Some(Duration::from_secs(1)));
        assert_eq!(strategy.delay_for_attempt(1), Some(Duration::from_secs(5)));
        assert_eq!(strategy.delay_for_attempt(2), Some(Duration::from_secs(30)));
        assert_eq!(strategy.delay_for_attempt(3), None); // No more delays
        assert_eq!(strategy.max_attempts(), 3);
    }
    
    #[test]
    fn test_scheduled_job_creation() {
        let job = TestJob {
            message: "Hello, World!".to_string(),
        };
        
        let scheduled = ScheduledJob::new(
            "test_schedule".to_string(),
            "0 0 0 * * *", // Daily at midnight
            job,
            Some(Priority::High),
            None,
        ).unwrap();
        
        assert_eq!(scheduled.id, "test_schedule");
        assert_eq!(scheduled.job_type, "test_job");
        assert_eq!(scheduled.priority, Priority::High);
        assert!(scheduled.enabled);
        assert!(scheduled.next_run.is_some());
    }
    
    #[tokio::test]
    async fn test_job_scheduler_basic() {
        let backend = std::sync::Arc::new(MemoryBackend::new(QueueConfig::default()));
        let scheduler = JobScheduler::new(backend);
        
        let job = TestJob {
            message: "Scheduled job".to_string(),
        };
        
        let scheduled = ScheduledJob::new(
            "test_schedule".to_string(),
            "0 * * * * *", // Every minute (for testing)
            job,
            Some(Priority::Normal),
            None,
        ).unwrap();
        
        scheduler.add_schedule(scheduled).unwrap();
        
        let schedules = scheduler.list_schedules();
        assert_eq!(schedules.len(), 1);
        assert_eq!(schedules[0].id, "test_schedule");
        
        // Test retrieval
        let retrieved = scheduler.get_schedule("test_schedule");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, "test_schedule");
        
        // Test removal
        assert!(scheduler.remove_schedule("test_schedule").unwrap());
        assert!(scheduler.get_schedule("test_schedule").is_none());
    }
    
    #[test]
    fn test_cancellation_token() {
        let token = CancellationToken::new();
        
        // Initially not cancelled
        assert!(!token.is_cancelled());
        
        // Cancel the token
        token.cancel();
        assert!(token.is_cancelled());
        
        // Test cloning preserves state
        let cloned = token.clone();
        assert!(cloned.is_cancelled());
    }
    
    #[test]
    fn test_job_cancellation_manager() {
        let manager = JobCancellationManager::new();
        let job_id = crate::JobId::new_v4();
        
        // Register a job
        let token = manager.register_job(job_id);
        assert_eq!(manager.active_job_count(), 1);
        assert!(manager.active_jobs().contains(&job_id));
        
        // Token should not be cancelled initially
        assert!(!token.is_cancelled());
        
        // Cancel the specific job
        assert!(manager.cancel_job(job_id));
        assert!(token.is_cancelled());
        
        // Unregister the job
        manager.unregister_job(job_id);
        assert_eq!(manager.active_job_count(), 0);
        
        // Cancelling non-existent job should return false
        assert!(!manager.cancel_job(job_id));
    }
    
    #[test]
    fn test_job_metrics() {
        let mut metrics = JobMetrics::new();
        
        // Record scheduled jobs
        metrics.record_scheduled("test_job", Priority::High);
        metrics.record_scheduled("test_job", Priority::Normal);
        metrics.record_scheduled("email_job", Priority::High);
        
        assert_eq!(metrics.total_scheduled, 3);
        assert_eq!(*metrics.jobs_by_type.get("test_job").unwrap(), 2);
        assert_eq!(*metrics.jobs_by_type.get("email_job").unwrap(), 1);
        assert_eq!(*metrics.jobs_by_priority.get("High").unwrap(), 2);
        assert_eq!(*metrics.jobs_by_priority.get("Normal").unwrap(), 1);
        
        // Record executions
        metrics.record_execution_start();
        metrics.record_execution_start();
        assert_eq!(metrics.total_executed, 2);
        
        // Record success
        metrics.record_success(100); // 100ms execution time
        assert_eq!(metrics.successful_jobs, 1);
        assert_eq!(metrics.min_execution_time_ms, 100);
        assert_eq!(metrics.max_execution_time_ms, 100);
        assert_eq!(metrics.avg_execution_time_ms, 100.0); // Only one completed job
        
        // Record failure with retry attempts
        metrics.record_failure(200, 2); // 200ms, 2 retry attempts
        assert_eq!(metrics.failed_jobs, 1);
        assert_eq!(metrics.avg_retry_attempts, 2.0);
        assert_eq!(metrics.max_execution_time_ms, 200);
        assert_eq!(metrics.avg_execution_time_ms, 150.0); // (100 + 200) / 2
        
        // Check success rate
        assert_eq!(metrics.success_rate, 0.5); // 1 success out of 2 total
    }
    
    #[test]
    fn test_job_metrics_collector() {
        let collector = JobMetricsCollector::new();
        let job_id = crate::JobId::new_v4();
        
        // Record scheduled job
        collector.record_job_scheduled("test_job", Priority::High);
        
        // Record execution
        collector.record_execution_start(job_id);
        assert_eq!(collector.active_executions_count(), 1);
        
        // Record success
        std::thread::sleep(Duration::from_millis(10)); // Small delay for timing
        collector.record_job_success(job_id);
        assert_eq!(collector.active_executions_count(), 0);
        
        let metrics = collector.get_metrics();
        assert_eq!(metrics.total_scheduled, 1);
        assert_eq!(metrics.total_executed, 1);
        assert_eq!(metrics.successful_jobs, 1);
        assert_eq!(metrics.success_rate, 1.0);
        assert!(metrics.min_execution_time_ms >= 10);
    }
}