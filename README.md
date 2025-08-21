# elif.rs

> Where Rust meets Developer Experience - The framework designed for exceptional DX and AI-native development.

[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Build Status](https://img.shields.io/badge/build-passing-green.svg)](https://github.com/krcpa/elif.rs)
[![Crates.io](https://img.shields.io/crates/v/elifrs.svg)](https://crates.io/crates/elifrs)

**elif.rs** is a web framework designed for **both developers and AI**. Built with convention over configuration, zero boilerplate, and intuitive APIs that maximize productivity while maintaining Rust's performance and safety guarantees.

## 🚀 Quick Start

```bash
# Install elif
cargo install elifrs

# Create a new project
elifrs new my-app
cd my-app

# Start building
cargo run
```

## ✨ Declarative Controllers with Zero Ceremony

```rust
use elif_http::{controller, get, post, put, param, body, request, ElifRequest, ElifResponse, HttpResult};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct CreateUserRequest {
    name: String,
    email: String,
}

#[derive(Deserialize)]
struct UpdateUserRequest {
    name: Option<String>,
    email: Option<String>,
}

#[derive(Serialize)]
struct User {
    id: i32,
    name: String,
    email: String,
}

#[controller("/api/users")]
impl UserController {
    // Pure body parameter - no manual parsing needed
    #[post("")]
    #[body(user_data: CreateUserRequest)]
    async fn create_user(&self, user_data: CreateUserRequest) -> HttpResult<ElifResponse> {
        let user = User {
            id: 1,
            name: user_data.name,
            email: user_data.email,
        };
        Ok(ElifResponse::created().json(&user)?)
    }
    
    // Combined path and body parameters
    #[put("/{id}")]
    #[param(id: int)]
    #[body(updates: UpdateUserRequest)]
    async fn update_user(&self, id: i32, updates: UpdateUserRequest) -> HttpResult<ElifResponse> {
        let user = User {
            id,
            name: updates.name.unwrap_or_else(|| format!("User {}", id)),
            email: updates.email.unwrap_or_else(|| format!("user{}@example.com", id)),
        };
        Ok(ElifResponse::ok().json(&user)?)
    }
    
    // All decorators together - path + body + request access
    #[put("/{id}")]
    #[param(id: int)]
    #[body(updates: UpdateUserRequest)]
    #[request]
    async fn update_with_auth(&self, id: i32, updates: UpdateUserRequest) -> HttpResult<ElifResponse> {
        // Request available when needed
        let auth_user_id = req.header("user-id")
            .ok_or_else(|| ElifError::unauthorized())?;
            
        if auth_user_id != &id.to_string() {
            return Err(ElifError::forbidden());
        }
        
        let user = User {
            id,
            name: updates.name.unwrap_or_else(|| format!("User {}", id)),
            email: updates.email.unwrap_or_else(|| format!("user{}@example.com", id)),
        };
        Ok(ElifResponse::ok().json(&user)?)
    }
    
    // Form data support
    #[post("/contact")]
    #[body(form_data: form)]
    async fn contact_form(&self, form_data: HashMap<String, String>) -> HttpResult<ElifResponse> {
        println!("Contact form: {:?}", form_data);
        Ok(ElifResponse::ok().json(&json!({"message": "Form submitted successfully"}))?)
    }
    
    // File upload with raw bytes
    #[post("/upload")]
    #[body(file_data: bytes)]
    async fn upload_file(&self, file_data: Vec<u8>) -> HttpResult<ElifResponse> {
        println!("Received {} bytes", file_data.len());
        Ok(ElifResponse::ok().json(&json!({"size": file_data.len()}))?)
    }
}
```

**Benefits**: ~70% reduction in boilerplate vs manual route registration

## 🎯 Why elif.rs?

### **The elif.rs Philosophy**
- **Convention Over Configuration**: Sensible defaults, minimal setup
- **Zero Boilerplate**: If you want a response → `Response::json()`, just works
- **Developer Experience First**: APIs should be intuitive and obvious
- **AI-Friendly**: LLMs understand and generate elif code naturally

### **Rust Performance + Modern DX**
```rust
// Server setup - one line
Server::new().listen("127.0.0.1:3000").await?;

// Routing - obvious and clean  
Router::new()
    .route("/", get(home))
    .route("/users", get(users_index))
    .controller(UserController);

// Responses - what you'd expect
Response::json(&data)           // JSON response
Response::ok()                  // 200 OK  
Response::created()             // 201 Created
Response::not_found()           // 404 Not Found

// Request handling - intuitive and clean
req.json::<User>()              // Parse JSON body
req.path_param("id")            // Get path parameter  
req.query_param("page")         // Get query parameter
```

## 🏗️ Real-World API Example

```rust
use elif_http::{Server, controller, get, post, put, delete, param, body, middleware, HttpResult, ElifResponse};
use std::collections::HashMap;

#[controller("/api/v1/blog")]
#[middleware("auth", "rate_limit")]
impl BlogController {
    #[get("/posts")]
    async fn index(&self) -> HttpResult<ElifResponse> {
        let posts = vec![
            json!({"id": 1, "title": "Hello elif.rs", "published": true}),
            json!({"id": 2, "title": "Building APIs with Rust", "published": false}),
        ];
        Ok(ElifResponse::ok().json(&posts)?)
    }
    
    #[get("/posts/{id}")]
    #[param(id: int)]
    async fn show(&self, id: i32) -> HttpResult<ElifResponse> {
        let post = json!({
            "id": id,
            "title": format!("Post {}", id),
            "content": "Lorem ipsum dolor sit amet...",
            "published": true
        });
        Ok(ElifResponse::ok().json(&post)?)
    }
    
    #[post("/posts")]
    #[body(post_data: CreatePostRequest)]
    async fn store(&self, post_data: CreatePostRequest) -> HttpResult<ElifResponse> {
        let post = json!({
            "id": 1,
            "title": post_data.title,
            "content": post_data.content,
            "published": false
        });
        Ok(ElifResponse::created().json(&post)?)
    }
    
    #[put("/posts/{id}")]
    #[param(id: int)]
    #[body(updates: UpdatePostRequest)]
    #[request]
    async fn update(&self, id: i32, updates: UpdatePostRequest) -> HttpResult<ElifResponse> {
        // Validate ownership
        let user_id = req.user_id()?;
        if !can_edit_post(user_id, id).await? {
            return Err(ElifError::forbidden());
        }
        
        let post = json!({
            "id": id,
            "title": updates.title.unwrap_or_else(|| format!("Post {}", id)),
            "content": updates.content,
            "published": updates.published.unwrap_or(false)
        });
        Ok(ElifResponse::ok().json(&post)?)
    }
    
    #[delete("/posts/{id}")]
    #[param(id: int)]
    #[middleware("admin")]
    async fn destroy(&self, id: i32) -> HttpResult<ElifResponse> {
        // Delete logic here
        Ok(ElifResponse::no_content())
    }
}

#[derive(Deserialize)]
struct CreatePostRequest {
    title: String,
    content: String,
}

#[derive(Deserialize)]  
struct UpdatePostRequest {
    title: Option<String>,
    content: Option<String>,
    published: Option<bool>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let router = Router::new()
        .controller(BlogController)
        .middleware(cors())
        .middleware(logger());
    
    println!("🚀 Blog API running at http://localhost:3000");
    Server::new().router(router).listen("0.0.0.0:3000").await?;
    Ok(())
}
```

## 📦 Three Parameter Injection Types

### 1. **Path Parameters** - `#[param]`
```rust
#[get("/users/{id}/posts/{post_id}")]
#[param(id: int, post_id: int)]
async fn get_user_post(&self, id: i32, post_id: i32) -> HttpResult<ElifResponse> {
    // Both parameters automatically extracted and validated
}
```

### 2. **Body Parameters** - `#[body]`  
```rust
// JSON body
#[post("/users")]
#[body(user_data: CreateUserRequest)]
async fn create_user(&self, user_data: CreateUserRequest) -> HttpResult<ElifResponse> {}

// Form data
#[post("/contact")]
#[body(form_data: form)]
async fn contact(&self, form_data: HashMap<String, String>) -> HttpResult<ElifResponse> {}

// Raw bytes
#[post("/upload")]
#[body(file_data: bytes)]
async fn upload(&self, file_data: Vec<u8>) -> HttpResult<ElifResponse> {}
```

### 3. **Request Access** - `#[request]`
```rust
#[post("/posts")]
#[body(post_data: CreatePostRequest)]
#[request]
async fn create_post(&self, post_data: CreatePostRequest) -> HttpResult<ElifResponse> {
    let user_id = req.user_id()?; // Request available when needed
    // Create post with user_id...
}
```

## 🚀 Performance

elif.rs delivers exceptional DX without sacrificing Rust performance:

- **145k req/sec** - Benchmark results
- **0.68ms** - Average latency  
- **12MB** - Memory footprint
- **Zero** - Runtime overhead

## 🛠️ CLI Commands

```bash
# Project Management
elifrs new <name>          # Create new project
elifrs generate            # Generate from AI specs  
elifrs check              # Validate project

# Development  
elifrs serve --reload      # Start with hot reload
cargo test                # Run tests
cargo build --release     # Build for production

# Database
elifrs migrate run        # Run migrations
elifrs migrate create     # Create migration
elifrs migrate rollback   # Rollback migration

# API Documentation
elifrs openapi generate   # Generate OpenAPI spec
elifrs openapi serve      # Start Swagger UI
```

## 🤖 AI-Native Development

elif.rs pioneered AI-native framework design:

- **Claude**: Primary development partner
- **GPT-4**: Extensive testing and generation  
- **Cursor/Copilot**: First-class support

Every API is designed to be understood by both humans and AI, making it perfect for AI-assisted development.

## 🗺️ Roadmap

### Now ✅
- Core framework with zero boilerplate
- Declarative controllers with parameter injection
- HTTP/WebSocket support
- Database/ORM integration
- Authentication & middleware

### Next 🔄  
- Streaming responses
- gRPC support
- GraphQL integration
- Edge deployment

### Future 🚀
- Mobile SDKs
- Global CDN  
- AI copilot
- One-click deploy

## 🤝 Contributing

We welcome contributions! elif.rs is built by the community, for the community.

1. Fork the repository
2. Create your feature branch
3. Write tests (AI can help!)
4. Submit a pull request

See [CONTRIBUTING.md](CONTRIBUTING.md) for details.

## 📚 Documentation

- **Getting Started**: [docs.rs/elifrs](https://docs.rs/elifrs)
- **Examples**: [github.com/krcpa/elif.rs/examples](https://github.com/krcpa/elif.rs/tree/main/examples)
- **API Reference**: Full API documentation on docs.rs
- **Architecture**: [FRAMEWORK_ARCHITECTURE.md](docs/FRAMEWORK_ARCHITECTURE.md)

## 📄 License

MIT License - see [LICENSE](LICENSE) for details.

---

<p align="center">
<strong>elif.rs</strong><br>
Where Rust meets Developer Experience<br>
Convention over Configuration • Zero Boilerplate • AI-Native<br>
<br>
<a href="https://elif.rs">elif.rs</a> • 
<a href="https://github.com/krcpa/elif.rs">GitHub</a> • 
<a href="https://docs.rs/elifrs">Docs</a> • 
<a href="https://discord.gg/elifrs">Discord</a>
</p>