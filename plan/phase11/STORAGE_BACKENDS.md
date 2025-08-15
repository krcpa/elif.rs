# Distributed Storage Backends - Architecture & Implementation

## Overview
Phase 11 introduces multiple storage backends for enterprise-grade rate limiting, moving beyond the current in-memory solution to support distributed deployments, persistence, and high availability.

## Storage Architecture

### Core Abstraction
```rust
#[async_trait]
pub trait RateLimitStorage: Send + Sync {
    /// Get rate limit state for an identifier
    async fn get_state(&self, identifier: &str) -> Result<Option<RateLimitState>, StorageError>;
    
    /// Set rate limit state for an identifier
    async fn set_state(&self, identifier: &str, state: RateLimitState, ttl: Option<Duration>) -> Result<(), StorageError>;
    
    /// Atomic increment operation for counters
    async fn increment(&self, identifier: &str, amount: u64, window: Duration) -> Result<u64, StorageError>;
    
    /// Batch operations for performance
    async fn batch_get(&self, identifiers: &[&str]) -> Result<Vec<(String, Option<RateLimitState>)>, StorageError>;
    async fn batch_set(&self, operations: &[(String, RateLimitState, Option<Duration>)]) -> Result<(), StorageError>;
    
    /// Health check and monitoring
    async fn health_check(&self) -> Result<StorageHealth, StorageError>;
    
    /// Cleanup expired entries
    async fn cleanup_expired(&self) -> Result<u64, StorageError>;
    
    /// Get storage statistics
    async fn get_stats(&self) -> Result<StorageStats, StorageError>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitState {
    pub algorithm_data: AlgorithmData,
    pub created_at: Instant,
    pub last_accessed: Instant,
    pub request_count: u64,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlgorithmData {
    SlidingWindow {
        windows: Vec<WindowData>,
        current_window: u32,
    },
    TokenBucket {
        tokens: f64,
        last_refill: Instant,
    },
    LeakyBucket {
        volume: f64,
        last_leak: Instant,
    },
    SlidingWindowLog {
        requests: VecDeque<Instant>,
    },
    Adaptive {
        current_limit: u64,
        traffic_history: Vec<TrafficSample>,
        performance_score: f64,
    },
}
```

## Storage Backend Implementations

### 1. Enhanced Memory Storage
**Current Implementation (Enhanced)**

```rust
pub struct EnhancedMemoryStorage {
    data: Arc<DashMap<String, (RateLimitState, Instant)>>, // (state, expires_at)
    config: MemoryStorageConfig,
    metrics: Arc<Mutex<StorageMetrics>>,
    cleanup_handle: Option<JoinHandle<()>>,
}

#[derive(Debug, Clone)]
pub struct MemoryStorageConfig {
    pub max_entries: usize,
    pub cleanup_interval: Duration,
    pub eviction_policy: EvictionPolicy,
    pub memory_limit: Option<usize>,
    pub persist_to_disk: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub enum EvictionPolicy {
    LRU,         // Least Recently Used
    LFU,         // Least Frequently Used
    TTL,         // Time To Live based
    Random,      // Random eviction
    AdaptiveLRU, // LRU with frequency consideration
}

impl RateLimitStorage for EnhancedMemoryStorage {
    async fn get_state(&self, identifier: &str) -> Result<Option<RateLimitState>, StorageError> {
        // Update metrics
        self.metrics.lock().await.record_read();
        
        if let Some((state, expires_at)) = self.data.get(identifier) {
            if Instant::now() < *expires_at {
                // Update access time for LRU
                let mut entry = state.clone();
                entry.last_accessed = Instant::now();
                self.data.insert(identifier.to_string(), (entry.clone(), *expires_at));
                Ok(Some(entry))
            } else {
                // Expired entry
                self.data.remove(identifier);
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }
    
    async fn set_state(&self, identifier: &str, state: RateLimitState, ttl: Option<Duration>) -> Result<(), StorageError> {
        // Check memory limits
        self.ensure_capacity().await?;
        
        let expires_at = ttl.map(|ttl| Instant::now() + ttl)
            .unwrap_or(Instant::now() + Duration::from_secs(3600));
        
        self.data.insert(identifier.to_string(), (state, expires_at));
        self.metrics.lock().await.record_write();
        
        // Persist to disk if configured
        if let Some(ref persist_path) = self.config.persist_to_disk {
            self.persist_entry(identifier, &state, expires_at).await?;
        }
        
        Ok(())
    }
    
    async fn increment(&self, identifier: &str, amount: u64, _window: Duration) -> Result<u64, StorageError> {
        let mut new_count = amount;
        
        self.data.alter(identifier, |existing| {
            match existing {
                Some((mut state, expires_at)) => {
                    state.request_count += amount;
                    state.last_accessed = Instant::now();
                    new_count = state.request_count;
                    Some((state, expires_at))
                }
                None => {
                    let state = RateLimitState {
                        algorithm_data: AlgorithmData::Counter { count: amount },
                        created_at: Instant::now(),
                        last_accessed: Instant::now(),
                        request_count: amount,
                        metadata: HashMap::new(),
                    };
                    let expires_at = Instant::now() + Duration::from_secs(3600);
                    Some((state, expires_at))
                }
            }
        });
        
        self.metrics.lock().await.record_increment();
        Ok(new_count)
    }
}

impl EnhancedMemoryStorage {
    async fn ensure_capacity(&self) -> Result<(), StorageError> {
        while self.data.len() >= self.config.max_entries {
            match self.config.eviction_policy {
                EvictionPolicy::LRU => self.evict_lru().await?,
                EvictionPolicy::LFU => self.evict_lfu().await?,
                EvictionPolicy::TTL => self.evict_expired().await?,
                EvictionPolicy::Random => self.evict_random().await?,
                EvictionPolicy::AdaptiveLRU => self.evict_adaptive_lru().await?,
            }
        }
        Ok(())
    }
    
    async fn evict_lru(&self) -> Result<(), StorageError> {
        let mut oldest_key = None;
        let mut oldest_time = Instant::now();
        
        for entry in self.data.iter() {
            let (state, _) = entry.value();
            if state.last_accessed < oldest_time {
                oldest_time = state.last_accessed;
                oldest_key = Some(entry.key().clone());
            }
        }
        
        if let Some(key) = oldest_key {
            self.data.remove(&key);
            self.metrics.lock().await.record_eviction();
        }
        
        Ok(())
    }
}
```

**Advantages:**
- Fastest performance (<0.01ms latency)
- No network overhead
- Simple debugging and monitoring
- Automatic cleanup and eviction policies

**Disadvantages:**
- Data lost on restart
- No sharing between instances
- Memory usage grows with active users
- Single point of failure

### 2. Redis Distributed Storage
**New Implementation**

```rust
pub struct RedisStorage {
    client: redis::Client,
    pool: bb8::Pool<RedisConnectionManager>,
    config: RedisStorageConfig,
    lua_scripts: LuaScripts,
    metrics: Arc<Mutex<StorageMetrics>>,
}

#[derive(Debug, Clone)]
pub struct RedisStorageConfig {
    pub url: String,
    pub pool_size: u32,
    pub connection_timeout: Duration,
    pub command_timeout: Duration,
    pub cluster_mode: bool,
    pub key_prefix: String,
    pub compression: CompressionType,
    pub retry_policy: RetryPolicy,
}

#[derive(Debug, Clone)]
pub enum CompressionType {
    None,
    Gzip,
    Snappy,
    Lz4,
}

struct LuaScripts {
    increment_script: Script,
    token_bucket_script: Script,
    leaky_bucket_script: Script,
    cleanup_script: Script,
}

impl RateLimitStorage for RedisStorage {
    async fn get_state(&self, identifier: &str) -> Result<Option<RateLimitState>, StorageError> {
        let mut conn = self.pool.get().await?;
        let key = format!("{}:{}", self.config.key_prefix, identifier);
        
        let start_time = Instant::now();
        let data: Option<Vec<u8>> = conn.get(&key).await?;
        self.metrics.lock().await.record_read_latency(start_time.elapsed());
        
        match data {
            Some(bytes) => {
                let decompressed = self.decompress(&bytes)?;
                let state: RateLimitState = bincode::deserialize(&decompressed)?;
                Ok(Some(state))
            }
            None => Ok(None),
        }
    }
    
    async fn set_state(&self, identifier: &str, state: RateLimitState, ttl: Option<Duration>) -> Result<(), StorageError> {
        let mut conn = self.pool.get().await?;
        let key = format!("{}:{}", self.config.key_prefix, identifier);
        
        let serialized = bincode::serialize(&state)?;
        let compressed = self.compress(&serialized)?;
        
        let start_time = Instant::now();
        if let Some(ttl) = ttl {
            conn.set_ex(&key, compressed, ttl.as_secs()).await?;
        } else {
            conn.set(&key, compressed).await?;
        }
        self.metrics.lock().await.record_write_latency(start_time.elapsed());
        
        Ok(())
    }
    
    async fn increment(&self, identifier: &str, amount: u64, window: Duration) -> Result<u64, StorageError> {
        let mut conn = self.pool.get().await?;
        let key = format!("{}:{}", self.config.key_prefix, identifier);
        
        // Use Lua script for atomic increment with TTL
        let result: u64 = self.lua_scripts.increment_script
            .key(&key)
            .arg(amount)
            .arg(window.as_secs())
            .invoke_async(&mut conn)
            .await?;
        
        self.metrics.lock().await.record_increment();
        Ok(result)
    }
    
    async fn batch_get(&self, identifiers: &[&str]) -> Result<Vec<(String, Option<RateLimitState>)>, StorageError> {
        let mut conn = self.pool.get().await?;
        let keys: Vec<String> = identifiers.iter()
            .map(|id| format!("{}:{}", self.config.key_prefix, id))
            .collect();
        
        let start_time = Instant::now();
        let results: Vec<Option<Vec<u8>>> = redis::cmd("MGET")
            .arg(&keys)
            .query_async(&mut conn)
            .await?;
        self.metrics.lock().await.record_batch_read_latency(start_time.elapsed());
        
        let mut output = Vec::new();
        for (i, data) in results.into_iter().enumerate() {
            let identifier = identifiers[i].to_string();
            match data {
                Some(bytes) => {
                    let decompressed = self.decompress(&bytes)?;
                    let state: RateLimitState = bincode::deserialize(&decompressed)?;
                    output.push((identifier, Some(state)));
                }
                None => output.push((identifier, None)),
            }
        }
        
        Ok(output)
    }
}

impl RedisStorage {
    pub async fn new(config: RedisStorageConfig) -> Result<Self, StorageError> {
        let client = redis::Client::open(config.url.clone())?;
        
        let manager = RedisConnectionManager::new(client.clone())?;
        let pool = bb8::Pool::builder()
            .max_size(config.pool_size)
            .connection_timeout(config.connection_timeout)
            .build(manager)
            .await?;
        
        let lua_scripts = Self::load_lua_scripts(&client).await?;
        
        Ok(Self {
            client,
            pool,
            config,
            lua_scripts,
            metrics: Arc::new(Mutex::new(StorageMetrics::new())),
        })
    }
    
    async fn load_lua_scripts(client: &redis::Client) -> Result<LuaScripts, StorageError> {
        let increment_script = Script::new(r#"
            local key = KEYS[1]
            local amount = tonumber(ARGV[1])
            local ttl = tonumber(ARGV[2])
            
            local current = redis.call('GET', key)
            if current then
                current = tonumber(current) + amount
            else
                current = amount
                redis.call('EXPIRE', key, ttl)
            end
            
            redis.call('SET', key, current)
            return current
        "#);
        
        let token_bucket_script = Script::new(r#"
            local key = KEYS[1]
            local capacity = tonumber(ARGV[1])
            local refill_rate = tonumber(ARGV[2])
            local now = tonumber(ARGV[3])
            
            local bucket = redis.call('HMGET', key, 'tokens', 'last_refill')
            local tokens = tonumber(bucket[1]) or capacity
            local last_refill = tonumber(bucket[2]) or now
            
            -- Refill tokens
            local elapsed = now - last_refill
            local new_tokens = math.min(capacity, tokens + (elapsed * refill_rate))
            
            if new_tokens >= 1 then
                new_tokens = new_tokens - 1
                redis.call('HMSET', key, 'tokens', new_tokens, 'last_refill', now)
                redis.call('EXPIRE', key, 3600)
                return {1, new_tokens} -- allowed, remaining tokens
            else
                redis.call('HMSET', key, 'tokens', new_tokens, 'last_refill', now)
                return {0, 0} -- denied
            end
        "#);
        
        Ok(LuaScripts {
            increment_script,
            token_bucket_script,
            leaky_bucket_script: Script::new("..."), // Similar implementation
            cleanup_script: Script::new("..."),      // Cleanup expired keys
        })
    }
    
    fn compress(&self, data: &[u8]) -> Result<Vec<u8>, StorageError> {
        match self.config.compression {
            CompressionType::None => Ok(data.to_vec()),
            CompressionType::Gzip => {
                use flate2::write::GzEncoder;
                use flate2::Compression;
                use std::io::Write;
                
                let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
                encoder.write_all(data)?;
                Ok(encoder.finish()?)
            }
            CompressionType::Snappy => {
                Ok(snap::raw::Encoder::new().compress_vec(data)?)
            }
            CompressionType::Lz4 => {
                Ok(lz4_flex::compress_prepend_size(data))
            }
        }
    }
    
    fn decompress(&self, data: &[u8]) -> Result<Vec<u8>, StorageError> {
        match self.config.compression {
            CompressionType::None => Ok(data.to_vec()),
            CompressionType::Gzip => {
                use flate2::read::GzDecoder;
                use std::io::Read;
                
                let mut decoder = GzDecoder::new(data);
                let mut decompressed = Vec::new();
                decoder.read_to_end(&mut decompressed)?;
                Ok(decompressed)
            }
            CompressionType::Snappy => {
                Ok(snap::raw::Decoder::new().decompress_vec(data)?)
            }
            CompressionType::Lz4 => {
                Ok(lz4_flex::decompress_size_prepended(data)?)
            }
        }
    }
}
```

**Advantages:**
- Persistent storage survives restarts
- Shared state across multiple instances
- Built-in clustering and replication
- Atomic operations with Lua scripts
- High performance with connection pooling

**Disadvantages:**
- Network latency (0.1-1ms typical)
- Additional infrastructure requirement
- Memory usage on Redis server
- Potential single point of failure

### 3. PostgreSQL Persistent Storage
**New Implementation**

```rust
pub struct PostgresStorage {
    pool: PgPool,
    config: PostgresStorageConfig,
    prepared_statements: PreparedStatements,
    metrics: Arc<Mutex<StorageMetrics>>,
}

#[derive(Debug, Clone)]
pub struct PostgresStorageConfig {
    pub database_url: String,
    pub pool_size: u32,
    pub connection_timeout: Duration,
    pub query_timeout: Duration,
    pub table_name: String,
    pub partition_strategy: PartitionStrategy,
    pub cleanup_batch_size: u32,
}

#[derive(Debug, Clone)]
pub enum PartitionStrategy {
    None,
    ByTime(Duration),     // Partition by time ranges
    ByHash(u32),          // Hash-based partitioning
    ByIdentifier(String), // Partition by identifier pattern
}

struct PreparedStatements {
    get_state: String,
    set_state: String,
    increment_counter: String,
    batch_get: String,
    cleanup_expired: String,
}

impl RateLimitStorage for PostgresStorage {
    async fn get_state(&self, identifier: &str) -> Result<Option<RateLimitState>, StorageError> {
        let start_time = Instant::now();
        
        let row = sqlx::query(&self.prepared_statements.get_state)
            .bind(identifier)
            .fetch_optional(&self.pool)
            .await?;
        
        self.metrics.lock().await.record_read_latency(start_time.elapsed());
        
        match row {
            Some(row) => {
                let data: Vec<u8> = row.get("state_data");
                let expires_at: Option<DateTime<Utc>> = row.get("expires_at");
                
                // Check expiration
                if let Some(expires_at) = expires_at {
                    if Utc::now() > expires_at {
                        // Expired, clean up asynchronously
                        self.cleanup_expired_entry(identifier).await;
                        return Ok(None);
                    }
                }
                
                let state: RateLimitState = bincode::deserialize(&data)?;
                Ok(Some(state))
            }
            None => Ok(None),
        }
    }
    
    async fn set_state(&self, identifier: &str, state: RateLimitState, ttl: Option<Duration>) -> Result<(), StorageError> {
        let data = bincode::serialize(&state)?;
        let expires_at = ttl.map(|ttl| Utc::now() + chrono::Duration::from_std(ttl).unwrap());
        
        let start_time = Instant::now();
        
        sqlx::query(&self.prepared_statements.set_state)
            .bind(identifier)
            .bind(&data)
            .bind(expires_at)
            .bind(Utc::now()) // updated_at
            .execute(&self.pool)
            .await?;
        
        self.metrics.lock().await.record_write_latency(start_time.elapsed());
        Ok(())
    }
    
    async fn increment(&self, identifier: &str, amount: u64, window: Duration) -> Result<u64, StorageError> {
        let expires_at = Utc::now() + chrono::Duration::from_std(window).unwrap();
        
        let start_time = Instant::now();
        
        let row = sqlx::query(&self.prepared_statements.increment_counter)
            .bind(identifier)
            .bind(amount as i64)
            .bind(expires_at)
            .fetch_one(&self.pool)
            .await?;
        
        self.metrics.lock().await.record_increment_latency(start_time.elapsed());
        
        let new_count: i64 = row.get("new_count");
        Ok(new_count as u64)
    }
    
    async fn batch_get(&self, identifiers: &[&str]) -> Result<Vec<(String, Option<RateLimitState>)>, StorageError> {
        let start_time = Instant::now();
        
        let rows = sqlx::query(&self.prepared_statements.batch_get)
            .bind(&identifiers.iter().map(|s| s.to_string()).collect::<Vec<_>>())
            .fetch_all(&self.pool)
            .await?;
        
        self.metrics.lock().await.record_batch_read_latency(start_time.elapsed());
        
        let mut results: HashMap<String, Option<RateLimitState>> = identifiers
            .iter()
            .map(|id| (id.to_string(), None))
            .collect();
        
        for row in rows {
            let identifier: String = row.get("identifier");
            let data: Vec<u8> = row.get("state_data");
            let expires_at: Option<DateTime<Utc>> = row.get("expires_at");
            
            // Check expiration
            if let Some(expires_at) = expires_at {
                if Utc::now() > expires_at {
                    continue; // Skip expired entries
                }
            }
            
            let state: RateLimitState = bincode::deserialize(&data)?;
            results.insert(identifier, Some(state));
        }
        
        Ok(results.into_iter().collect())
    }
    
    async fn cleanup_expired(&self) -> Result<u64, StorageError> {
        let start_time = Instant::now();
        
        let result = sqlx::query(&self.prepared_statements.cleanup_expired)
            .bind(Utc::now())
            .bind(self.config.cleanup_batch_size as i64)
            .execute(&self.pool)
            .await?;
        
        self.metrics.lock().await.record_cleanup_latency(start_time.elapsed());
        Ok(result.rows_affected())
    }
}

impl PostgresStorage {
    pub async fn new(config: PostgresStorageConfig) -> Result<Self, StorageError> {
        let pool = PgPoolOptions::new()
            .max_connections(config.pool_size)
            .acquire_timeout(config.connection_timeout)
            .connect(&config.database_url)
            .await?;
        
        // Create tables if they don't exist
        Self::ensure_tables(&pool, &config).await?;
        
        let prepared_statements = Self::prepare_statements(&config);
        
        Ok(Self {
            pool,
            config,
            prepared_statements,
            metrics: Arc::new(Mutex::new(StorageMetrics::new())),
        })
    }
    
    async fn ensure_tables(pool: &PgPool, config: &PostgresStorageConfig) -> Result<(), StorageError> {
        let create_table_sql = match config.partition_strategy {
            PartitionStrategy::None => format!(r#"
                CREATE TABLE IF NOT EXISTS {} (
                    identifier VARCHAR(255) PRIMARY KEY,
                    state_data BYTEA NOT NULL,
                    expires_at TIMESTAMP WITH TIME ZONE,
                    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
                    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
                );
                
                CREATE INDEX IF NOT EXISTS idx_{}_expires_at ON {} (expires_at);
                CREATE INDEX IF NOT EXISTS idx_{}_updated_at ON {} (updated_at);
            "#, config.table_name, config.table_name, config.table_name, config.table_name, config.table_name),
            
            PartitionStrategy::ByTime(duration) => format!(r#"
                CREATE TABLE IF NOT EXISTS {} (
                    identifier VARCHAR(255) NOT NULL,
                    state_data BYTEA NOT NULL,
                    expires_at TIMESTAMP WITH TIME ZONE,
                    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
                    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
                ) PARTITION BY RANGE (created_at);
                
                -- Create initial partitions for next 30 days
                {}
            "#, config.table_name, Self::generate_time_partitions(&config.table_name, duration, 30)),
            
            PartitionStrategy::ByHash(partitions) => format!(r#"
                CREATE TABLE IF NOT EXISTS {} (
                    identifier VARCHAR(255) NOT NULL,
                    state_data BYTEA NOT NULL,
                    expires_at TIMESTAMP WITH TIME ZONE,
                    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
                    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
                ) PARTITION BY HASH (identifier);
                
                -- Create hash partitions
                {}
            "#, config.table_name, Self::generate_hash_partitions(&config.table_name, *partitions)),
            
            PartitionStrategy::ByIdentifier(_pattern) => {
                // Custom partitioning logic based on identifier patterns
                format!(r#"
                    CREATE TABLE IF NOT EXISTS {} (
                        identifier VARCHAR(255) NOT NULL,
                        state_data BYTEA NOT NULL,
                        expires_at TIMESTAMP WITH TIME ZONE,
                        created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
                        updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
                    );
                "#, config.table_name)
            }
        };
        
        sqlx::query(&create_table_sql)
            .execute(pool)
            .await?;
        
        Ok(())
    }
    
    fn prepare_statements(config: &PostgresStorageConfig) -> PreparedStatements {
        PreparedStatements {
            get_state: format!(
                "SELECT state_data, expires_at FROM {} WHERE identifier = $1",
                config.table_name
            ),
            set_state: format!(
                r#"INSERT INTO {} (identifier, state_data, expires_at, updated_at) 
                   VALUES ($1, $2, $3, $4)
                   ON CONFLICT (identifier) DO UPDATE SET 
                   state_data = EXCLUDED.state_data, 
                   expires_at = EXCLUDED.expires_at,
                   updated_at = EXCLUDED.updated_at"#,
                config.table_name
            ),
            increment_counter: format!(
                r#"INSERT INTO {} (identifier, state_data, expires_at) 
                   VALUES ($1, $2, $3)
                   ON CONFLICT (identifier) DO UPDATE SET 
                   state_data = (get_byte(state_data, 0) + $2)::bytea,
                   expires_at = EXCLUDED.expires_at
                   RETURNING get_byte(state_data, 0) as new_count"#,
                config.table_name
            ),
            batch_get: format!(
                "SELECT identifier, state_data, expires_at FROM {} WHERE identifier = ANY($1)",
                config.table_name
            ),
            cleanup_expired: format!(
                "DELETE FROM {} WHERE expires_at < $1 LIMIT $2",
                config.table_name
            ),
        }
    }
}
```

**Advantages:**
- Persistent storage with ACID guarantees
- Advanced querying and analytics capabilities
- Partitioning for large-scale deployments
- Built-in replication and backup
- SQL-based monitoring and debugging

**Disadvantages:**
- Higher latency (1-5ms typical)
- More complex setup and maintenance
- Higher resource usage
- Potential bottleneck for high-throughput scenarios

### 4. Hybrid Storage (Memory + Redis)
**Advanced Implementation**

```rust
pub struct HybridStorage {
    l1_cache: EnhancedMemoryStorage,
    l2_cache: RedisStorage,
    config: HybridStorageConfig,
    sync_handle: Option<JoinHandle<()>>,
    metrics: Arc<Mutex<HybridStorageMetrics>>,
}

#[derive(Debug, Clone)]
pub struct HybridStorageConfig {
    pub l1_config: MemoryStorageConfig,
    pub l2_config: RedisStorageConfig,
    pub sync_interval: Duration,
    pub write_policy: WritePolicy,
    pub read_policy: ReadPolicy,
    pub consistency_level: ConsistencyLevel,
}

#[derive(Debug, Clone)]
pub enum WritePolicy {
    WriteThrough,    // Write to both L1 and L2 synchronously
    WriteBack,       // Write to L1, sync to L2 asynchronously
    WriteBehind,     // Write to L1, queue for L2
    WriteAround,     // Write only to L2, invalidate L1
}

#[derive(Debug, Clone)]
pub enum ReadPolicy {
    CacheAside,      // Check L1, then L2, update L1
    ReadThrough,     // Always read through L2
    RefreshAhead,    // Proactively refresh from L2
}

#[derive(Debug, Clone)]
pub enum ConsistencyLevel {
    Eventual,        // Best performance, eventual consistency
    Strong,          // Immediate consistency, slower
    Session,         // Consistent within session
}

impl RateLimitStorage for HybridStorage {
    async fn get_state(&self, identifier: &str) -> Result<Option<RateLimitState>, StorageError> {
        let start_time = Instant::now();
        
        match self.config.read_policy {
            ReadPolicy::CacheAside => {
                // Try L1 cache first
                if let Some(state) = self.l1_cache.get_state(identifier).await? {
                    self.metrics.lock().await.record_l1_hit();
                    return Ok(Some(state));
                }
                
                // Try L2 cache
                if let Some(state) = self.l2_cache.get_state(identifier).await? {
                    self.metrics.lock().await.record_l2_hit();
                    
                    // Update L1 cache asynchronously
                    let l1_cache = self.l1_cache.clone();
                    let identifier = identifier.to_string();
                    let state_clone = state.clone();
                    tokio::spawn(async move {
                        let _ = l1_cache.set_state(&identifier, state_clone, Some(Duration::from_secs(300))).await;
                    });
                    
                    return Ok(Some(state));
                }
                
                self.metrics.lock().await.record_cache_miss();
                Ok(None)
            }
            
            ReadPolicy::ReadThrough => {
                // Always read from L2, update L1
                let state = self.l2_cache.get_state(identifier).await?;
                
                if let Some(ref state) = state {
                    let _ = self.l1_cache.set_state(identifier, state.clone(), Some(Duration::from_secs(300))).await;
                }
                
                Ok(state)
            }
            
            ReadPolicy::RefreshAhead => {
                // Check L1 first
                if let Some(mut state) = self.l1_cache.get_state(identifier).await? {
                    let age = start_time.duration_since(state.last_accessed);
                    
                    // If data is getting old, refresh from L2
                    if age > Duration::from_secs(60) {
                        let l2_cache = self.l2_cache.clone();
                        let l1_cache = self.l1_cache.clone();
                        let identifier = identifier.to_string();
                        
                        tokio::spawn(async move {
                            if let Ok(Some(fresh_state)) = l2_cache.get_state(&identifier).await {
                                let _ = l1_cache.set_state(&identifier, fresh_state, Some(Duration::from_secs(300))).await;
                            }
                        });
                    }
                    
                    return Ok(Some(state));
                }
                
                // Fall back to L2
                self.l2_cache.get_state(identifier).await
            }
        }
    }
    
    async fn set_state(&self, identifier: &str, state: RateLimitState, ttl: Option<Duration>) -> Result<(), StorageError> {
        match self.config.write_policy {
            WritePolicy::WriteThrough => {
                // Write to both caches synchronously
                let l1_future = self.l1_cache.set_state(identifier, state.clone(), ttl);
                let l2_future = self.l2_cache.set_state(identifier, state, ttl);
                
                let (l1_result, l2_result) = tokio::try_join!(l1_future, l2_future)?;
                self.metrics.lock().await.record_write_through();
                Ok(())
            }
            
            WritePolicy::WriteBack => {
                // Write to L1 immediately
                self.l1_cache.set_state(identifier, state.clone(), ttl).await?;
                
                // Queue for L2 sync
                self.queue_l2_write(identifier.to_string(), state, ttl).await;
                self.metrics.lock().await.record_write_back();
                Ok(())
            }
            
            WritePolicy::WriteBehind => {
                // Write to L1 and queue for L2
                self.l1_cache.set_state(identifier, state.clone(), ttl).await?;
                self.queue_l2_write_delayed(identifier.to_string(), state, ttl).await;
                self.metrics.lock().await.record_write_behind();
                Ok(())
            }
            
            WritePolicy::WriteAround => {
                // Write only to L2, invalidate L1
                self.l2_cache.set_state(identifier, state, ttl).await?;
                self.l1_cache.invalidate(identifier).await?;
                self.metrics.lock().await.record_write_around();
                Ok(())
            }
        }
    }
    
    async fn increment(&self, identifier: &str, amount: u64, window: Duration) -> Result<u64, StorageError> {
        match self.config.consistency_level {
            ConsistencyLevel::Eventual => {
                // Increment in L1, sync to L2 later
                let result = self.l1_cache.increment(identifier, amount, window).await?;
                self.queue_l2_sync(identifier.to_string()).await;
                Ok(result)
            }
            
            ConsistencyLevel::Strong => {
                // Increment in L2 first (source of truth)
                let result = self.l2_cache.increment(identifier, amount, window).await?;
                
                // Update L1 to maintain consistency
                if let Ok(Some(state)) = self.l2_cache.get_state(identifier).await {
                    let _ = self.l1_cache.set_state(identifier, state, Some(window)).await;
                }
                
                Ok(result)
            }
            
            ConsistencyLevel::Session => {
                // Use session-based consistency (implementation specific)
                // For now, use strong consistency
                self.increment_strong(identifier, amount, window).await
            }
        }
    }
}

impl HybridStorage {
    pub async fn new(config: HybridStorageConfig) -> Result<Self, StorageError> {
        let l1_cache = EnhancedMemoryStorage::new(config.l1_config.clone()).await?;
        let l2_cache = RedisStorage::new(config.l2_config.clone()).await?;
        
        let mut storage = Self {
            l1_cache,
            l2_cache,
            config: config.clone(),
            sync_handle: None,
            metrics: Arc::new(Mutex::new(HybridStorageMetrics::new())),
        };
        
        // Start background synchronization
        storage.start_sync_process().await;
        
        Ok(storage)
    }
    
    async fn start_sync_process(&mut self) {
        let l1_cache = self.l1_cache.clone();
        let l2_cache = self.l2_cache.clone();
        let sync_interval = self.config.sync_interval;
        let metrics = Arc::clone(&self.metrics);
        
        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(sync_interval);
            
            loop {
                interval.tick().await;
                
                // Sync dirty entries from L1 to L2
                if let Err(e) = Self::sync_l1_to_l2(&l1_cache, &l2_cache).await {
                    eprintln!("Hybrid storage sync error: {}", e);
                    metrics.lock().await.record_sync_error();
                } else {
                    metrics.lock().await.record_sync_success();
                }
            }
        });
        
        self.sync_handle = Some(handle);
    }
    
    async fn sync_l1_to_l2(
        l1_cache: &EnhancedMemoryStorage,
        l2_cache: &RedisStorage,
    ) -> Result<(), StorageError> {
        // Get dirty entries from L1
        let dirty_entries = l1_cache.get_dirty_entries().await?;
        
        // Batch sync to L2
        let mut batch_ops = Vec::new();
        for (identifier, state, ttl) in dirty_entries {
            batch_ops.push((identifier, state, ttl));
        }
        
        if !batch_ops.is_empty() {
            l2_cache.batch_set(&batch_ops).await?;
            l1_cache.mark_clean(&batch_ops.iter().map(|(id, _, _)| id.as_str()).collect::<Vec<_>>()).await?;
        }
        
        Ok(())
    }
}
```

**Advantages:**
- Best of both worlds: L1 speed + L2 persistence
- Configurable consistency levels
- Automatic failover and recovery
- Optimized for different access patterns

**Disadvantages:**
- Complex implementation and debugging
- Potential consistency issues
- Higher memory usage (dual storage)
- More failure modes to handle

## Storage Selection Guidelines

### Performance Requirements
```rust
pub struct StorageRecommendation;

impl StorageRecommendation {
    pub fn recommend_storage(
        requirements: &PerformanceRequirements,
        deployment: &DeploymentType,
        data_requirements: &DataRequirements,
    ) -> StorageConfig {
        match (requirements.latency_target, deployment, data_requirements.persistence) {
            // Ultra-low latency requirements
            (LatencyTarget::SubMillisecond, _, _) => {
                StorageConfig::Memory(MemoryStorageConfig {
                    max_entries: 1_000_000,
                    eviction_policy: EvictionPolicy::AdaptiveLRU,
                    memory_limit: Some(512 * 1024 * 1024), // 512MB
                    ..Default::default()
                })
            }
            
            // Single instance deployment
            (_, DeploymentType::SingleInstance, PersistenceLevel::None) => {
                StorageConfig::Memory(MemoryStorageConfig::default())
            }
            
            // Clustered deployment with persistence
            (_, DeploymentType::Clustered, PersistenceLevel::Session) => {
                StorageConfig::Hybrid(HybridStorageConfig {
                    write_policy: WritePolicy::WriteBack,
                    read_policy: ReadPolicy::CacheAside,
                    consistency_level: ConsistencyLevel::Eventual,
                    ..Default::default()
                })
            }
            
            // High availability with strong consistency
            (_, DeploymentType::HighAvailability, PersistenceLevel::Durable) => {
                StorageConfig::Postgres(PostgresStorageConfig {
                    partition_strategy: PartitionStrategy::ByTime(Duration::from_secs(86400)),
                    ..Default::default()
                })
            }
            
            // Default: Redis for distributed deployments
            _ => {
                StorageConfig::Redis(RedisStorageConfig {
                    cluster_mode: true,
                    compression: CompressionType::Snappy,
                    ..Default::default()
                })
            }
        }
    }
}
```

## Monitoring and Observability

### Storage Metrics
```rust
#[derive(Debug, Default)]
pub struct StorageMetrics {
    // Operation counts
    pub reads: AtomicU64,
    pub writes: AtomicU64,
    pub increments: AtomicU64,
    pub evictions: AtomicU64,
    
    // Latency histograms
    pub read_latency: Histogram,
    pub write_latency: Histogram,
    pub increment_latency: Histogram,
    
    // Cache statistics
    pub hit_rate: AtomicF64,
    pub miss_rate: AtomicF64,
    pub eviction_rate: AtomicF64,
    
    // Error counts
    pub connection_errors: AtomicU64,
    pub serialization_errors: AtomicU64,
    pub timeout_errors: AtomicU64,
}

impl StorageMetrics {
    pub fn export_prometheus_metrics(&self) -> Vec<prometheus::proto::MetricFamily> {
        let mut metrics = Vec::new();
        
        // Operation counters
        metrics.push(prometheus::counter_vec!(
            "rate_limit_storage_operations_total",
            "Total number of storage operations",
            &["operation", "storage_type"]
        ));
        
        // Latency histograms
        metrics.push(prometheus::histogram_vec!(
            "rate_limit_storage_operation_duration_seconds",
            "Duration of storage operations",
            &["operation", "storage_type"],
            prometheus::exponential_buckets(0.0001, 2.0, 15).unwrap()
        ));
        
        // Cache hit rates
        metrics.push(prometheus::gauge_vec!(
            "rate_limit_storage_hit_rate",
            "Storage cache hit rate",
            &["storage_type", "cache_level"]
        ));
        
        metrics
    }
}
```

---

This comprehensive storage backend system provides the foundation for enterprise-grade rate limiting with multiple storage options optimized for different deployment scenarios and performance requirements.