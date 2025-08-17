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
        
        // Get all jobs ready to be processed
        let ready_jobs: Vec<(String, f64)> = conn
            .zrangebyscore_withscores(&delayed_key, 0.0, now)
            .await
            .map_err(|e| QueueError::Backend(format!("Failed to get delayed jobs: {}", e)))?;
        
        if ready_jobs.is_empty() {
            return Ok(());
        }
        
        // Move jobs to priority queue and update their state
        for (job_data, _score) in ready_jobs {
            let mut job = self.deserialize_job(&job_data)?;
            
            if job.state() == &JobState::Pending || job.state() == &JobState::Failed {
                let new_score = self.calculate_score(&job);
                
                // Add to priority queue
                let _: () = conn
                    .zadd(&priority_key, job_data.clone(), new_score)
                    .await
                    .map_err(|e| QueueError::Backend(format!("Failed to add job to priority queue: {}", e)))?;
                
                // Update job in storage (if needed, job state might already be correct)
                let job_key = self.get_job_key(job.id());
                let serialized = self.serialize_job(&job)?;
                let _: () = conn
                    .set(&job_key, serialized)
                    .await
                    .map_err(|e| QueueError::Backend(format!("Failed to update job: {}", e)))?;
            }
        }
        
        // Remove processed jobs from delayed queue
        let job_scores: Vec<f64> = ready_jobs.iter().map(|(_, score)| *score).collect();
        if !job_scores.is_empty() {
            let _: () = conn
                .zremrangebyscore(&delayed_key, 0.0, now)
                .await
                .map_err(|e| QueueError::Backend(format!("Failed to remove delayed jobs: {}", e)))?;
        }
        
        Ok(())
    }
}

#[async_trait]
impl QueueBackend for RedisBackend {
    async fn enqueue(&self, job: JobEntry) -> QueueResult<JobId> {
        let mut conn = self.get_connection().await?;
        let job_id = job.id();
        let job_key = self.get_job_key(job_id);
        let serialized = self.serialize_job(&job)?;
        
        // Store job data
        let _: () = conn
            .set(&job_key, &serialized)
            .await
            .map_err(|e| QueueError::Backend(format!("Failed to store job: {}", e)))?;
        
        // Add to appropriate queue
        if job.is_ready() {
            // Add to priority queue
            let priority_key = self.get_priority_queue_key();
            let score = self.calculate_score(&job);
            let _: () = conn
                .zadd(&priority_key, &serialized, score)
                .await
                .map_err(|e| QueueError::Backend(format!("Failed to add job to priority queue: {}", e)))?;
        } else {
            // Add to delayed queue
            let delayed_key = self.get_delayed_key();
            let score = job.run_at().timestamp_millis() as f64;
            let _: () = conn
                .zadd(&delayed_key, &serialized, score)
                .await
                .map_err(|e| QueueError::Backend(format!("Failed to add job to delayed queue: {}", e)))?;
        }
        
        // Add to state list
        let state_key = self.get_state_key(job.state());
        let _: () = conn
            .sadd(&state_key, job_id.to_string())
            .await
            .map_err(|e| QueueError::Backend(format!("Failed to add job to state list: {}", e)))?;
        
        Ok(job_id)
    }
    
    async fn dequeue(&self) -> QueueResult<Option<JobEntry>> {
        // Process any delayed jobs that are now ready
        self.process_delayed_jobs().await?;
        
        let mut conn = self.get_connection().await?;
        let priority_key = self.get_priority_queue_key();
        
        // Get highest priority job
        let result: Vec<String> = conn
            .zpopmax(&priority_key, 1)
            .await
            .map_err(|e| QueueError::Backend(format!("Failed to dequeue job: {}", e)))?;
        
        if result.is_empty() {
            return Ok(None);
        }
        
        let job_data = &result[0];
        let mut job = self.deserialize_job(job_data)?;
        let job_id = job.id();
        
        // Update job state
        let old_state = job.state().clone();
        job.mark_processing();
        
        // Update job in storage
        let job_key = self.get_job_key(job_id);
        let serialized = self.serialize_job(&job)?;
        let _: () = conn
            .set(&job_key, serialized)
            .await
            .map_err(|e| QueueError::Backend(format!("Failed to update job state: {}", e)))?;
        
        // Update state lists
        let old_state_key = self.get_state_key(&old_state);
        let new_state_key = self.get_state_key(job.state());
        
        let _: () = conn
            .srem(&old_state_key, job_id.to_string())
            .await
            .map_err(|e| QueueError::Backend(format!("Failed to remove from old state list: {}", e)))?;
        
        let _: () = conn
            .sadd(&new_state_key, job_id.to_string())
            .await
            .map_err(|e| QueueError::Backend(format!("Failed to add to new state list: {}", e)))?;
        
        Ok(Some(job))
    }
    
    async fn complete(&self, job_id: JobId, result: JobResult<()>) -> QueueResult<()> {
        let mut conn = self.get_connection().await?;
        let job_key = self.get_job_key(job_id);
        
        // Get current job
        let job_data: Option<String> = conn
            .get(&job_key)
            .await
            .map_err(|e| QueueError::Backend(format!("Failed to get job: {}", e)))?;
        
        let job_data = job_data.ok_or_else(|| QueueError::JobNotFound(job_id.to_string()))?;
        let mut job = self.deserialize_job(&job_data)?;
        let old_state = job.state().clone();
        
        match result {
            Ok(_) => {
                job.mark_completed();
            }
            Err(error) => {
                let error_message = error.to_string();
                job.mark_failed(error_message);
                
                // If job should be retried, add it back to appropriate queue
                if job.state() == &JobState::Failed {
                    if job.run_at() <= Utc::now() {
                        // Add to priority queue
                        let priority_key = self.get_priority_queue_key();
                        let score = self.calculate_score(&job);
                        let serialized = self.serialize_job(&job)?;
                        let _: () = conn
                            .zadd(&priority_key, &serialized, score)
                            .await
                            .map_err(|e| QueueError::Backend(format!("Failed to re-queue job: {}", e)))?;
                    } else {
                        // Add to delayed queue
                        let delayed_key = self.get_delayed_key();
                        let score = job.run_at().timestamp_millis() as f64;
                        let serialized = self.serialize_job(&job)?;
                        let _: () = conn
                            .zadd(&delayed_key, &serialized, score)
                            .await
                            .map_err(|e| QueueError::Backend(format!("Failed to delay retry job: {}", e)))?;
                    }
                }
            }
        }
        
        // Update job in storage
        let serialized = self.serialize_job(&job)?;
        let _: () = conn
            .set(&job_key, serialized)
            .await
            .map_err(|e| QueueError::Backend(format!("Failed to update completed job: {}", e)))?;
        
        // Update state lists
        let old_state_key = self.get_state_key(&old_state);
        let new_state_key = self.get_state_key(job.state());
        
        let _: () = conn
            .srem(&old_state_key, job_id.to_string())
            .await
            .map_err(|e| QueueError::Backend(format!("Failed to remove from old state list: {}", e)))?;
        
        let _: () = conn
            .sadd(&new_state_key, job_id.to_string())
            .await
            .map_err(|e| QueueError::Backend(format!("Failed to add to new state list: {}", e)))?;
        
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
    
    /// Atomic requeue implementation for Redis using Lua script
    async fn requeue_job(&self, job_id: JobId, mut job: JobEntry) -> QueueResult<bool> {
        let mut conn = self.get_connection().await?;
        
        let job_key = format!("{}:job:{}", self.config.key_prefix, job_id);
        let dead_state_key = format!("{}:state:Dead", self.config.key_prefix);
        let pending_state_key = format!("{}:state:Pending", self.config.key_prefix);
        let priority_key = format!("{}:priority:{:?}", self.config.key_prefix, job.priority());
        
        // Lua script for atomic requeue operation
        let script = r#"
            local job_key = KEYS[1]
            local dead_state_key = KEYS[2]
            local pending_state_key = KEYS[3]
            local priority_key = KEYS[4]
            local job_id = ARGV[1]
            local updated_job = ARGV[2]
            
            -- Check if job exists and is in dead state
            local exists = redis.call('EXISTS', job_key)
            if exists == 0 then
                return 0  -- Job not found
            end
            
            local in_dead_state = redis.call('SISMEMBER', dead_state_key, job_id)
            if in_dead_state == 0 then
                return -1  -- Job exists but not in dead state
            end
            
            -- Atomically move job from dead to pending state
            redis.call('SREM', dead_state_key, job_id)
            redis.call('SADD', pending_state_key, job_id)
            
            -- Update job data
            redis.call('SET', job_key, updated_job)
            
            -- Add to priority queue for processing
            local now = redis.call('TIME')
            local timestamp = now[1] + now[2] / 1000000
            redis.call('ZADD', priority_key, timestamp, job_id)
            
            return 1  -- Success
        "#;
        
        // Reset job for retry
        job.reset_for_retry();
        let serialized_job = self.serialize_job(&job)?;
        
        let result: i32 = redis::Script::new(script)
            .key(&job_key)
            .key(&dead_state_key)
            .key(&pending_state_key)
            .key(&priority_key)
            .arg(job_id.to_string())
            .arg(&serialized_job)
            .invoke_async(&mut conn)
            .await
            .map_err(|e| QueueError::Backend(format!("Failed to requeue job atomically: {}", e)))?;
            
        match result {
            1 => Ok(true),   // Success
            0 => Ok(false),  // Job not found
            -1 => Ok(false), // Job not in dead state
            _ => Err(QueueError::Backend("Unexpected result from requeue script".to_string())),
        }
    }
    
    /// Atomic clear jobs by state implementation for Redis using Lua script
    async fn clear_jobs_by_state(&self, state: JobState) -> QueueResult<u64> {
        let mut conn = self.get_connection().await?;
        
        let state_key = format!("{}:state:{:?}", self.config.key_prefix, state);
        let job_prefix = format!("{}:job:", self.config.key_prefix);
        
        // Lua script for atomic clear by state operation
        let script = r#"
            local state_key = KEYS[1]
            local job_prefix = ARGV[1]
            local priority_prefix = ARGV[2]
            local delayed_prefix = ARGV[3]
            
            -- Get all job IDs in this state
            local job_ids = redis.call('SMEMBERS', state_key)
            local count = #job_ids
            
            if count == 0 then
                return 0
            end
            
            -- Remove all jobs from the state set
            redis.call('DEL', state_key)
            
            -- Remove job data and from priority/delayed queues
            for i = 1, count do
                local job_id = job_ids[i]
                local job_key = job_prefix .. job_id
                
                -- Delete job data
                redis.call('DEL', job_key)
                
                -- Remove from all possible priority queues
                for priority = 0, 3 do
                    local priority_key = priority_prefix .. priority
                    redis.call('ZREM', priority_key, job_id)
                end
                
                -- Remove from delayed queue
                redis.call('ZREM', delayed_prefix, job_id)
            end
            
            return count
        "#;
        
        let priority_prefix = format!("{}:priority:", self.config.key_prefix);
        let delayed_key = format!("{}:delayed", self.config.key_prefix);
        
        let count: u64 = redis::Script::new(script)
            .key(&state_key)
            .arg(&job_prefix)
            .arg(&priority_prefix)
            .arg(&delayed_key)
            .invoke_async(&mut conn)
            .await
            .map_err(|e| QueueError::Backend(format!("Failed to clear jobs atomically: {}", e)))?;
            
        Ok(count)
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