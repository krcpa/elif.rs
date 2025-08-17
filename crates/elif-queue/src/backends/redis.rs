//! Redis queue backend implementation for production use

use crate::{JobEntry, JobId, JobState, QueueBackend, QueueError, QueueResult, QueueStats, JobResult, RedisConfig};
use async_trait::async_trait;
use redis::{Client, Connection, AsyncCommands, RedisResult};
use serde_json;
use std::collections::HashMap;
use tokio::sync::RwLock;
use chrono::{Utc, DateTime};
use std::time::Duration;

/// Redis queue backend
pub struct RedisBackend {
    config: RedisConfig,
    client: Client,
    connection_pool: RwLock<Option<redis::aio::ConnectionManager>>,
}

impl RedisBackend {
    /// Create a new Redis backend
    pub async fn new(config: RedisConfig) -> QueueResult<Self> {
        let client = Client::open(config.url.as_str())
            .map_err(|e| QueueError::Configuration(format!("Invalid Redis URL: {}", e)))?;
        
        let backend = Self {
            config,
            client,
            connection_pool: RwLock::new(None),
        };
        
        // Initialize connection pool
        backend.ensure_connection().await?;
        
        Ok(backend)
    }
    
    /// Lua script for atomic enqueue operation
    const ENQUEUE_SCRIPT: &'static str = r#"
        local job_key = KEYS[1]
        local priority_key = KEYS[2]
        local delayed_key = KEYS[3]
        local state_key = KEYS[4]
        local job_data = ARGV[1]
        local score = tonumber(ARGV[2])
        local is_ready = ARGV[3] == '1'
        local job_id = ARGV[4]
        
        -- Store job data
        redis.call('SET', job_key, job_data)
        
        -- Add to appropriate queue
        if is_ready then
            redis.call('ZADD', priority_key, score, job_data)
        else
            redis.call('ZADD', delayed_key, score, job_data)
        end
        
        -- Add to state tracking
        redis.call('SADD', state_key, job_id)
        
        return 'OK'
    "#;
    
    /// Lua script for atomic dequeue operation
    const DEQUEUE_SCRIPT: &'static str = r#"
        local priority_key = KEYS[1]
        local job_key_prefix = KEYS[2]
        local pending_state_key = KEYS[3]
        local processing_state_key = KEYS[4]
        
        -- Get highest priority job
        local result = redis.call('ZPOPMAX', priority_key, 1)
        if #result == 0 then
            return nil
        end
        
        local job_data = result[1]
        local job_obj = cjson.decode(job_data)
        local job_id = job_obj.id
        
        -- Update job state to processing
        job_obj.state = 'Processing'
        job_obj.updated_at = ARGV[1]
        local updated_data = cjson.encode(job_obj)
        
        -- Store updated job
        local job_key = job_key_prefix .. job_id
        redis.call('SET', job_key, updated_data)
        
        -- Update state tracking
        redis.call('SREM', pending_state_key, job_id)
        redis.call('SADD', processing_state_key, job_id)
        
        return updated_data
    "#;
    
    /// Lua script for atomic complete operation
    const COMPLETE_SCRIPT: &'static str = r#"
        local job_key = KEYS[1]
        local priority_key = KEYS[2]
        local delayed_key = KEYS[3]
        local processing_state_key = KEYS[4]
        local completed_state_key = KEYS[5]
        local failed_state_key = KEYS[6]
        local dead_state_key = KEYS[7]
        
        local job_id = ARGV[1]
        local success = ARGV[2] == '1'
        local error_message = ARGV[3]
        local now = ARGV[4]
        local retry_score = tonumber(ARGV[5] or '0')
        local is_delayed_retry = ARGV[6] == '1'
        
        -- Get current job
        local job_data = redis.call('GET', job_key)
        if not job_data then
            return {err = 'Job not found'}
        end
        
        local job_obj = cjson.decode(job_data)
        local new_state_key
        
        if success then
            -- Mark as completed
            job_obj.state = 'Completed'
            job_obj.completed_at = now
            new_state_key = completed_state_key
        else
            -- Mark as failed and handle retry
            job_obj.attempts = (job_obj.attempts or 0) + 1
            job_obj.error_message = error_message
            job_obj.updated_at = now
            
            if job_obj.attempts < job_obj.max_retries then
                -- Retry the job
                job_obj.state = 'Failed'
                new_state_key = failed_state_key
                local updated_data = cjson.encode(job_obj)
                
                -- Add back to appropriate queue
                if is_delayed_retry then
                    redis.call('ZADD', delayed_key, retry_score, updated_data)
                else
                    redis.call('ZADD', priority_key, retry_score, updated_data)
                end
            else
                -- Job is dead
                job_obj.state = 'Dead'
                new_state_key = dead_state_key
            end
        end
        
        -- Update job in storage
        local final_data = cjson.encode(job_obj)
        redis.call('SET', job_key, final_data)
        
        -- Update state tracking
        redis.call('SREM', processing_state_key, job_id)
        redis.call('SADD', new_state_key, job_id)
        
        return 'OK'
    "#;
    
    /// Lua script for processing delayed jobs atomically
    const PROCESS_DELAYED_SCRIPT: &'static str = r#"
        local delayed_key = KEYS[1]
        local priority_key = KEYS[2]
        local now = tonumber(ARGV[1])
        
        -- Get ready jobs from delayed queue
        local ready_jobs = redis.call('ZRANGEBYSCORE', delayed_key, 0, now)
        
        for i, job_data in ipairs(ready_jobs) do
            -- Remove from delayed queue
            redis.call('ZREM', delayed_key, job_data)
            
            -- Add to priority queue with calculated score
            local job_obj = cjson.decode(job_data)
            local priority_score = 0
            
            -- Calculate priority score
            if job_obj.priority == 'Critical' then
                priority_score = 1000000
            elseif job_obj.priority == 'High' then
                priority_score = 100000
            elseif job_obj.priority == 'Normal' then
                priority_score = 10000
            else
                priority_score = 1000
            end
            
            -- Subtract timestamp to ensure earlier jobs come first within same priority
            local timestamp = job_obj.run_at and job_obj.run_at.timestamp_millis or 0
            local final_score = priority_score - (timestamp / 1000000)
            
            redis.call('ZADD', priority_key, final_score, job_data)
        end
        
        return #ready_jobs
    "#;
    
    /// Ensure we have a valid connection
    async fn ensure_connection(&self) -> QueueResult<()> {
        let mut pool = self.connection_pool.write().await;
        
        if pool.is_none() {
            let manager = self.client
                .get_tokio_connection_manager()
                .await
                .map_err(|e| QueueError::Network(format!("Failed to connect to Redis: {}", e)))?;
            
            *pool = Some(manager);
        }
        
        Ok(())
    }
    
    /// Get a connection from the pool
    async fn get_connection(&self) -> QueueResult<redis::aio::ConnectionManager> {
        self.ensure_connection().await?;
        let pool = self.connection_pool.read().await;
        Ok(pool.as_ref().unwrap().clone())
    }
    
    /// Get Redis key for a specific purpose
    fn get_key(&self, suffix: &str) -> String {
        format!("{}:{}", self.config.key_prefix, suffix)
    }
    
    /// Get Redis key for job storage
    fn get_job_key(&self, job_id: JobId) -> String {
        self.get_key(&format!("job:{}", job_id))
    }
    
    /// Get Redis key for state-based job lists
    fn get_state_key(&self, state: &JobState) -> String {
        match state {
            JobState::Pending => self.get_key("pending"),
            JobState::Processing => self.get_key("processing"),
            JobState::Completed => self.get_key("completed"),
            JobState::Failed => self.get_key("failed"),
            JobState::Dead => self.get_key("dead"),
        }
    }
    
    /// Get Redis key for priority queue
    fn get_priority_queue_key(&self) -> String {
        self.get_key("priority_queue")
    }
    
    /// Get Redis key for delayed jobs
    fn get_delayed_key(&self) -> String {
        self.get_key("delayed")
    }
    
    /// Serialize job entry to JSON
    fn serialize_job(&self, job: &JobEntry) -> QueueResult<String> {
        serde_json::to_string(job)
            .map_err(|e| QueueError::Serialization(e))
    }
    
    /// Deserialize job entry from JSON
    fn deserialize_job(&self, data: &str) -> QueueResult<JobEntry> {
        serde_json::from_str(data)
            .map_err(|e| QueueError::Serialization(e))
    }
    
    /// Calculate score for priority queue (higher priority and earlier run_at = lower score)
    fn calculate_score(&self, job: &JobEntry) -> f64 {
        let priority_weight = match job.priority() {
            crate::Priority::Critical => 1000000.0,
            crate::Priority::High => 100000.0,
            crate::Priority::Normal => 10000.0,
            crate::Priority::Low => 1000.0,
        };
        
        let timestamp = job.run_at().timestamp_millis() as f64;
        priority_weight - timestamp / 1000000.0 // Ensure priority dominates
    }
    
    /// Move expired delayed jobs to the priority queue
    async fn process_delayed_jobs(&self) -> QueueResult<()> {
        let mut conn = self.get_connection().await?;
        let delayed_key = self.get_delayed_key();
        let priority_key = self.get_priority_queue_key();
        let now = Utc::now().timestamp_millis() as f64;
        
        // Execute atomic process delayed jobs script
        let processed_count: i32 = conn
            .eval(
                Self::PROCESS_DELAYED_SCRIPT,
                &[delayed_key, priority_key],
                &[now.to_string()]
            )
            .await
            .map_err(|e| QueueError::Backend(format!("Failed to process delayed jobs atomically: {}", e)))?;
        
        Ok(())
    }
}

#[async_trait]
impl QueueBackend for RedisBackend {
    async fn enqueue(&self, job: JobEntry) -> QueueResult<JobId> {
        let mut conn = self.get_connection().await?;
        let job_id = job.id();
        let job_key = self.get_job_key(job_id);
        let priority_key = self.get_priority_queue_key();
        let delayed_key = self.get_delayed_key();
        let state_key = self.get_state_key(job.state());
        let serialized = self.serialize_job(&job)?;
        
        let score = if job.is_ready() {
            self.calculate_score(&job)
        } else {
            job.run_at().timestamp_millis() as f64
        };
        
        let is_ready = if job.is_ready() { "1" } else { "0" };
        
        // Execute atomic enqueue script
        let _: String = conn
            .eval(
                Self::ENQUEUE_SCRIPT,
                &[job_key, priority_key, delayed_key, state_key],
                &[serialized, score.to_string(), is_ready.to_string(), job_id.to_string()]
            )
            .await
            .map_err(|e| QueueError::Backend(format!("Failed to enqueue job atomically: {}", e)))?;
        
        Ok(job_id)
    }
    
    async fn dequeue(&self) -> QueueResult<Option<JobEntry>> {
        // Process any delayed jobs that are now ready
        self.process_delayed_jobs().await?;
        
        let mut conn = self.get_connection().await?;
        let priority_key = self.get_priority_queue_key();
        let job_key_prefix = self.get_key("");
        let pending_state_key = self.get_state_key(&JobState::Pending);
        let processing_state_key = self.get_state_key(&JobState::Processing);
        let now = Utc::now().to_rfc3339();
        
        // Execute atomic dequeue script
        let result: Option<String> = conn
            .eval(
                Self::DEQUEUE_SCRIPT,
                &[priority_key, job_key_prefix, pending_state_key, processing_state_key],
                &[now]
            )
            .await
            .map_err(|e| QueueError::Backend(format!("Failed to dequeue job atomically: {}", e)))?;
        
        match result {
            Some(job_data) => {
                let job = self.deserialize_job(&job_data)?;
                Ok(Some(job))
            }
            None => Ok(None)
        }
    }
    
    async fn complete(&self, job_id: JobId, result: JobResult<()>) -> QueueResult<()> {
        let mut conn = self.get_connection().await?;
        let job_key = self.get_job_key(job_id);
        let priority_key = self.get_priority_queue_key();
        let delayed_key = self.get_delayed_key();
        let processing_state_key = self.get_state_key(&JobState::Processing);
        let completed_state_key = self.get_state_key(&JobState::Completed);
        let failed_state_key = self.get_state_key(&JobState::Failed);
        let dead_state_key = self.get_state_key(&JobState::Dead);
        
        let success = result.is_ok();
        let error_message = match &result {
            Ok(_) => String::new(),
            Err(e) => e.to_string(),
        };
        let now = Utc::now().to_rfc3339();
        
        // For failed jobs, we need to calculate retry parameters
        let (retry_score, is_delayed_retry) = if result.is_err() {
            // We need to get the job first to calculate retry parameters
            let job_data: Option<String> = conn
                .get(&job_key)
                .await
                .map_err(|e| QueueError::Backend(format!("Failed to get job for retry calculation: {}", e)))?;
            
            if let Some(data) = job_data {
                let mut job = self.deserialize_job(&data)?;
                job.mark_failed(error_message.clone()); // This increments attempts and sets retry time
                
                if job.attempts() < job.max_retries() {
                    let score = if job.run_at() <= Utc::now() {
                        (self.calculate_score(&job), false)
                    } else {
                        (job.run_at().timestamp_millis() as f64, true)
                    };
                    score
                } else {
                    (0.0, false) // Job will be marked as dead
                }
            } else {
                (0.0, false)
            }
        } else {
            (0.0, false)
        };
        
        // Execute atomic complete script
        let _: String = conn
            .eval(
                Self::COMPLETE_SCRIPT,
                &[job_key, priority_key, delayed_key, processing_state_key, completed_state_key, failed_state_key, dead_state_key],
                &[
                    job_id.to_string(),
                    if success { "1" } else { "0" }.to_string(),
                    error_message,
                    now,
                    retry_score.to_string(),
                    if is_delayed_retry { "1" } else { "0" }.to_string()
                ]
            )
            .await
            .map_err(|e| QueueError::Backend(format!("Failed to complete job atomically: {}", e)))?;
        
        Ok(())
    }
    
    async fn get_job(&self, job_id: JobId) -> QueueResult<Option<JobEntry>> {
        let mut conn = self.get_connection().await?;
        let job_key = self.get_job_key(job_id);
        
        let job_data: Option<String> = conn
            .get(&job_key)
            .await
            .map_err(|e| QueueError::Backend(format!("Failed to get job: {}", e)))?;
        
        match job_data {
            Some(data) => Ok(Some(self.deserialize_job(&data)?)),
            None => Ok(None),
        }
    }
    
    async fn get_jobs_by_state(&self, state: JobState, limit: Option<usize>) -> QueueResult<Vec<JobEntry>> {
        let mut conn = self.get_connection().await?;
        let state_key = self.get_state_key(&state);
        
        // Get job IDs from state set
        let job_ids: Vec<String> = if let Some(limit) = limit {
            conn.srandmember_multiple(&state_key, limit as isize)
                .await
                .map_err(|e| QueueError::Backend(format!("Failed to get job IDs: {}", e)))?
        } else {
            conn.smembers(&state_key)
                .await
                .map_err(|e| QueueError::Backend(format!("Failed to get job IDs: {}", e)))?
        };
        
        let mut jobs = Vec::with_capacity(job_ids.len());
        
        // Get job data for each ID
        for job_id_str in job_ids {
            if let Ok(job_id) = job_id_str.parse::<JobId>() {
                if let Ok(Some(job)) = self.get_job(job_id).await {
                    jobs.push(job);
                }
            }
        }
        
        // Sort by created_at for consistent ordering
        jobs.sort_by(|a, b| a.created_at.cmp(&b.created_at));
        
        Ok(jobs)
    }
    
    async fn remove_job(&self, job_id: JobId) -> QueueResult<bool> {
        let mut conn = self.get_connection().await?;
        
        // Get job to determine its state
        let job = match self.get_job(job_id).await? {
            Some(job) => job,
            None => return Ok(false),
        };
        
        let job_key = self.get_job_key(job_id);
        let state_key = self.get_state_key(job.state());
        let serialized = self.serialize_job(&job)?;
        
        // Remove from storage
        let removed: u32 = conn
            .del(&job_key)
            .await
            .map_err(|e| QueueError::Backend(format!("Failed to delete job: {}", e)))?;
        
        // Remove from state list
        let _: () = conn
            .srem(&state_key, job_id.to_string())
            .await
            .map_err(|e| QueueError::Backend(format!("Failed to remove from state list: {}", e)))?;
        
        // Remove from queues
        let priority_key = self.get_priority_queue_key();
        let delayed_key = self.get_delayed_key();
        
        let _: () = conn
            .zrem(&priority_key, &serialized)
            .await
            .map_err(|e| QueueError::Backend(format!("Failed to remove from priority queue: {}", e)))?;
        
        let _: () = conn
            .zrem(&delayed_key, &serialized)
            .await
            .map_err(|e| QueueError::Backend(format!("Failed to remove from delayed queue: {}", e)))?;
        
        Ok(removed > 0)
    }
    
    async fn clear(&self) -> QueueResult<()> {
        let mut conn = self.get_connection().await?;
        
        // Get all keys with our prefix
        let pattern = format!("{}:*", self.config.key_prefix);
        let keys: Vec<String> = conn
            .keys(&pattern)
            .await
            .map_err(|e| QueueError::Backend(format!("Failed to get keys: {}", e)))?;
        
        if !keys.is_empty() {
            let _: () = conn
                .del(keys)
                .await
                .map_err(|e| QueueError::Backend(format!("Failed to clear queue: {}", e)))?;
        }
        
        Ok(())
    }
    
    async fn stats(&self) -> QueueResult<QueueStats> {
        let mut conn = self.get_connection().await?;
        
        let mut stats = QueueStats::default();
        
        // Count jobs in each state
        for state in [JobState::Pending, JobState::Processing, JobState::Completed, JobState::Failed, JobState::Dead] {
            let state_key = self.get_state_key(&state);
            let count: u64 = conn
                .scard(&state_key)
                .await
                .map_err(|e| QueueError::Backend(format!("Failed to count jobs in state {:?}: {}", state, e)))?;
            
            match state {
                JobState::Pending => stats.pending_jobs = count,
                JobState::Processing => stats.processing_jobs = count,
                JobState::Completed => stats.completed_jobs = count,
                JobState::Failed => stats.failed_jobs = count,
                JobState::Dead => stats.dead_jobs = count,
            }
        }
        
        stats.total_jobs = stats.pending_jobs + stats.processing_jobs + 
                          stats.completed_jobs + stats.failed_jobs + stats.dead_jobs;
        
        Ok(stats)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Job, Priority};
    use serde::{Deserialize, Serialize};
    use std::time::Duration;
    
    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct TestJob {
        id: u32,
        message: String,
    }
    
    #[async_trait]
    impl Job for TestJob {
        async fn execute(&self) -> JobResult<()> {
            Ok(())
        }
        
        fn job_type(&self) -> &'static str {
            "test"
        }
    }
    
    async fn create_test_backend() -> RedisBackend {
        let config = RedisConfig {
            url: "redis://localhost:6379".to_string(),
            key_prefix: "test_elif_queue".to_string(),
            ..Default::default()
        };
        
        match RedisBackend::new(config).await {
            Ok(backend) => {
                // Clear any existing test data
                let _ = backend.clear().await;
                backend
            }
            Err(_) => {
                // Skip tests if Redis is not available
                panic!("Redis server not available for testing");
            }
        }
    }
    
    #[tokio::test]
    #[ignore] // Requires Redis server
    async fn test_redis_backend_basic_operations() {
        let backend = create_test_backend().await;
        
        let job = TestJob {
            id: 1,
            message: "test job".to_string(),
        };
        let entry = JobEntry::new(job, Some(Priority::Normal), None).unwrap();
        let job_id = entry.id();
        
        // Enqueue job
        backend.enqueue(entry).await.unwrap();
        
        // Check stats
        let stats = backend.stats().await.unwrap();
        assert_eq!(stats.pending_jobs, 1);
        assert_eq!(stats.total_jobs, 1);
        
        // Dequeue job
        let dequeued = backend.dequeue().await.unwrap().unwrap();
        assert_eq!(dequeued.id(), job_id);
        assert_eq!(dequeued.state(), &JobState::Processing);
        
        // Complete job
        backend.complete(job_id, Ok(())).await.unwrap();
        
        // Check final stats
        let stats = backend.stats().await.unwrap();
        assert_eq!(stats.completed_jobs, 1);
        assert_eq!(stats.processing_jobs, 0);
        assert_eq!(stats.pending_jobs, 0);
        
        // Clean up
        backend.clear().await.unwrap();
    }
    
    #[tokio::test]
    #[ignore] // Requires Redis server
    async fn test_redis_atomicity_guarantees() {
        let backend = create_test_backend().await;
        
        // Test 1: Enqueue is atomic - job should exist in storage AND queue
        let job = TestJob { id: 100, message: "atomicity test".to_string() };
        let entry = JobEntry::new(job, Some(Priority::High), None).unwrap();
        let job_id = backend.enqueue(entry).await.unwrap();
        
        // Verify job was stored and queued atomically
        let stored_job = backend.get_job(job_id).await.unwrap().unwrap();
        assert_eq!(stored_job.priority(), Priority::High);
        assert_eq!(stored_job.state(), &JobState::Pending);
        
        // Test 2: Dequeue is atomic - job should be removed from queue AND updated in storage
        let dequeued = backend.dequeue().await.unwrap().unwrap();
        assert_eq!(dequeued.id(), job_id);
        assert_eq!(dequeued.state(), &JobState::Processing);
        
        // Verify the stored job was also updated
        let stored_job = backend.get_job(job_id).await.unwrap().unwrap();
        assert_eq!(stored_job.state(), &JobState::Processing);
        
        // Test 3: Complete is atomic - job state and queue state are updated together
        backend.complete(job_id, Ok(())).await.unwrap();
        
        let completed_job = backend.get_job(job_id).await.unwrap().unwrap();
        assert_eq!(completed_job.state(), &JobState::Completed);
        
        // Test 4: Failed job retry is atomic
        let job2 = TestJob { id: 101, message: "retry test".to_string() };
        let entry2 = JobEntry::new(job2, Some(Priority::Normal), None).unwrap();
        let job_id2 = backend.enqueue(entry2).await.unwrap();
        
        // Dequeue and fail the job
        let _dequeued = backend.dequeue().await.unwrap().unwrap();
        let error = Box::new(std::io::Error::new(std::io::ErrorKind::Other, "test failure"));
        backend.complete(job_id2, Err(error)).await.unwrap();
        
        // Job should be marked as failed and available for retry
        let failed_job = backend.get_job(job_id2).await.unwrap().unwrap();
        assert_eq!(failed_job.state(), &JobState::Failed);
        assert_eq!(failed_job.attempts(), 1);
        
        // Should be available for retry
        let retry_job = backend.dequeue().await.unwrap().unwrap();
        assert_eq!(retry_job.id(), job_id2);
        assert_eq!(retry_job.attempts(), 1);
        
        // Clean up
        backend.clear().await.unwrap();
    }
    
    #[tokio::test]
    #[ignore] // Requires Redis server
    async fn test_redis_delayed_job() {
        let backend = create_test_backend().await;
        
        let job = TestJob {
            id: 1,
            message: "delayed job".to_string(),
        };
        let delay = Duration::from_millis(100);
        let entry = JobEntry::new(job, None, Some(delay)).unwrap();
        
        backend.enqueue(entry).await.unwrap();
        
        // Should not be available immediately
        let result = backend.dequeue().await.unwrap();
        assert!(result.is_none());
        
        // Wait for delay
        tokio::time::sleep(delay + Duration::from_millis(50)).await;
        
        // Should be available now
        let result = backend.dequeue().await.unwrap();
        assert!(result.is_some());
        
        // Clean up
        backend.clear().await.unwrap();
    }
}