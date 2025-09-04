# Quickstart: Build a Blog API (No Rust Required)

Build a complete blog API in 10 minutes using only CLI commands - no Rust coding required! This guide shows the power of elif.rs's code generation and convention-over-configuration philosophy.

## What You'll Build

By the end of this tutorial, you'll have:
- ✅ A complete REST API with CRUD operations
- ✅ Database models with relationships  
- ✅ Automatic OpenAPI documentation
- ✅ Request validation and error handling
- ✅ Authentication middleware
- ✅ Comprehensive test suite
- ✅ Production-ready deployment configuration

**Time Required**: ~10 minutes  
**Difficulty**: Beginner  
**Prerequisites**: [Installation complete](installation.md)

## Step 1: Create the Project

```bash
# Create a new blog API project
elifrs new blog-api --template api
cd blog-api

# Verify the setup
elifrs check
```

**What happened?** elif.rs generated a complete project structure with sensible defaults, including database configuration, middleware setup, and basic routing.

## Step 2: Generate Your Models

Let's create a blog with users, posts, and comments:

### Create User Model
```bash
elifrs make resource User \
  --fields "name:string:required,email:string:required:unique,bio:text,avatar_url:string" \
  --auth \
  --api \
  --tests
```

### Create Post Model  
```bash
elifrs make resource Post \
  --fields "title:string:required,slug:string:required:unique,content:text:required,published:boolean:default=false,published_at:datetime" \
  --belongs-to User \
  --api \
  --tests
```

### Create Comment Model
```bash
elifrs make resource Comment \
  --fields "content:text:required,approved:boolean:default=false" \
  --belongs-to User \
  --belongs-to Post \
  --api \
  --tests
```

**What happened?** elif.rs generated:
- Database models with proper field types and constraints
- Database migrations with foreign key relationships  
- Controllers with full CRUD operations
- Request validation structs
- API routes with proper HTTP methods
- Comprehensive test suites
- Authentication middleware for users

## Step 3: Configure the Database

### Set up your database connection
Edit `.env`:
```bash
# Database (required)
DATABASE_URL=postgresql://username:password@localhost/blog_api_dev

# Optional: customize server settings
HOST=127.0.0.1
PORT=3000
RUST_ENV=development
```

### Create and migrate the database
```bash
# Create the database
createdb blog_api_dev

# Run all generated migrations
elifrs migrate run

# Check migration status
elifrs migrate status
```

**Expected output:**
```
✅ Running migration: 001_create_users_table
✅ Running migration: 002_create_posts_table  
✅ Running migration: 003_create_comments_table
✅ All migrations completed successfully
```

## Step 4: Add Sample Data (Optional)

Generate realistic test data:

```bash
# Create a data seeder
elifrs make seeder BlogSeeder

# Run the seeder to populate your database
elifrs db seed --seeder BlogSeeder
```

This creates:
- 10 sample users
- 25 blog posts with realistic content
- 100 comments across posts
- Proper relationships between all models

## Step 5: Start the Development Server

The generated project uses elif.rs's zero-boilerplate bootstrap setup:

```rust
// Generated main.rs - No manual server configuration needed!
use elif::prelude::*;

#[module(
    controllers: [UserController, PostController, CommentController],
    providers: [DatabaseService],
    is_app
)]
struct BlogApp;

#[elif::bootstrap(BlogApp)]
async fn main() -> Result<(), HttpError> {
    println!("🚀 Blog API starting...");
    // Server automatically starts with:
    // ✅ All controllers registered
    // ✅ Database connection configured  
    // ✅ Middleware pipeline active
    // ✅ API documentation available
}
```

Start the server:
```bash
# Start with hot reload - true Laravel-style development experience
elifrs serve --hot-reload --port 3000
```

Your API is now running at `http://127.0.0.1:3000`! 🚀

**What just happened?** The `#[elif::bootstrap]` macro automatically:
- ✅ Discovered all your controllers and registered their routes
- ✅ Set up dependency injection for all services  
- ✅ Applied authentication middleware where specified
- ✅ Started the HTTP server with proper error handling
- ✅ Enabled OpenAPI documentation generation

## Step 6: Test Your API

### Using curl
```bash
# List all posts
curl http://127.0.0.1:3000/api/posts

# Get a specific post  
curl http://127.0.0.1:3000/api/posts/1

# Create a new user
curl -X POST http://127.0.0.1:3000/api/users \
  -H "Content-Type: application/json" \
  -d '{"name": "John Doe", "email": "john@example.com", "bio": "Software developer"}'

# Create a new post (requires authentication)
curl -X POST http://127.0.0.1:3000/api/posts \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <your-jwt-token>" \
  -d '{"title": "My First Post", "content": "Hello, elif.rs world!", "published": true}'
```

### Available Endpoints

Your generated API includes these endpoints:

#### Users API
- `GET /api/users` - List all users
- `POST /api/users` - Create user & get JWT token
- `GET /api/users/{id}` - Get user details  
- `PUT /api/users/{id}` - Update user (auth required)
- `DELETE /api/users/{id}` - Delete user (auth required)

#### Posts API  
- `GET /api/posts` - List posts (supports pagination & filtering)
- `POST /api/posts` - Create post (auth required)
- `GET /api/posts/{id}` - Get post details
- `PUT /api/posts/{id}` - Update post (auth required)  
- `DELETE /api/posts/{id}` - Delete post (auth required)
- `GET /api/posts/slug/{slug}` - Get post by slug

#### Comments API
- `GET /api/posts/{post_id}/comments` - List post comments
- `POST /api/posts/{post_id}/comments` - Add comment (auth required)  
- `PUT /api/comments/{id}` - Update comment (auth required)
- `DELETE /api/comments/{id}` - Delete comment (auth required)

## Step 7: Generate API Documentation

```bash
# Generate OpenAPI specification
elifrs openapi generate --output openapi/blog-api.yml

# Start Swagger UI server
elifrs openapi serve --port 8080
```

Visit `http://127.0.0.1:8080` to see your interactive API documentation! 📚

**Features included:**
- Complete endpoint documentation
- Request/response schemas
- Authentication examples
- Try-it-out functionality
- Model relationships visualization

## Step 8: Run the Test Suite

```bash
# Run all tests
elifrs test

# Run tests for a specific resource
elifrs test --filter User

# Run with coverage report
elifrs test --coverage
```

**Expected output:**
```
✅ User model tests (12 passed)
✅ Post model tests (15 passed)  
✅ Comment model tests (8 passed)
✅ User API tests (18 passed)
✅ Post API tests (22 passed)
✅ Comment API tests (14 passed)
✅ Authentication tests (6 passed)

Total: 95 tests passed, 0 failed
Coverage: 94.2%
```

## Step 9: Add Authentication & Authorization

### Enable JWT Authentication
```bash
# Add JWT middleware to protected routes
elifrs make middleware JwtAuth --template auth

# Apply to specific routes
elifrs middleware apply JwtAuth --routes "posts:write,comments:write,users:update"
```

### Create API Keys (Optional)
```bash
# Generate API key system  
elifrs make auth api-keys --scopes "read,write,admin"
```

## Step 10: Production Preparation

### Environment Configuration
```bash
# Generate production configuration
elifrs config generate --env production

# Create Docker configuration
elifrs docker init --postgres --redis
```

### Build for Production
```bash
# Optimize for production
cargo build --release

# Run production server
./target/release/blog-api
```

## What You've Accomplished

🎉 **Congratulations!** You've built a production-ready blog API without writing a single line of Rust code. Your API includes:

### ✅ Core Features
- **Complete CRUD operations** for users, posts, and comments
- **Database relationships** with proper foreign key constraints
- **Authentication & authorization** with JWT tokens
- **Input validation** with comprehensive error messages
- **Pagination** for large datasets
- **Filtering & sorting** on all list endpoints

### ✅ Developer Experience  
- **Hot reload** for rapid development
- **Comprehensive test suite** with 95%+ coverage
- **Interactive API documentation** with Swagger UI
- **Database migrations** with rollback support
- **Seed data** for development and testing

### ✅ Production Ready
- **Error handling** with structured responses
- **Request logging** and observability  
- **Rate limiting** to prevent abuse
- **CORS configuration** for browser integration
- **Docker support** for containerized deployment
- **Environment-based configuration**

## Next Steps: Customize Your API

Now that you have a working API, you can customize it:

### Add Business Logic
```rust
// Example: Custom post publishing logic
impl PostController {
    #[put("/posts/{id}/publish")]
    #[param(id: int)]
    #[middleware("auth")]
    async fn publish_post(&self, id: i32) -> HttpResult<ElifResponse> {
        // Your custom publishing logic here
        let post = Post::find(id)?;
        post.update(json!({"published": true, "published_at": Utc::now()}))?;
        Ok(ElifResponse::ok().json(&post)?)
    }
}
```

### Add More Resources
```bash
# Add categories
elifrs make resource Category --fields "name:string:required:unique,description:text"

# Add tags with many-to-many relationship
elifrs make resource Tag --fields "name:string:required:unique"  
elifrs make migration create_post_tags_table --pivot Post Tag
```

### Advanced Features
```bash
# Add file uploads
elifrs make storage --provider s3

# Add email notifications  
elifrs make mail --template welcome

# Add background jobs
elifrs make queue --backend redis
```

## Learn More

Ready to dive deeper into elif.rs?

- **[Project Structure](project-structure.md)** - Understand the generated code
- **[Controllers](../basics/controllers.md)** - Learn declarative request handling  
- **[Database](../database/introduction.md)** - Master the ORM and query builder
- **[Authentication](../security/authentication.md)** - Implement advanced auth patterns
- **[Testing](../testing/introduction.md)** - Write comprehensive tests
- **[Deployment](../deployment/overview.md)** - Deploy to production

**Next**: [Project Structure →](project-structure.md)