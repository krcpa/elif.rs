# Rate Limiting Algorithms - Technical Deep Dive

## Overview
Phase 11 introduces multiple sophisticated rate limiting algorithms, each optimized for different use cases and traffic patterns.

## Algorithm Comparison

| Algorithm | Accuracy | Memory Usage | CPU Usage | Use Case |
|-----------|----------|--------------|-----------|----------|
| Sliding Window | Good | Low | Low | General purpose, current implementation |
| Token Bucket | Good | Low | Low | Burst handling, API quotas |
| Leaky Bucket | Excellent | Low | Low | Smooth rate limiting, video streaming |
| Sliding Window Log | Perfect | High | Medium | Precise limits, financial APIs |
| Adaptive | Good | Medium | High | Dynamic traffic, ML-driven |

## Algorithm Implementations

### 1. Enhanced Sliding Window
**Current Implementation (Enhanced)**

```rust
pub struct SlidingWindowAlgorithm {
    precision: WindowPrecision,
    sub_windows: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WindowPrecision {
    Second,      // 1-second sub-windows
    Minute,      // 1-minute sub-windows  
    TenSecond,   // 10-second sub-windows
    Custom(u32), // Custom sub-window size
}
```

**Advantages:**
- Memory efficient (O(1) per identifier)
- Fast decision making (<0.01ms)
- Good approximation for most use cases
- Already implemented and tested

**Disadvantages:**
- Potential burst at window boundaries
- Less accurate than true sliding window
- Fixed window alignment

**Best For:** General web APIs, high-throughput services

### 2. Token Bucket Algorithm
**New Implementation**

```rust
pub struct TokenBucketAlgorithm {
    capacity: u64,           // Maximum tokens
    refill_rate: f64,        // Tokens per second
    refill_interval: Duration, // How often to refill
}

struct TokenBucket {
    tokens: f64,
    last_refill: Instant,
    capacity: u64,
    refill_rate: f64,
}

impl RateLimitAlgorithm for TokenBucketAlgorithm {
    async fn check_rate_limit(
        &self,
        identifier: &str,
        storage: &dyn Storage,
    ) -> Result<RateLimitDecision, RateLimitError> {
        let now = Instant::now();
        let mut bucket = storage.get_bucket(identifier).await?
            .unwrap_or_else(|| TokenBucket::new(self.capacity, self.refill_rate));
        
        // Refill tokens based on elapsed time
        let elapsed = now.duration_since(bucket.last_refill).as_secs_f64();
        let new_tokens = (elapsed * self.refill_rate).min(self.capacity as f64 - bucket.tokens);
        bucket.tokens += new_tokens;
        bucket.last_refill = now;
        
        // Check if request can be allowed
        if bucket.tokens >= 1.0 {
            bucket.tokens -= 1.0;
            storage.set_bucket(identifier, bucket).await?;
            
            Ok(RateLimitDecision::Allow {
                remaining: bucket.tokens as u64,
                reset_time: calculate_next_token_time(&bucket),
                headers: generate_token_bucket_headers(&bucket),
            })
        } else {
            Ok(RateLimitDecision::Deny {
                retry_after: calculate_retry_time(&bucket),
                headers: generate_exceeded_headers(&bucket),
            })
        }
    }
}
```

**Advantages:**
- Excellent burst handling
- Natural quota management
- Industry standard for APIs
- Intuitive configuration

**Disadvantages:**
- Slightly more complex than sliding window
- Requires floating-point calculations
- Token state persistence needed

**Best For:** API quotas, burst-tolerant services, mobile apps

### 3. Leaky Bucket Algorithm
**New Implementation**

```rust
pub struct LeakyBucketAlgorithm {
    capacity: u64,      // Maximum requests in bucket
    leak_rate: f64,     // Requests processed per second
}

struct LeakyBucket {
    volume: f64,        // Current volume
    last_leak: Instant, // Last leak time
}

impl RateLimitAlgorithm for LeakyBucketAlgorithm {
    async fn check_rate_limit(
        &self,
        identifier: &str,
        storage: &dyn Storage,
    ) -> Result<RateLimitDecision, RateLimitError> {
        let now = Instant::now();
        let mut bucket = storage.get_leaky_bucket(identifier).await?
            .unwrap_or_else(|| LeakyBucket::new());
        
        // Leak requests at constant rate
        let elapsed = now.duration_since(bucket.last_leak).as_secs_f64();
        let leaked = elapsed * self.leak_rate;
        bucket.volume = (bucket.volume - leaked).max(0.0);
        bucket.last_leak = now;
        
        // Check if bucket has capacity
        if bucket.volume < self.capacity as f64 {
            bucket.volume += 1.0;
            storage.set_leaky_bucket(identifier, bucket).await?;
            
            Ok(RateLimitDecision::Allow {
                remaining: (self.capacity as f64 - bucket.volume) as u64,
                reset_time: calculate_leak_time(&bucket, self.leak_rate),
                headers: generate_leaky_bucket_headers(&bucket),
            })
        } else {
            Ok(RateLimitDecision::Deny {
                retry_after: calculate_leak_retry_time(&bucket, self.leak_rate),
                headers: generate_overflow_headers(&bucket),
            })
        }
    }
}
```

**Advantages:**
- Perfectly smooth rate limiting
- No burst spikes
- Predictable output rate
- Good for streaming services

**Disadvantages:**
- No burst tolerance
- May seem restrictive to users
- Requires continuous state updates

**Best For:** Video streaming, file downloads, bandwidth limiting

### 4. Sliding Window Log Algorithm
**New Implementation**

```rust
pub struct SlidingWindowLogAlgorithm {
    window_size: Duration,
    max_requests: u64,
    log_retention: Duration,
    cleanup_interval: Duration,
}

struct RequestLog {
    timestamps: VecDeque<Instant>,
    last_cleanup: Instant,
}

impl RateLimitAlgorithm for SlidingWindowLogAlgorithm {
    async fn check_rate_limit(
        &self,
        identifier: &str,
        storage: &dyn Storage,
    ) -> Result<RateLimitDecision, RateLimitError> {
        let now = Instant::now();
        let mut log = storage.get_request_log(identifier).await?
            .unwrap_or_else(|| RequestLog::new());
        
        // Clean up old requests
        let window_start = now - self.window_size;
        while let Some(&front) = log.timestamps.front() {
            if front < window_start {
                log.timestamps.pop_front();
            } else {
                break;
            }
        }
        
        // Check if under limit
        if log.timestamps.len() < self.max_requests as usize {
            log.timestamps.push_back(now);
            storage.set_request_log(identifier, log).await?;
            
            Ok(RateLimitDecision::Allow {
                remaining: self.max_requests - log.timestamps.len() as u64,
                reset_time: calculate_log_reset_time(&log, self.window_size),
                headers: generate_precise_headers(&log, self.max_requests),
            })
        } else {
            Ok(RateLimitDecision::Deny {
                retry_after: calculate_log_retry_time(&log, self.window_size),
                headers: generate_precise_exceeded_headers(&log),
            })
        }
    }
}
```

**Advantages:**
- Perfect accuracy
- True sliding window behavior
- No boundary effects
- Detailed request tracking

**Disadvantages:**
- High memory usage (O(n) per identifier)
- Complex cleanup logic
- Potential performance impact
- Not suitable for high-volume APIs

**Best For:** Financial APIs, precise billing, compliance requirements

### 5. Adaptive Algorithm (ML-Driven)
**Advanced Implementation**

```rust
pub struct AdaptiveAlgorithm {
    learning_window: Duration,
    adjustment_factor: f64,
    min_limit: u64,
    max_limit: u64,
    base_algorithm: Box<dyn RateLimitAlgorithm>,
}

struct AdaptiveState {
    current_limit: u64,
    traffic_history: CircularBuffer<TrafficSample>,
    last_adjustment: Instant,
    performance_score: f64,
}

struct TrafficSample {
    timestamp: Instant,
    requests_per_second: f64,
    error_rate: f64,
    average_latency: Duration,
}

impl RateLimitAlgorithm for AdaptiveAlgorithm {
    async fn check_rate_limit(
        &self,
        identifier: &str,
        storage: &dyn Storage,
    ) -> Result<RateLimitDecision, RateLimitError> {
        let now = Instant::now();
        let mut state = storage.get_adaptive_state(identifier).await?
            .unwrap_or_else(|| AdaptiveState::new(self.min_limit));
        
        // Collect current traffic sample
        let current_sample = TrafficSample {
            timestamp: now,
            requests_per_second: calculate_current_rps(&state),
            error_rate: calculate_current_error_rate(&state),
            average_latency: calculate_current_latency(&state),
        };
        
        state.traffic_history.push(current_sample);
        
        // Adjust limits based on performance
        if should_adjust_limits(&state, self.learning_window) {
            let new_limit = calculate_optimal_limit(
                &state.traffic_history,
                self.adjustment_factor,
                self.min_limit,
                self.max_limit,
            );
            
            state.current_limit = new_limit;
            state.last_adjustment = now;
        }
        
        // Use base algorithm with adaptive limit
        let mut config = self.base_algorithm.get_config();
        config.max_requests = state.current_limit;
        
        let decision = self.base_algorithm
            .with_config(config)
            .check_rate_limit(identifier, storage)
            .await?;
        
        storage.set_adaptive_state(identifier, state).await?;
        Ok(decision)
    }
}
```

**Advantages:**
- Self-optimizing limits
- Adapts to traffic patterns
- Learns from system performance
- Reduces false positives

**Disadvantages:**
- Complex implementation
- Requires performance metrics
- Potential instability
- Difficult to debug

**Best For:** Dynamic APIs, machine learning services, experimental features

## Algorithm Selection Guidelines

### Traffic Pattern Analysis

```rust
pub struct TrafficAnalyzer;

impl TrafficAnalyzer {
    pub fn recommend_algorithm(
        &self,
        traffic_pattern: &TrafficPattern,
        performance_requirements: &PerformanceRequirements,
        business_requirements: &BusinessRequirements,
    ) -> AlgorithmRecommendation {
        match traffic_pattern {
            TrafficPattern::Steady { avg_rps, variance } => {
                if *variance < 0.2 {
                    AlgorithmRecommendation::LeakyBucket {
                        capacity: (avg_rps * 10.0) as u64,
                        leak_rate: *avg_rps,
                    }
                } else {
                    AlgorithmRecommendation::SlidingWindow {
                        precision: WindowPrecision::Minute,
                    }
                }
            }
            
            TrafficPattern::Bursty { peak_rps, avg_rps, burst_duration } => {
                let burst_capacity = (peak_rps * burst_duration.as_secs_f64()) as u64;
                AlgorithmRecommendation::TokenBucket {
                    capacity: burst_capacity,
                    refill_rate: *avg_rps,
                }
            }
            
            TrafficPattern::Irregular { .. } => {
                if performance_requirements.accuracy == AccuracyLevel::Precise {
                    AlgorithmRecommendation::SlidingWindowLog {
                        max_requests: calculate_conservative_limit(traffic_pattern),
                        window_size: Duration::from_secs(3600),
                    }
                } else {
                    AlgorithmRecommendation::Adaptive {
                        base_algorithm: Box::new(TokenBucketAlgorithm::default()),
                        learning_window: Duration::from_secs(300),
                    }
                }
            }
            
            TrafficPattern::Seasonal { patterns, .. } => {
                AlgorithmRecommendation::Adaptive {
                    base_algorithm: Box::new(SlidingWindowAlgorithm::default()),
                    learning_window: Duration::from_secs(3600),
                }
            }
        }
    }
}
```

### Configuration Examples

```rust
// High-throughput API with burst tolerance
let api_config = RateLimitRule {
    max_requests: 10000,
    window_seconds: 3600,
    algorithm: RateLimitAlgorithm::TokenBucket {
        refill_rate: 10000.0 / 3600.0, // ~2.78 requests/second
        burst_capacity: 100,           // Allow 100 request burst
    },
    identifier: EnhancedIdentifier::ApiKey {
        quota_field: Some("monthly_quota".to_string()),
        key_prefix: None,
    },
    headers: HeaderConfig::detailed(),
};

// Video streaming with smooth rate limiting
let streaming_config = RateLimitRule {
    max_requests: 1000,  // 1000 chunks
    window_seconds: 3600, // per hour
    algorithm: RateLimitAlgorithm::LeakyBucket {
        leak_rate: 1000.0 / 3600.0,  // ~0.28 chunks/second
        capacity: 50,                 // 50 chunk buffer
    },
    identifier: EnhancedIdentifier::UserId {
        tier_field: Some("subscription_tier".to_string()),
        fallback_to_ip: true,
    },
    headers: HeaderConfig::streaming_optimized(),
};

// Financial API with precise limits
let financial_config = RateLimitRule {
    max_requests: 100,
    window_seconds: 3600,
    algorithm: RateLimitAlgorithm::SlidingWindowLog {
        log_retention: 7200,  // Keep 2 hours of logs
        cleanup_interval: 60, // Clean every minute
    },
    identifier: EnhancedIdentifier::Composite {
        primary: Box::new(EnhancedIdentifier::ApiKey {
            quota_field: Some("trading_quota".to_string()),
            key_prefix: Some("TRADE_".to_string()),
        }),
        secondary: Box::new(EnhancedIdentifier::IpAddress {
            use_forwarded: true,
            geolocation: true,
            reputation_check: true,
        }),
        strategy: CompositeStrategy::StrictestLimit,
    },
    headers: HeaderConfig::compliance_mode(),
};
```

## Performance Optimizations

### Algorithm-Specific Optimizations

1. **Sliding Window**: Use bit manipulation for sub-window tracking
2. **Token Bucket**: Batch token refills to reduce storage operations
3. **Leaky Bucket**: Implement leak calculations using integer arithmetic
4. **Sliding Window Log**: Use circular buffers and lazy cleanup
5. **Adaptive**: Cache ML model predictions and update asynchronously

### Storage Optimizations

- **Compression**: Use compact binary formats for storage
- **Batching**: Batch multiple rate limit checks in single storage operation
- **Caching**: Multi-level caching (L1: memory, L2: Redis)
- **Partitioning**: Shard rate limit data across multiple storage instances

### Memory Management

- **Object Pooling**: Reuse algorithm state objects
- **Lazy Cleanup**: Clean expired data only when necessary
- **Memory Limits**: Set maximum memory usage per algorithm
- **Garbage Collection**: Optimize for low-latency GC pauses

## Testing and Validation

### Algorithm Correctness Tests
```rust
#[cfg(test)]
mod algorithm_tests {
    use super::*;
    
    #[tokio::test]
    async fn test_token_bucket_burst_handling() {
        let algorithm = TokenBucketAlgorithm {
            capacity: 100,
            refill_rate: 10.0, // 10 tokens per second
        };
        
        let storage = MemoryStorage::new();
        let identifier = "test_user";
        
        // Should allow burst up to capacity
        for i in 0..100 {
            let decision = algorithm.check_rate_limit(identifier, &storage).await?;
            assert!(matches!(decision, RateLimitDecision::Allow { .. }));
        }
        
        // 101st request should be denied
        let decision = algorithm.check_rate_limit(identifier, &storage).await?;
        assert!(matches!(decision, RateLimitDecision::Deny { .. }));
        
        // Wait for token refill
        tokio::time::sleep(Duration::from_secs(1)).await;
        
        // Should allow ~10 more requests
        for i in 0..10 {
            let decision = algorithm.check_rate_limit(identifier, &storage).await?;
            assert!(matches!(decision, RateLimitDecision::Allow { .. }));
        }
    }
    
    #[tokio::test]
    async fn test_leaky_bucket_smooth_rate() {
        let algorithm = LeakyBucketAlgorithm {
            capacity: 10,
            leak_rate: 1.0, // 1 request per second
        };
        
        let storage = MemoryStorage::new();
        let identifier = "test_user";
        
        // Fill bucket to capacity
        for i in 0..10 {
            let decision = algorithm.check_rate_limit(identifier, &storage).await?;
            assert!(matches!(decision, RateLimitDecision::Allow { .. }));
        }
        
        // Should deny further requests
        let decision = algorithm.check_rate_limit(identifier, &storage).await?;
        assert!(matches!(decision, RateLimitDecision::Deny { .. }));
        
        // Wait for leak
        tokio::time::sleep(Duration::from_secs(1)).await;
        
        // Should allow exactly 1 more request
        let decision = algorithm.check_rate_limit(identifier, &storage).await?;
        assert!(matches!(decision, RateLimitDecision::Allow { .. }));
        
        let decision = algorithm.check_rate_limit(identifier, &storage).await?;
        assert!(matches!(decision, RateLimitDecision::Deny { .. }));
    }
}
```

### Performance Benchmarks
```rust
#[cfg(test)]
mod benchmarks {
    use criterion::{black_box, criterion_group, criterion_main, Criterion};
    
    fn benchmark_algorithms(c: &mut Criterion) {
        let mut group = c.benchmark_group("rate_limiting_algorithms");
        
        group.bench_function("sliding_window", |b| {
            let algorithm = SlidingWindowAlgorithm::default();
            let storage = MemoryStorage::new();
            
            b.iter(|| {
                black_box(algorithm.check_rate_limit("user123", &storage))
            })
        });
        
        group.bench_function("token_bucket", |b| {
            let algorithm = TokenBucketAlgorithm::default();
            let storage = MemoryStorage::new();
            
            b.iter(|| {
                black_box(algorithm.check_rate_limit("user123", &storage))
            })
        });
        
        group.bench_function("leaky_bucket", |b| {
            let algorithm = LeakyBucketAlgorithm::default();
            let storage = MemoryStorage::new();
            
            b.iter(|| {
                black_box(algorithm.check_rate_limit("user123", &storage))
            })
        });
        
        group.finish();
    }
    
    criterion_group!(benches, benchmark_algorithms);
    criterion_main!(benches);
}
```

## Migration from Phase 3.13

### Backward Compatibility
The enhanced rate limiting system maintains full backward compatibility with the Phase 3.13 implementation:

```rust
// Phase 3.13 configuration continues to work
let basic_config = RateLimitConfig {
    max_requests: 100,
    window_seconds: 60,
    identifier: RateLimitIdentifier::IpAddress,
    exempt_paths: HashSet::new(),
};

// Automatically converts to enterprise config with sliding window
let enterprise_config = EnterpriseRateLimitConfig::from_basic(basic_config);
assert_eq!(enterprise_config.global.free_tier.algorithm, 
           RateLimitAlgorithm::SlidingWindow { precision: WindowPrecision::Minute });
```

### Gradual Algorithm Migration
```rust
// Start with current algorithm
let config = EnterpriseRateLimitConfig::builder()
    .algorithm(RateLimitAlgorithm::SlidingWindow { 
        precision: WindowPrecision::Minute 
    })
    .build();

// A/B test new algorithm
let ab_test_config = config.clone()
    .enable_ab_test("token_bucket_test", 0.1) // 10% of traffic
    .add_test_algorithm(RateLimitAlgorithm::TokenBucket {
        refill_rate: 100.0 / 60.0,
        burst_capacity: 20,
    });

// Monitor performance and gradually increase rollout
management_api.update_ab_test("token_bucket_test", 0.5).await?; // 50%
management_api.update_ab_test("token_bucket_test", 1.0).await?; // 100%

// Make it permanent
let final_config = config.set_algorithm(RateLimitAlgorithm::TokenBucket {
    refill_rate: 100.0 / 60.0,
    burst_capacity: 20,
});
```

---

This technical deep dive provides the foundation for implementing multiple sophisticated rate limiting algorithms in Phase 11, each optimized for different use cases while maintaining backward compatibility with the current Phase 3.13 implementation.