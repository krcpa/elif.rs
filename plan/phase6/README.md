# Phase 6: Advanced ORM & Relationships 🔗

**Duration**: 3-4 weeks  
**Goal**: Modern ORM experience with relationships  
**Status**: Ready after Phase 5

## Overview

Phase 6 completes our ORM system with full relationship support, eager loading, and advanced model features. This transforms our basic ORM foundation into a complete modern ORM system with all major relationship types and advanced querying capabilities.

## Dependencies

- **Phase 2.1**: ✅ Complete (ORM foundation, Query builder)
- **Phase 4**: ✅ Complete (Database operations, transactions)

## Key Components

### 1. Relationship System Core
**File**: `crates/elif-orm/src/relationships/mod.rs`

Complete relationship system supporting all major database relationship patterns.

**Requirements**:
- HasOne and HasMany relationships
- BelongsTo and BelongsToMany relationships  
- HasManyThrough relationships
- Polymorphic relationships (MorphTo, MorphMany)
- Self-referencing relationships
- Relationship constraints and cascading

**API Design**:
```rust
// Relationship definitions in models
#[derive(Model)]
#[model(table = "users")]
pub struct User {
    #[model(primary_key)]
    pub id: u64,
    pub name: String,
    pub email: String,
    
    // Relationships are defined as methods
}

impl User {
    // HasOne relationship
    pub fn profile(&self) -> HasOne<Profile> {
        HasOne::new(self, "user_id")
    }
    
    // HasMany relationship  
    pub fn posts(&self) -> HasMany<Post> {
        HasMany::new(self, "user_id")
    }
    
    // BelongsToMany relationship
    pub fn roles(&self) -> BelongsToMany<Role> {
        BelongsToMany::new(self, "user_roles", "user_id", "role_id")
    }
    
    // HasManyThrough relationship
    pub fn comments(&self) -> HasManyThrough<Comment> {
        HasManyThrough::new(self, Comment::table(), "user_id")
            .through(Post::table(), "user_id", "id")
    }
}

#[derive(Model)]
#[model(table = "posts")]  
pub struct Post {
    pub id: u64,
    pub user_id: u64,
    pub title: String,
    pub content: String,
}

impl Post {
    // BelongsTo relationship
    pub fn user(&self) -> BelongsTo<User> {
        BelongsTo::new(self, "user_id")
    }
    
    // HasMany relationship
    pub fn comments(&self) -> HasMany<Comment> {
        HasMany::new(self, "post_id")
    }
    
    // Polymorphic relationship
    pub fn tags(&self) -> MorphToMany<Tag> {
        MorphToMany::new(self, Tag::table(), "taggable")
    }
}
```

### 2. Eager Loading System
**File**: `crates/elif-orm/src/relationships/eager_loading.rs`

Efficient eager loading to prevent N+1 query problems.

**Requirements**:
- Eager loading with `with()` method
- Nested eager loading (e.g., `with("posts.comments.user")`)
- Conditional eager loading
- Eager loading with constraints
- Lazy loading fallback
- Load counting and optimization

**API Design**:
```rust
// Basic eager loading
let users = User::query()
    .with("profile")
    .with("posts")
    .limit(10)
    .get()
    .await?;

// Nested eager loading
let users = User::query()
    .with("posts.comments.user")
    .with("posts.tags")
    .get()
    .await?;

// Eager loading with constraints
let users = User::query()
    .with_where("posts", |query| {
        query.where_eq("published", true)
             .order_by_desc("created_at")
             .limit(5)
    })
    .get()
    .await?;

// Conditional eager loading
let users = User::query()
    .with_when(include_posts, "posts")
    .with_count("posts") // Load post count without loading posts
    .get()
    .await?;

// Usage with loaded relationships
for user in users {
    println!("User: {}", user.name);
    
    // Profile is already loaded (no additional query)
    if let Some(profile) = user.profile().get()? {
        println!("Bio: {}", profile.bio);
    }
    
    // Posts are already loaded
    for post in user.posts().get()? {
        println!("Post: {}", post.title);
        
        // Comments are loaded due to nested eager loading
        for comment in post.comments().get()? {
            println!("Comment: {}", comment.content);
        }
    }
}
```

### 3. Lazy Loading Implementation
**File**: `crates/elif-orm/src/relationships/lazy_loading.rs`

Lazy loading with transparent query execution when relationships are accessed.

**Requirements**:
- Transparent lazy loading on first access
- Caching of loaded relationships
- Lazy loading with constraints
- Relationship reloading
- Memory-efficient lazy loading

**API Design**:
```rust
// Lazy loading wrapper
pub struct Lazy<T> {
    value: Option<T>,
    loader: Box<dyn RelationshipLoader<T>>,
    loaded: bool,
}

impl<T> Lazy<T> {
    pub async fn get(&mut self) -> Result<&T, ModelError>;
    pub async fn load(&mut self) -> Result<&T, ModelError>;
    pub async fn reload(&mut self) -> Result<&T, ModelError>;
    pub fn is_loaded(&self) -> bool;
}

// Usage in models
#[derive(Model)]
pub struct User {
    pub id: u64,
    pub name: String,
    
    // Lazy-loaded relationships
    #[model(relationship = "HasMany")]
    pub posts: Lazy<Vec<Post>>,
    
    #[model(relationship = "HasOne")]  
    pub profile: Lazy<Option<Profile>>,
}

// Transparent loading
let user = User::find(pool, 1).await?;

// This triggers a database query to load posts
let posts = user.posts.get().await?;

// This doesn't trigger another query (cached)
let posts_again = user.posts.get().await?;
```

### 4. Polymorphic Relationships
**File**: `crates/elif-orm/src/relationships/polymorphic.rs`

Support for polymorphic relationships where a model can belong to multiple types.

**Requirements**:
- MorphTo relationships (belongs to multiple types)
- MorphMany relationships (has many of different types)
- MorphToMany relationships (many-to-many polymorphic)
- Polymorphic eager loading
- Type resolution and casting

**API Design**:
```rust
// Polymorphic models
#[derive(Model)]
pub struct Comment {
    pub id: u64,
    pub content: String,
    pub commentable_id: u64,
    pub commentable_type: String,
}

impl Comment {
    // MorphTo relationship
    pub fn commentable(&self) -> MorphTo {
        MorphTo::new(self, "commentable")
            .register_type::<Post>("Post")
            .register_type::<User>("User")
    }
}

#[derive(Model)]  
pub struct Tag {
    pub id: u64,
    pub name: String,
}

impl Tag {
    // MorphToMany relationship
    pub fn posts(&self) -> MorphedByMany<Post> {
        MorphedByMany::new(self, Post::table(), "taggable")
    }
    
    pub fn users(&self) -> MorphedByMany<User> {
        MorphedByMany::new(self, User::table(), "taggable")
    }
}

// Usage
let comment = Comment::find(pool, 1).await?;

// This resolves to either Post or User based on commentable_type
let commentable = comment.commentable().get().await?;

match commentable {
    Commentable::Post(post) => println!("Comment on post: {}", post.title),
    Commentable::User(user) => println!("Comment on user: {}", user.name),
}
```

### 5. Model Events and Observers
**File**: `crates/elif-orm/src/events.rs`

Complete event system for model lifecycle hooks with observer pattern.

**Requirements**:
- Model lifecycle events (creating, created, updating, updated, deleting, deleted, saving, saved)
- Observer pattern for event handling
- Global and model-specific observers
- Event propagation control (stopping events)
- Async event handlers with error handling

**API Design**:
```rust
// Model events enum
#[derive(Debug, Clone)]
pub enum ModelEvent<T> {
    Creating(T),
    Created(T),
    Updating(T, T), // (old, new)
    Updated(T, T),
    Saving(T),
    Saved(T),
    Deleting(T),
    Deleted(T),
}

// Observer trait
pub trait ModelObserver<T>: Send + Sync {
    async fn creating(&self, model: &mut T) -> Result<(), EventError> { Ok(()) }
    async fn created(&self, model: &T) -> Result<(), EventError> { Ok(()) }
    async fn updating(&self, model: &mut T, original: &T) -> Result<(), EventError> { Ok(()) }
    async fn updated(&self, model: &T, original: &T) -> Result<(), EventError> { Ok(()) }
    async fn saving(&self, model: &mut T) -> Result<(), EventError> { Ok(()) }
    async fn saved(&self, model: &T) -> Result<(), EventError> { Ok(()) }
    async fn deleting(&self, model: &T) -> Result<(), EventError> { Ok(()) }
    async fn deleted(&self, model: &T) -> Result<(), EventError> { Ok(()) }
}

// Example observer
pub struct UserObserver;

impl ModelObserver<User> for UserObserver {
    async fn creating(&self, user: &mut User) -> Result<(), EventError> {
        // Normalize email before creating
        user.email = user.email.to_lowercase();
        
        // Validate email uniqueness
        if User::query().where_eq("email", &user.email).exists().await? {
            return Err(EventError::validation("Email already exists"));
        }
        
        Ok(())
    }
    
    async fn created(&self, user: &User) -> Result<(), EventError> {
        // Send welcome email (async operation)
        EmailService::send_welcome_email(user).await?;
        
        // Create default user profile
        let profile = Profile {
            user_id: user.id,
            display_name: user.name.clone(),
            ..Default::default()
        };
        Profile::create(&self.pool, profile).await?;
        
        Ok(())
    }
    
    async fn updating(&self, user: &mut User, original: &User) -> Result<(), EventError> {
        // Log email changes for security
        if user.email != original.email {
            SecurityLog::log_email_change(original.id, &original.email, &user.email).await?;
        }
        
        Ok(())
    }
}

// Observer registration
User::observe(UserObserver);

// Global observer for all models
GlobalObserver::register(Box::new(AuditObserver::new()));
```

### 6. Advanced Query Features for Relationships
**File**: `crates/elif-orm/src/relationships/queries.rs`

Advanced querying capabilities specifically for relationships.

**Requirements**:
- Relationship existence queries (`whereHas`, `whereDoesntHave`)
- Relationship counting (`withCount`)
- Subquery relationships
- Relationship aggregations
- Complex relationship constraints

**API Design**:
```rust
// Querying based on relationship existence
let users_with_posts = User::query()
    .where_has("posts", |query| {
        query.where_eq("published", true)
             .where_gt("views", 1000)
    })
    .get()
    .await?;

// Users without any posts
let users_without_posts = User::query()
    .where_doesnt_have("posts")
    .get()
    .await?;

// Load relationship counts
let users = User::query()
    .with_count("posts")
    .with_count("comments")
    .with_count_where("published_posts", "posts", |query| {
        query.where_eq("published", true)
    })
    .get()
    .await?;

// Access counts
for user in users {
    println!("User {} has {} posts and {} comments", 
             user.name, user.posts_count, user.comments_count);
}

// Relationship aggregations
let users = User::query()
    .with_sum("posts", "views", "total_post_views")
    .with_avg("posts", "rating", "avg_post_rating")
    .with_max("posts", "created_at", "latest_post_date")
    .get()
    .await?;

// Complex relationship queries
let popular_users = User::query()
    .where_has("posts", |posts_query| {
        posts_query.where_has("comments", |comments_query| {
            comments_query.where_gt("rating", 4)
        }).having_count("comments", ">", 10)
    })
    .with("posts.comments.user")
    .order_by_desc("posts_count")
    .limit(20)
    .get()
    .await?;
```

## Implementation Plan

### Week 1: Basic Relationships
- [ ] HasOne and HasMany relationship implementations
- [ ] BelongsTo and BelongsToMany relationships
- [ ] Basic relationship loading (lazy loading foundation)
- [ ] Relationship query integration

### Week 2: Advanced Relationships & Eager Loading
- [ ] HasManyThrough relationships
- [ ] Polymorphic relationships (MorphTo, MorphMany)
- [ ] Eager loading system with nested loading
- [ ] Eager loading with constraints and conditions

### Week 3: Model Events & Advanced Features
- [ ] Complete model events system
- [ ] Observer pattern implementation
- [ ] Relationship existence queries (whereHas, etc.)
- [ ] Relationship counting and aggregations

### Week 4: Polish & Optimization
- [ ] Performance optimization for relationship loading
- [ ] Memory usage optimization for large datasets
- [ ] Comprehensive testing and benchmarks
- [ ] Documentation and examples

## Testing Strategy

### Unit Tests
- Individual relationship type functionality
- Eager loading query generation
- Model event triggering and handling
- Observer pattern behavior

### Integration Tests
- Complete relationship workflows
- Complex nested eager loading
- N+1 query prevention verification
- Model events in real scenarios

### Performance Tests
- Eager loading vs N+1 query benchmarks
- Memory usage with large relationship datasets
- Complex query performance
- Observer overhead measurement

## Success Criteria

### Functional Requirements
- [ ] All relationship types work correctly (HasOne, HasMany, BelongsTo, BelongsToMany, etc.)
- [ ] Eager loading prevents N+1 queries effectively
- [ ] Model events trigger at correct lifecycle points
- [ ] Polymorphic relationships resolve correct types

### Performance Requirements
- [ ] Eager loading reduces queries by >90% vs lazy loading
- [ ] Relationship loading <10ms for typical datasets
- [ ] Memory overhead <100 bytes per relationship
- [ ] Observer overhead <0.1ms per event

### API Completeness
- [ ] Modern ORM feature parity for relationships
- [ ] Intuitive and type-safe relationship definitions
- [ ] Flexible eager loading with constraints
- [ ] Comprehensive relationship querying capabilities

## Deliverables

1. **Complete Relationship System**:
   - All major relationship types implemented
   - Eager and lazy loading mechanisms
   - Polymorphic relationship support

2. **Model Events Framework**:
   - Full lifecycle events
   - Observer pattern implementation
   - Async event handling

3. **Advanced Querying**:
   - Relationship-based queries
   - Counting and aggregations
   - Complex nested queries

4. **Documentation & Examples**:
   - Relationship definition guide
   - Eager loading best practices
   - Model events and observers guide
   - Performance optimization tips

## Files Structure
```
crates/elif-orm/src/relationships/
├── mod.rs                  # Public relationship exports
├── has_one.rs              # HasOne relationship
├── has_many.rs             # HasMany relationship  
├── belongs_to.rs           # BelongsTo relationship
├── belongs_to_many.rs      # BelongsToMany relationship
├── has_many_through.rs     # HasManyThrough relationship
├── polymorphic.rs          # Polymorphic relationships
├── eager_loading.rs        # Eager loading implementation
├── lazy_loading.rs         # Lazy loading implementation
└── queries.rs              # Relationship querying

crates/elif-orm/src/
├── events.rs               # Model events system
├── observers.rs            # Observer pattern
└── relationship_loader.rs  # Relationship loading utilities

examples/blog-with-relationships/
├── src/
│   ├── models/
│   │   ├── user.rs         # User with relationships
│   │   ├── post.rs         # Post with relationships
│   │   ├── comment.rs      # Comment model
│   │   └── tag.rs          # Tag with polymorphic relations
│   └── observers/
│       ├── user_observer.rs
│       └── post_observer.rs
└── Cargo.toml
```

This phase completes the ORM system to provide modern ORM functionality while maintaining Rust's type safety and performance characteristics.