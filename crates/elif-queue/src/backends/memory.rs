//! In-memory queue backend implementation for development and testing

use crate::{
    JobEntry, JobId, JobResult, JobState, QueueBackend, QueueConfig, QueueError, QueueResult,
    QueueStats,
};
use async_trait::async_trait;
use chrono::Utc;
use dashmap::DashMap;
use parking_lot::RwLock;
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::sync::Arc;
use tokio::time::Instant;

/// Wrapper for JobEntry to implement Ord for priority queue
#[derive(Debug, Clone)]
struct PriorityJobEntry {
    entry: JobEntry,
    enqueue_time: Instant,
}

impl PartialEq for PriorityJobEntry {
    fn eq(&self, other: &Self) -> bool {
        self.entry.priority() == other.entry.priority()
            && self.entry.run_at() == other.entry.run_at()
            && self.enqueue_time == other.enqueue_time
    }
}

impl Eq for PriorityJobEntry {}

impl PartialOrd for PriorityJobEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PriorityJobEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        // Higher priority first, then earlier run_at, then earlier enqueue time
        match self.entry.priority().cmp(&other.entry.priority()) {
            Ordering::Equal => match other.entry.run_at().cmp(&self.entry.run_at()) {
                Ordering::Equal => other.enqueue_time.cmp(&self.enqueue_time),
                other_ord => other_ord,
            },
            priority_ord => priority_ord,
        }
    }
}

/// In-memory queue backend
pub struct MemoryBackend {
    config: QueueConfig,
    jobs: DashMap<JobId, JobEntry>,
    pending_queue: Arc<RwLock<BinaryHeap<PriorityJobEntry>>>,
    stats: Arc<RwLock<QueueStats>>,
}

impl MemoryBackend {
    /// Create a new memory backend
    pub fn new(config: QueueConfig) -> Self {
        Self {
            config,
            jobs: DashMap::new(),
            pending_queue: Arc::new(RwLock::new(BinaryHeap::new())),
            stats: Arc::new(RwLock::new(QueueStats::default())),
        }
    }

    /// Update statistics based on job state changes
    fn update_stats(&self, old_state: Option<JobState>, new_state: JobState) {
        let mut stats = self.stats.write();

        // Decrement old state count
        if let Some(old) = old_state {
            match old {
                JobState::Pending => stats.pending_jobs = stats.pending_jobs.saturating_sub(1),
                JobState::Processing => {
                    stats.processing_jobs = stats.processing_jobs.saturating_sub(1)
                }
                JobState::Completed => {
                    stats.completed_jobs = stats.completed_jobs.saturating_sub(1)
                }
                JobState::Failed => stats.failed_jobs = stats.failed_jobs.saturating_sub(1),
                JobState::Dead => stats.dead_jobs = stats.dead_jobs.saturating_sub(1),
            }
        } else {
            // New job
            stats.total_jobs += 1;
        }

        // Increment new state count
        match new_state {
            JobState::Pending => stats.pending_jobs += 1,
            JobState::Processing => stats.processing_jobs += 1,
            JobState::Completed => stats.completed_jobs += 1,
            JobState::Failed => stats.failed_jobs += 1,
            JobState::Dead => stats.dead_jobs += 1,
        }
    }

    /// Get the next ready job from the queue
    fn get_next_ready_job(&self) -> Option<JobEntry> {
        let mut queue = self.pending_queue.write();
        let now = Utc::now();

        // Look for a ready job at the top of the heap
        while let Some(priority_entry) = queue.peek() {
            // If the job has been removed from the main map, discard it from the pending queue.
            // This handles "ghost" jobs that might remain in the BinaryHeap after being removed
            // via `remove_job`.
            if !self.jobs.contains_key(&priority_entry.entry.id()) {
                queue.pop();
                continue;
            }

            if priority_entry.entry.is_ready() {
                let priority_entry = queue.pop().unwrap();
                return Some(priority_entry.entry);
            } else if priority_entry.entry.run_at() > now {
                // No ready jobs (heap is ordered by run_at)
                break;
            } else {
                // Job exists but may not be ready due to other conditions
                queue.pop();
            }
        }

        None
    }
}

#[async_trait]
impl QueueBackend for MemoryBackend {
    async fn enqueue(&self, job: JobEntry) -> QueueResult<JobId> {
        let job_id = job.id();

        // Check queue size limit
        if *self.config.get_max_queue_size() > 0
            && self.jobs.len() >= *self.config.get_max_queue_size()
        {
            return Err(QueueError::Configuration(format!(
                "Queue size limit exceeded: {}",
                *self.config.get_max_queue_size()
            )));
        }

        // Update stats
        self.update_stats(None, job.state().clone());

        // Add to pending queue if ready or pending
        if job.state() == &JobState::Pending {
            let priority_entry = PriorityJobEntry {
                entry: job.clone(),
                enqueue_time: Instant::now(),
            };
            self.pending_queue.write().push(priority_entry);
        }

        // Store the job
        self.jobs.insert(job_id, job);

        Ok(job_id)
    }

    async fn dequeue(&self) -> QueueResult<Option<JobEntry>> {
        if let Some(mut job) = self.get_next_ready_job() {
            let old_state = job.state().clone();
            job.mark_processing();

            // Update stats
            self.update_stats(Some(old_state), job.state().clone());

            // Update stored job
            self.jobs.insert(job.id(), job.clone());

            Ok(Some(job))
        } else {
            Ok(None)
        }
    }

    async fn complete(&self, job_id: JobId, result: JobResult<()>) -> QueueResult<()> {
        if let Some(mut job_entry) = self.jobs.get_mut(&job_id) {
            let old_state = job_entry.state().clone();

            match result {
                Ok(_) => {
                    job_entry.mark_completed();
                    self.update_stats(Some(old_state), job_entry.state().clone());
                }
                Err(error) => {
                    let error_message = error.to_string();
                    job_entry.mark_failed(error_message);
                    let new_state = job_entry.state().clone();

                    self.update_stats(Some(old_state), new_state.clone());

                    // Re-queue for retry if not dead
                    if new_state == JobState::Failed {
                        let priority_entry = PriorityJobEntry {
                            entry: job_entry.clone(),
                            enqueue_time: Instant::now(),
                        };
                        self.pending_queue.write().push(priority_entry);
                    }
                }
            }
            Ok(())
        } else {
            Err(QueueError::JobNotFound(job_id.to_string()))
        }
    }

    async fn get_job(&self, job_id: JobId) -> QueueResult<Option<JobEntry>> {
        Ok(self.jobs.get(&job_id).map(|entry| entry.clone()))
    }

    async fn get_jobs_by_state(
        &self,
        state: JobState,
        limit: Option<usize>,
    ) -> QueueResult<Vec<JobEntry>> {
        let mut jobs: Vec<JobEntry> = self
            .jobs
            .iter()
            .filter(|entry| entry.state() == &state)
            .map(|entry| entry.clone())
            .collect();

        // Sort by created_at for consistent ordering
        jobs.sort_by(|a, b| a.created_at.cmp(&b.created_at));

        if let Some(limit) = limit {
            jobs.truncate(limit);
        }

        Ok(jobs)
    }

    async fn remove_job(&self, job_id: JobId) -> QueueResult<bool> {
        if let Some((_, job)) = self.jobs.remove(&job_id) {
            // Update stats
            let mut stats = self.stats.write();
            match job.state() {
                JobState::Pending => stats.pending_jobs = stats.pending_jobs.saturating_sub(1),
                JobState::Processing => {
                    stats.processing_jobs = stats.processing_jobs.saturating_sub(1)
                }
                JobState::Completed => {
                    stats.completed_jobs = stats.completed_jobs.saturating_sub(1)
                }
                JobState::Failed => stats.failed_jobs = stats.failed_jobs.saturating_sub(1),
                JobState::Dead => stats.dead_jobs = stats.dead_jobs.saturating_sub(1),
            }
            stats.total_jobs = stats.total_jobs.saturating_sub(1);

            Ok(true)
        } else {
            Ok(false)
        }
    }

    async fn clear(&self) -> QueueResult<()> {
        self.jobs.clear();
        self.pending_queue.write().clear();
        *self.stats.write() = QueueStats::default();
        Ok(())
    }

    async fn stats(&self) -> QueueResult<QueueStats> {
        Ok(self.stats.read().clone())
    }

    /// Atomic requeue implementation for memory backend
    async fn requeue_job(&self, job_id: JobId, _job: JobEntry) -> QueueResult<bool> {
        // For memory backend, we can make this atomic using the DashMap's atomic operations
        if let Some(mut existing_job) = self.jobs.get_mut(&job_id) {
            if existing_job.state() == &JobState::Dead {
                // Reset the job for retry
                existing_job.reset_for_retry();

                // Add back to pending queue if it's ready
                if existing_job.is_ready() {
                    let priority_entry = PriorityJobEntry {
                        entry: existing_job.clone(),
                        enqueue_time: Instant::now(),
                    };
                    self.pending_queue.write().push(priority_entry);
                }

                // Update stats - move from dead back to pending
                let mut stats = self.stats.write();
                stats.dead_jobs = stats.dead_jobs.saturating_sub(1);
                stats.pending_jobs = stats.pending_jobs.saturating_add(1);

                Ok(true)
            } else {
                Ok(false)
            }
        } else {
            Ok(false)
        }
    }

    /// Atomic clear jobs by state implementation for memory backend
    async fn clear_jobs_by_state(&self, state: JobState) -> QueueResult<u64> {
        let mut count = 0u64;
        let mut stats = self.stats.write();

        // Use retain to atomically filter and count removed jobs
        self.jobs.retain(|_, job| {
            if job.state() == &state {
                count += 1;

                // Update stats for removed job
                match state {
                    JobState::Pending => stats.pending_jobs = stats.pending_jobs.saturating_sub(1),
                    JobState::Processing => {
                        stats.processing_jobs = stats.processing_jobs.saturating_sub(1)
                    }
                    JobState::Completed => {
                        stats.completed_jobs = stats.completed_jobs.saturating_sub(1)
                    }
                    JobState::Failed => stats.failed_jobs = stats.failed_jobs.saturating_sub(1),
                    JobState::Dead => stats.dead_jobs = stats.dead_jobs.saturating_sub(1),
                }

                false // Remove this job
            } else {
                true // Keep this job
            }
        });

        // Also remove any matching jobs from the pending queue if we're clearing pending jobs
        if state == JobState::Pending {
            let mut pending_queue = self.pending_queue.write();
            pending_queue
                .retain(|priority_entry| priority_entry.entry.state() != &JobState::Pending);
        }

        stats.total_jobs = stats.total_jobs.saturating_sub(count);
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

    #[tokio::test]
    async fn test_memory_backend_basic_operations() {
        let backend = MemoryBackend::new(QueueConfig::default());

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
    }

    #[tokio::test]
    async fn test_priority_ordering() {
        let backend = MemoryBackend::new(QueueConfig::default());

        // Enqueue jobs with different priorities
        let low_job = TestJob {
            id: 1,
            message: "low".to_string(),
        };
        let high_job = TestJob {
            id: 2,
            message: "high".to_string(),
        };
        let normal_job = TestJob {
            id: 3,
            message: "normal".to_string(),
        };

        let low_entry = JobEntry::new(low_job, Some(Priority::Low), None).unwrap();
        let high_entry = JobEntry::new(high_job, Some(Priority::High), None).unwrap();
        let normal_entry = JobEntry::new(normal_job, Some(Priority::Normal), None).unwrap();

        backend.enqueue(low_entry).await.unwrap();
        backend.enqueue(high_entry).await.unwrap();
        backend.enqueue(normal_entry).await.unwrap();

        // Dequeue should return high priority first
        let first = backend.dequeue().await.unwrap().unwrap();
        assert_eq!(first.priority(), Priority::High);

        let second = backend.dequeue().await.unwrap().unwrap();
        assert_eq!(second.priority(), Priority::Normal);

        let third = backend.dequeue().await.unwrap().unwrap();
        assert_eq!(third.priority(), Priority::Low);
    }

    #[tokio::test]
    async fn test_ghost_job_cleanup() {
        let config = crate::config::QueueConfigBuilder::testing()
            .build()
            .expect("Failed to build config");
        let backend = MemoryBackend::new(config);

        // Create and enqueue a job
        let job = TestJob {
            id: 1,
            message: "ghost test".to_string(),
        };
        let entry = JobEntry::new(job, Some(Priority::Normal), None).unwrap();
        let job_id = backend.enqueue(entry).await.unwrap();

        // Verify job is in both jobs map and pending queue
        assert!(backend.jobs.contains_key(&job_id));
        assert_eq!(backend.pending_queue.read().len(), 1);

        // Remove job directly from jobs map (simulating a manual removal)
        backend.jobs.remove(&job_id);

        // The pending queue still contains the ghost entry
        assert_eq!(backend.pending_queue.read().len(), 1);

        // But dequeue should clean up the ghost and return None
        let result = backend.dequeue().await.unwrap();
        assert!(result.is_none());

        // The pending queue should now be empty (ghost cleaned up)
        assert_eq!(backend.pending_queue.read().len(), 0);
    }

    #[tokio::test]
    async fn test_multiple_ghost_jobs_cleanup() {
        let config = crate::config::QueueConfigBuilder::testing()
            .build()
            .expect("Failed to build config");
        let backend = MemoryBackend::new(config);

        // Create multiple jobs
        let mut job_ids = Vec::new();
        for i in 1..=5 {
            let job = TestJob {
                id: i,
                message: format!("ghost test {}", i),
            };
            let entry = JobEntry::new(job, Some(Priority::Normal), None).unwrap();
            let job_id = backend.enqueue(entry).await.unwrap();
            job_ids.push(job_id);
        }

        // Verify all jobs are queued
        assert_eq!(backend.pending_queue.read().len(), 5);
        assert_eq!(backend.jobs.len(), 5);

        // Remove first 3 jobs from jobs map (creating ghosts)
        for &job_id in &job_ids[0..3] {
            backend.jobs.remove(&job_id);
        }

        // Pending queue still has all 5 entries
        assert_eq!(backend.pending_queue.read().len(), 5);
        assert_eq!(backend.jobs.len(), 2);

        // First dequeue should skip the 3 ghost jobs and return the 4th job
        let result = backend.dequeue().await.unwrap().unwrap();
        assert_eq!(result.payload.get("id").unwrap().as_u64().unwrap(), 4);

        // Pending queue should now have cleaned up the ghosts plus the dequeued job
        assert_eq!(backend.pending_queue.read().len(), 1);
    }

    #[tokio::test]
    async fn test_delayed_job() {
        let backend = MemoryBackend::new(QueueConfig::default());

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
        tokio::time::sleep(delay + Duration::from_millis(10)).await;

        // Should be available now
        let result = backend.dequeue().await.unwrap();
        assert!(result.is_some());
    }

    #[tokio::test]
    async fn test_job_failure_and_retry() {
        let backend = MemoryBackend::new(QueueConfig::default());

        let job = TestJob {
            id: 1,
            message: "failing job".to_string(),
        };
        let entry = JobEntry::new(job, None, None).unwrap();
        let job_id = entry.id();

        backend.enqueue(entry).await.unwrap();

        // Dequeue and fail the job
        let _job_entry = backend.dequeue().await.unwrap().unwrap();
        let error = Box::new(std::io::Error::new(std::io::ErrorKind::Other, "test error"));
        backend.complete(job_id, Err(error)).await.unwrap();

        // Job should be marked as failed and available for retry
        let stats = backend.stats().await.unwrap();
        assert_eq!(stats.failed_jobs, 1);
        assert_eq!(stats.processing_jobs, 0);

        // Should be able to get job by state
        let failed_jobs = backend
            .get_jobs_by_state(JobState::Failed, None)
            .await
            .unwrap();
        assert_eq!(failed_jobs.len(), 1);
        assert_eq!(failed_jobs[0].attempts(), 1);
    }

    #[tokio::test]
    async fn test_queue_size_limit() {
        let config = crate::config::QueueConfigBuilder::new()
            .max_queue_size(2)
            .build()
            .expect("Failed to build config");
        let backend = MemoryBackend::new(config);

        // Enqueue up to limit
        for i in 1..=2 {
            let job = TestJob {
                id: i,
                message: format!("job {}", i),
            };
            let entry = JobEntry::new(job, None, None).unwrap();
            backend.enqueue(entry).await.unwrap();
        }

        // Third job should fail
        let job = TestJob {
            id: 3,
            message: "overflow job".to_string(),
        };
        let entry = JobEntry::new(job, None, None).unwrap();
        let result = backend.enqueue(entry).await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), QueueError::Configuration(_)));
    }
}
