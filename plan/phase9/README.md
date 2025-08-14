# Phase 9: Advanced Features ⭐

**Duration**: 4-6 weeks  
**Goal**: Complete framework ecosystem  
**Status**: Ready after Phase 8

## Overview

Phase 9 completes the framework ecosystem with advanced features that provide a complete modern web framework experience. This includes database seeding and factories, file storage and upload handling, email system integration, WebSocket support, and additional ecosystem features.

## Dependencies

- **Phase 6**: ✅ Complete ORM with relationships
- **Phase 8**: ✅ Production features (caching, queues)

## Key Components

### 1. Database Seeding & Factory System
**File**: `crates/elif-database/src/factories.rs`

Complete data factory system for testing and database seeding.

**Requirements**:
- Model factories with relationships
- Realistic fake data generation
- Seeding system with environment controls
- Factory states and traits
- Large dataset generation with performance optimization
- Data consistency and referential integrity

**API Design**:
```rust
// Advanced factory definitions
#[factory]
pub struct UserFactory {
    #[field(fake = "Name()")]
    pub name: String,
    
    #[field(fake = "Email()")]
    pub email: String,
    
    #[field(fake = "Internet.password(8, 20)")]
    pub password: String,
    
    #[field(default = "Utc::now()")]
    pub created_at: DateTime<Utc>,
    
    #[field(default = "Utc::now()")]  
    pub updated_at: DateTime<Utc>,
}

impl UserFactory {
    // Factory states
    pub fn admin(mut self) -> Self {
        self.email = format!("admin+{}@example.com", Uuid::new_v4());
        self
    }
    
    pub fn verified(mut self) -> Self {
        self.email_verified_at = Some(Utc::now());
        self
    }
    
    // Factory relationships
    pub fn with_posts(self, count: usize) -> Self {
        self.has_many(PostFactory::new(), count, "user_id")
    }
    
    pub fn with_profile(self) -> Self {
        self.has_one(ProfileFactory::new(), "user_id")
    }
}

// Factory usage
let users = UserFactory::new()
    .admin()
    .verified()
    .with_posts(5)
    .with_profile()
    .count(100)
    .create()
    .await?;

// Seeding system
#[seeder]
pub struct UserSeeder {
    count: usize,
}

impl Seeder for UserSeeder {
    async fn run(&self) -> Result<(), SeedError> {
        // Create admin users
        UserFactory::new()
            .admin()
            .count(5)
            .create()
            .await?;
            
        // Create regular users with content
        UserFactory::new()
            .verified()
            .with_posts(rand(1..10))
            .count(self.count)
            .create()
            .await?;
            
        Ok(())
    }
    
    fn environments(&self) -> Vec<Environment> {
        vec![Environment::Development, Environment::Testing]
    }
}

// CLI commands
elifrs db:seed                          # Run all seeders
elifrs db:seed --class UserSeeder       # Run specific seeder
elifrs factory User 50                  # Generate 50 users
elifrs factory User --admin --with-posts=5  # Generate admin users with posts
```

### 2. File Storage & Upload System
**File**: `crates/elif-storage/src/lib.rs`

Comprehensive file storage system with multiple backends and upload handling.

**Requirements**:
- Multiple storage backends (local, S3, Google Cloud, etc.)
- File upload handling with validation
- Image processing and optimization
- File serving with CDN integration
- Temporary file management
- File access control and signed URLs

**API Design**:
```rust
// Storage configuration
#[derive(Config)]
pub struct StorageConfig {
    #[config(env = "FILESYSTEM_DISK", default = "local")]
    pub default: String,
    
    #[config(nested)]
    pub local: LocalStorageConfig,
    
    #[config(nested)]
    pub s3: S3StorageConfig,
}

// File upload handling
#[derive(Deserialize, Validate)]
pub struct FileUploadRequest {
    #[validate(file_size(max = "10MB"))]
    #[validate(file_type(allowed = ["image/jpeg", "image/png", "application/pdf"]))]
    pub file: UploadedFile,
    
    #[validate(length(max = 255))]
    pub description: Option<String>,
}

impl FileController {
    async fn upload(&self, mut request: Request) -> Result<Response, HttpError> {
        let upload: FileUploadRequest = request.validate_files()?;
        
        // Process image if needed
        let processed_file = if upload.file.is_image() {
            ImageProcessor::new()
                .resize(800, 600)
                .optimize(0.85)
                .process(upload.file)
                .await?
        } else {
            upload.file
        };
        
        // Store file
        let path = Storage::disk("uploads")
            .put_file_as(&processed_file, "images", None)
            .await?;
            
        // Create database record
        let file_record = StoredFile::create(FileData {
            filename: processed_file.original_name(),
            path: path.clone(),
            mime_type: processed_file.mime_type(),
            size: processed_file.size(),
            uploader_id: request.user()?.id,
        }).await?;
        
        Ok(Response::json(FileResource::new(file_record)))
    }
    
    async fn download(&self, request: Request) -> Result<Response, HttpError> {
        let file_id: u64 = request.param("id")?.parse()?;
        let file = StoredFile::find(file_id).await?;
        
        // Check access permissions
        if !request.user()?.can_access_file(&file)? {
            return Ok(Response::forbidden());
        }
        
        // Generate signed URL for private files
        if file.is_private() {
            let signed_url = Storage::disk("private")
                .temporary_url(&file.path, Duration::from_hours(1))
                .await?;
            return Ok(Response::redirect(signed_url));
        }
        
        // Serve file directly
        let file_content = Storage::disk("public").get(&file.path).await?;
        Ok(Response::file(file_content, file.mime_type))
    }
}

// Image processing
ImageProcessor::new()
    .resize(800, 600)
    .crop(400, 400, CropMode::Center)
    .watermark("logo.png", Position::BottomRight)
    .format(ImageFormat::WebP)
    .quality(85)
    .process(image_file)
    .await?;
```

### 3. Email System Integration
**File**: `crates/elif-mail/src/lib.rs`

Complete email system with templates, queuing, and multiple providers.

**Requirements**:
- Multiple email providers (SMTP, SendGrid, Mailgun, etc.)
- Email templating with layouts
- HTML and text email generation
- Email queuing and background sending
- Email tracking and analytics
- Transactional and marketing email support

**API Design**:
```rust
// Email configuration
#[derive(Config)]
pub struct MailConfig {
    #[config(env = "MAIL_DRIVER", default = "smtp")]
    pub driver: String,
    
    #[config(env = "MAIL_FROM_ADDRESS")]
    pub from_address: String,
    
    #[config(env = "MAIL_FROM_NAME")]
    pub from_name: String,
    
    #[config(nested)]
    pub smtp: SmtpConfig,
    
    #[config(nested)]
    pub sendgrid: SendGridConfig,
}

// Email template system
#[derive(Template)]
#[template(path = "emails/welcome.html")]
pub struct WelcomeEmail {
    pub user_name: String,
    pub verification_url: String,
    pub company_name: String,
}

impl Mailable for WelcomeEmail {
    fn build(&self) -> MailBuilder {
        Mail::new()
            .to(&self.user.email, &self.user.name)
            .subject("Welcome to our platform!")
            .template(self)
            .text_template("emails/welcome.txt")
            .attach_data(self.get_welcome_pdf(), "welcome.pdf", "application/pdf")
    }
}

// Sending emails
WelcomeEmail {
    user_name: user.name.clone(),
    verification_url: generate_verification_url(&user),
    company_name: config.app.name.clone(),
}
.send()           // Send immediately
.queue()          // Queue for background sending
.delay(Duration::from_mins(5))  // Delayed sending
.await?;

// Email jobs
#[derive(Job)]
pub struct SendNewsletterJob {
    pub newsletter_id: u64,
    pub recipient_ids: Vec<u64>,
}

impl JobHandler for SendNewsletterJob {
    async fn handle(&self) -> Result<(), JobError> {
        let newsletter = Newsletter::find(self.newsletter_id).await?;
        let recipients = User::find_many(&self.recipient_ids).await?;
        
        for recipient in recipients {
            NewsletterEmail {
                newsletter: newsletter.clone(),
                recipient,
            }
            .queue()
            .await?;
        }
        
        Ok(())
    }
}
```

### 4. WebSocket & Real-time Features
**File**: `crates/elif-websocket/src/lib.rs`

WebSocket support for real-time features like chat, live updates, and notifications.

**Requirements**:
- WebSocket server integration
- Channel-based messaging (rooms, private channels)
- Real-time event broadcasting
- Authentication for WebSocket connections
- Presence tracking and user lists
- Integration with existing authorization system

**API Design**:
```rust
// WebSocket configuration
#[derive(Config)]
pub struct WebSocketConfig {
    #[config(default = "/ws")]
    pub endpoint: String,
    
    #[config(default = 1000)]
    pub max_connections: usize,
    
    #[config(default = "30s")]
    pub ping_interval: Duration,
}

// Channel definitions
#[channel("chat.{room_id}")]
pub struct ChatChannel {
    pub room_id: u64,
}

impl ChannelHandler for ChatChannel {
    async fn join(&self, user: &AuthUser) -> Result<(), ChannelError> {
        // Check if user can join this chat room
        if !user.can_join_chat_room(self.room_id).await? {
            return Err(ChannelError::Unauthorized);
        }
        
        // Broadcast user joined event
        self.broadcast(UserJoinedEvent {
            user_id: user.id,
            user_name: user.name.clone(),
        }).await?;
        
        Ok(())
    }
    
    async fn leave(&self, user: &AuthUser) -> Result<(), ChannelError> {
        self.broadcast(UserLeftEvent {
            user_id: user.id,
        }).await?;
        
        Ok(())
    }
    
    async fn handle_message(&self, user: &AuthUser, message: Value) -> Result<(), ChannelError> {
        let chat_message: ChatMessage = serde_json::from_value(message)?;
        
        // Save message to database
        let saved_message = Message::create(MessageData {
            room_id: self.room_id,
            user_id: user.id,
            content: chat_message.content,
            ..Default::default()
        }).await?;
        
        // Broadcast to all channel members
        self.broadcast(NewMessageEvent {
            message: saved_message,
        }).await?;
        
        Ok(())
    }
}

// Broadcasting events
#[event]
pub struct PostPublishedEvent {
    pub post: Post,
    pub author: User,
}

// Broadcast to specific users
PostPublishedEvent { post, author }
    .to_users(vec![user1.id, user2.id])
    .broadcast()
    .await?;

// Broadcast to channel
PostPublishedEvent { post, author }
    .to_channel("blog.updates")
    .broadcast()
    .await?;

// Client-side integration (JavaScript example)
const socket = new ElifWebSocket('/ws', {
    auth: { token: authToken }
});

socket.channel('chat.1')
    .join()
    .listen('NewMessage', (event) => {
        displayMessage(event.message);
    })
    .listen('UserJoined', (event) => {
        showUserJoined(event.user_name);
    });

socket.channel('notifications.private.' + userId)
    .listen('NotificationReceived', (notification) => {
        showNotification(notification);
    });
```

### 5. API Rate Limiting & Throttling
**File**: `crates/elif-throttle/src/lib.rs`

Advanced rate limiting system with multiple strategies and backends.

**Requirements**:
- Multiple rate limiting algorithms (token bucket, sliding window, etc.)
- Per-user, per-IP, and global rate limiting
- Rate limit headers and proper HTTP responses
- Integration with caching backends
- Rate limit bypass for trusted sources
- Dynamic rate limit adjustment

**API Design**:
```rust
// Rate limiting middleware
RateLimitMiddleware::new()
    .requests_per_minute(60)
    .per_user()
    .with_redis("redis://localhost")
    .skip_on_header("X-Admin-Key")
    .custom_key_fn(|req| {
        format!("api:{}:{}", req.user()?.subscription_tier, req.user_id())
    });

// Route-specific rate limiting
Route::post("/api/uploads", FileController::upload)
    .middleware(RateLimit::requests_per_hour(10).per_user());

Route::post("/api/auth/login", AuthController::login)
    .middleware(RateLimit::requests_per_minute(5).per_ip());

// Dynamic rate limits based on user tier
impl RateLimitResolver for UserTierResolver {
    async fn resolve_limit(&self, request: &Request) -> Result<RateLimit, RateLimitError> {
        let user = request.user()?;
        
        match user.subscription_tier {
            SubscriptionTier::Free => Ok(RateLimit::requests_per_hour(100)),
            SubscriptionTier::Pro => Ok(RateLimit::requests_per_hour(1000)),
            SubscriptionTier::Enterprise => Ok(RateLimit::unlimited()),
        }
    }
}
```

### 6. Event System & Event Sourcing
**File**: `crates/elif-events/src/lib.rs`

Event-driven architecture support with event sourcing capabilities.

**Requirements**:
- Event definition and dispatching
- Event listeners and handlers
- Event store for event sourcing
- Event replay and projection building
- Async event handling with error recovery
- Event versioning and schema evolution

**API Design**:
```rust
// Event definitions
#[derive(Event, Serialize, Deserialize)]
pub struct UserRegisteredEvent {
    pub user_id: u64,
    pub email: String,
    pub registration_time: DateTime<Utc>,
}

#[derive(Event, Serialize, Deserialize)]
pub struct OrderPlacedEvent {
    pub order_id: u64,
    pub user_id: u64,
    pub total: Decimal,
    pub items: Vec<OrderItem>,
}

// Event listeners
#[listener(event = "UserRegisteredEvent")]
pub struct SendWelcomeEmailListener;

impl EventListener<UserRegisteredEvent> for SendWelcomeEmailListener {
    async fn handle(&self, event: &UserRegisteredEvent) -> Result<(), EventError> {
        WelcomeEmail::for_user(event.user_id)
            .queue()
            .await?;
        
        Ok(())
    }
}

// Event dispatching
UserRegisteredEvent {
    user_id: user.id,
    email: user.email.clone(),
    registration_time: Utc::now(),
}
.dispatch()
.await?;

// Event sourcing
#[derive(Aggregate)]
pub struct OrderAggregate {
    id: u64,
    status: OrderStatus,
    total: Decimal,
    items: Vec<OrderItem>,
}

impl EventSourced for OrderAggregate {
    fn apply_event(&mut self, event: &Event) -> Result<(), AggregateError> {
        match event.event_type.as_str() {
            "OrderPlaced" => {
                let data: OrderPlacedEvent = event.deserialize_data()?;
                self.status = OrderStatus::Pending;
                self.total = data.total;
                self.items = data.items;
            },
            "OrderShipped" => {
                self.status = OrderStatus::Shipped;
            },
            _ => {}
        }
        Ok(())
    }
}
```

## Implementation Plan

### Week 1: Database Factories & Seeding
- [ ] Advanced factory system with relationships
- [ ] Fake data generation integration
- [ ] Comprehensive seeding system
- [ ] CLI commands for data generation

### Week 2: File Storage & Upload System
- [ ] Multi-backend storage system
- [ ] File upload handling and validation
- [ ] Image processing capabilities
- [ ] CDN integration and signed URLs

### Week 3: Email & Communication
- [ ] Email system with multiple providers
- [ ] Template system with layouts
- [ ] Email queuing and background sending
- [ ] Email tracking and analytics

### Week 4: Real-time Features
- [ ] WebSocket server integration
- [ ] Channel-based messaging system
- [ ] Real-time event broadcasting
- [ ] Presence tracking and user lists

### Week 5: Advanced API Features
- [ ] Advanced rate limiting system
- [ ] Event system and event sourcing
- [ ] API versioning support
- [ ] Advanced middleware components

### Week 6: Integration & Ecosystem
- [ ] Integration testing across all features
- [ ] Performance optimization and benchmarking
- [ ] Comprehensive documentation
- [ ] Example applications showcasing all features

## Testing Strategy

### Unit Tests
- Factory data generation accuracy
- File storage backend implementations
- Email template rendering
- WebSocket message handling
- Event dispatching and handling

### Integration Tests
- End-to-end file upload workflows
- Email sending through various providers
- Real-time messaging scenarios
- Event sourcing and projection building

### Performance Tests
- Large dataset generation with factories
- File upload and processing performance
- WebSocket connection scaling
- Event processing throughput

## Success Criteria

### Feature Completeness
- [ ] Modern database seeding and factories
- [ ] Production-ready file storage system
- [ ] Comprehensive email capabilities
- [ ] Real-time WebSocket features
- [ ] Advanced API rate limiting

### Performance Requirements
- [ ] Factory generates 10k+ records per minute
- [ ] File uploads handle 100MB+ files efficiently
- [ ] Email system processes 1000+ emails/minute
- [ ] WebSocket supports 1000+ concurrent connections

### Integration Requirements
- [ ] All features integrate seamlessly
- [ ] Consistent API patterns across features
- [ ] Comprehensive error handling
- [ ] Production-ready configuration options

## Deliverables

1. **Database Ecosystem**:
   - Advanced factory system with relationships
   - Comprehensive seeding capabilities
   - Fake data generation integration

2. **File & Communication Systems**:
   - Multi-backend file storage
   - Image processing capabilities
   - Complete email system with templates

3. **Real-time & Advanced Features**:
   - WebSocket support with channels
   - Event-driven architecture
   - Advanced rate limiting

4. **Complete Framework Ecosystem**:
   - Integration documentation
   - Best practices guides
   - Example applications

## Files Structure
```
crates/elif-database/src/
├── factories/              # Factory system
│   ├── mod.rs             # Factory core
│   ├── traits.rs          # Factory traits
│   ├── relationships.rs   # Relationship factories
│   └── fake_data.rs       # Fake data generation
├── seeding/               # Seeding system
│   ├── mod.rs            # Seeder core
│   ├── runner.rs         # Seed runner
│   └── environment.rs    # Environment handling

crates/elif-storage/src/
├── backends/              # Storage backends
│   ├── local.rs          # Local filesystem
│   ├── s3.rs             # AWS S3
│   └── gcs.rs            # Google Cloud Storage
├── processing.rs          # File processing
└── uploads.rs             # Upload handling

crates/elif-mail/src/
├── providers/             # Email providers
│   ├── smtp.rs           # SMTP provider
│   ├── sendgrid.rs       # SendGrid provider
│   └── mailgun.rs        # Mailgun provider
├── templates.rs           # Template system
└── queue.rs               # Email queuing

crates/elif-websocket/src/
├── channels.rs            # Channel system
├── authentication.rs     # WebSocket auth
├── broadcasting.rs       # Event broadcasting
└── presence.rs           # Presence tracking

examples/complete-app/     # Comprehensive example app
├── src/
│   ├── models/           # All model types
│   ├── controllers/      # All controller patterns
│   ├── channels/         # WebSocket channels
│   ├── jobs/            # Background jobs
│   └── events/          # Event handlers
├── factories/           # Factory definitions
├── seeders/             # Database seeders
└── templates/           # Email templates
```

This phase completes the elif.rs framework ecosystem, providing a comprehensive set of features that rival modern web frameworks while maintaining Rust's performance and safety characteristics. The framework is now ready for building complex, production-ready applications with rich features and excellent developer experience.