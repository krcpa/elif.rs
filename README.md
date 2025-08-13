# elif.rs

> LLM-friendly Rust web framework designed for AI agent-driven development

[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

**elif.rs** is a spec-first, AI-agent-optimized web framework built on Axum and sqlx. It enables AI agents (like Claude) to build complex web applications through safe, structured code generation with MARKER-based editing zones.

## 🎯 Why elif.rs?

Traditional web frameworks are designed for human developers. **elif.rs** is specifically designed for AI agents:

- **🤖 AI-Safe Editing**: MARKER blocks prevent framework corruption
- **📝 Spec-First**: Single YAML file drives everything  
- **⚡ Rapid Scaffolding**: Complete apps in minutes, not hours
- **🔧 Laravel-like DX**: Familiar commands, AI-optimized workflows
- **🔍 Introspective**: Built-in project understanding via APIs

## 🚀 Quick Start

### 1. Install elif CLI

```bash
git clone https://github.com/yourusername/elif.rs
cd elif.rs
cargo install --path crates/cli
```

### 2. Create Your First App

```bash
# Create a new application
elif new my-todo-app
cd ../my-todo-app

# Add some routes
elif route add GET /todos list_todos
elif route add POST /todos create_todo
elif route add GET /health health_check

# Add a model
elif model add Todo title:string completed:boolean priority:int

# Run the server  
cargo run
```

Your API is now running at `http://localhost:3000` with:
- Swagger UI at `/docs`
- Project introspection at `/_map.json`
- Health check at `/health`

## 🏗️ Architecture

```
my-app/
├── src/
│   ├── controllers/     # HTTP request handlers (AI-editable MARKER blocks)
│   ├── models/          # Database models (generated from specs)
│   ├── routes/          # Route definitions (auto-generated)
│   └── main.rs          # Server entry point
├── migrations/          # SQL migrations (auto-generated)
├── resources/           # Resource specifications (optional)
└── .elif/
    ├── manifest.yaml    # Project configuration
    ├── errors.yaml      # Standardized error codes
    └── policies.yaml    # Access control policies
```

## 🤖 AI Agent Workflow

elif.rs is designed for the **"Plan → Generate → Edit → Test → Deploy"** workflow:

### 1. **Plan**: Define what you want to build
```bash
# AI agent analyzes requirements
elif new blog-api
cd ../blog-api
```

### 2. **Generate**: Scaffold the structure  
```bash
# Create endpoints
elif route add GET /posts list_posts
elif route add POST /posts create_post
elif route add GET /posts/:id get_post

# Create data models
elif model add Post title:string content:text author:string published:boolean
```

### 3. **Edit**: AI modifies only MARKER blocks
```rust
// <<<ELIF:BEGIN agent-editable:create_post>>>
pub async fn create_post(Json(payload): Json<CreatePost>) -> Result<Json<Post>, StatusCode> {
    // AI agent implements business logic here
    let post = Post {
        id: Uuid::new_v4(),
        title: payload.title,
        content: payload.content,
        author: payload.author,
        published: false,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    
    // Database logic, validation, etc.
    Ok(Json(post))
}
// <<<ELIF:END agent-editable:create_post>>>
```

### 4. **Test**: Verify everything works
```bash
cargo run  # Start the server
# API automatically available with OpenAPI docs
```

## 📋 CLI Commands

| Command | Description | Example |
|---------|-------------|---------|
| `elif new <name>` | Create new application | `elif new blog-api` |
| `elif route add METHOD /path handler` | Add HTTP route | `elif route add POST /login auth_handler` |
| `elif model add Name fields` | Add database model | `elif model add User name:string email:string` |
| `elif generate` | Generate from resource specs | `elif generate` |
| `elif check` | Lint and validate project | `elif check` |
| `elif map --json` | Show project structure | `elif map --json` |

## 🔧 Advanced Usage

### Resource Specifications

For complex resources, create YAML specifications:

```yaml
# resources/blog_post.resource.yaml
kind: Resource
name: BlogPost
route: /posts
storage:
  table: blog_posts
  soft_delete: true
  timestamps: true
  fields:
    - { name: id, type: uuid, pk: true, default: gen_random_uuid() }
    - { name: title, type: text, required: true, validate: { min: 1, max: 200 } }
    - { name: content, type: text, required: true }
    - { name: author_id, type: uuid, required: true }
    - { name: published, type: bool, default: false }
    - { name: published_at, type: timestamp }

api:
  operations:
    - { op: create, method: POST, path: "/" }
    - { op: list, method: GET, path: "/", paging: cursor, filter: [published, author_id], order_by: [published_at] }
    - { op: get, method: GET, path: "/:id" }
    - { op: update, method: PATCH, path: "/:id" }
    - { op: delete, method: DELETE, path: "/:id" }

policy:
  auth: user
  rate_limit: "100/m"

validate:
  constraints:
    - { rule: "title.len() > 0", code: EMPTY_TITLE, hint: "Post title cannot be empty" }
```

Then generate everything:

```bash
elif generate  # Creates models, handlers, migrations, tests
```

### Database Integration

elif.rs uses sqlx for type-safe database operations:

```rust
// Generated model with sqlx integration
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BlogPost {
    pub id: Uuid,
    pub title: String,
    pub content: String,
    pub author_id: Uuid,
    pub published: bool,
    pub published_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

### Error Handling

Standardized error responses via `.elif/errors.yaml`:

```yaml
- code: INVALID_CREDENTIALS
  http: 401
  message: "Invalid email or password"
  hint: "Check your login credentials and try again"

- code: POST_NOT_FOUND
  http: 404
  message: "Blog post not found"
  hint: "The requested post may have been deleted"
```

Used in handlers:

```rust
// <<<ELIF:BEGIN agent-editable:get_post>>>
pub async fn get_post(Path(id): Path<Uuid>) -> Result<Json<BlogPost>, StatusCode> {
    match find_post_by_id(id).await {
        Some(post) => Ok(Json(post)),
        None => Err(StatusCode::NOT_FOUND), // Automatically uses POST_NOT_FOUND error
    }
}
// <<<ELIF:END agent-editable:get_post>>>
```

## 🔍 Introspection & Debugging

elif.rs provides built-in introspection endpoints:

- **`/_map.json`**: Complete project structure and route mapping
- **`/_openapi.json`**: OpenAPI 3.0 specification  
- **`/_health`**: Service health check
- **`/_ui`**: Interactive Swagger documentation

Example `/_map.json` response:
```json
{
  "routes": [
    {
      "op_id": "BlogPost.create", 
      "method": "POST",
      "path": "/posts",
      "file": "src/controllers/blog_post.rs",
      "marker": "create_BlogPost"
    }
  ],
  "models": [
    {"name": "BlogPost", "file": "src/models/blog_post.rs"}
  ],
  "specs": [
    {"name": "BlogPost", "file": "resources/blog_post.resource.yaml"}
  ]
}
```

This enables AI agents to understand the project structure programmatically.

## 🧪 Testing

elif.rs generates test scaffolding automatically:

```rust
// Generated in tests/blog_post_http.rs
#[tokio::test]
async fn test_blog_post_crud() {
    let app = create_test_app().await;
    let server = TestServer::new(app).unwrap();
    
    // Test create post
    let response = server
        .post("/posts")
        .json(&json!({
            "title": "My First Post",
            "content": "Hello, world!",
            "author_id": "550e8400-e29b-41d4-a716-446655440000"
        }))
        .await;
    
    assert_eq!(response.status_code(), 201);
    let post: BlogPost = response.json();
    assert_eq!(post.title, "My First Post");
    
    // Test get post
    let response = server.get(&format!("/posts/{}", post.id)).await;
    assert_eq!(response.status_code(), 200);
}
```

Run tests:
```bash
cargo test                    # All tests
elif test --focus blog_post   # Focus on specific resource
```

## 🚀 Deployment

### Development
```bash
cargo run  # Runs on localhost:3000
```

### Production
```bash
# Build optimized binary
cargo build --release

# Set environment variables
export DATABASE_URL=postgresql://user:pass@host/db
export RUST_LOG=info

# Run
./target/release/my-app
```

### Docker
```dockerfile
FROM rust:1.70 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
WORKDIR /app
COPY --from=builder /app/target/release/my-app .
CMD ["./my-app"]
```

## 🛠️ Development

### Prerequisites
- Rust 1.70+
- PostgreSQL (for database features)

### Build from Source
```bash
git clone https://github.com/yourusername/elif.rs
cd elif.rs
cargo build --release
cargo install --path crates/cli
```

### Run Tests
```bash
cargo test --workspace
```

### Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Make your changes (follow MARKER conventions!)
4. Add tests for new functionality
5. Run `elif check` to ensure code quality
6. Commit your changes (`git commit -m 'Add amazing feature'`)
7. Push to the branch (`git push origin feature/amazing-feature`)
8. Open a Pull Request

## 🤝 AI Agent Integration

elif.rs is designed to work seamlessly with AI coding assistants:

### Claude Integration
```bash
# Claude can understand project structure
curl http://localhost:3000/_map.json

# Claude edits only MARKER blocks
# Safe regeneration preserves Claude's work
elif generate  
```

### GitHub Copilot / Cursor
The framework provides clear patterns and conventions that AI assistants can easily follow.

### Custom AI Agents
Use the introspection APIs to build custom AI agents:

```python
import requests

# Get project structure
map_data = requests.get("http://localhost:3000/_map.json").json()

# Understand what needs to be implemented
for route in map_data["routes"]:
    print(f"Route: {route['method']} {route['path']}")
    print(f"Handler: {route['file']}#{route['marker']}")
```

## 📚 Examples

### Blog API
Complete blog API with authentication:
- [Blog API Example](examples/blog-api/) 
- User management, post CRUD, comments
- JWT authentication, role-based access

### E-commerce API  
Product catalog with shopping cart:
- [E-commerce Example](examples/ecommerce-api/)
- Products, categories, orders, payments
- Inventory management, webhooks

### Task Management
Team collaboration tool:
- [Task Manager Example](examples/task-manager/)
- Projects, tasks, assignments, notifications  
- Real-time updates, file attachments

## 🔗 Ecosystem

### Related Crates
- **axum**: HTTP server framework
- **sqlx**: Async SQL toolkit  
- **utoipa**: OpenAPI generation
- **serde**: Serialization framework
- **tracing**: Structured logging

### Extensions
- **elif-auth**: Authentication middleware
- **elif-cache**: Redis caching layer  
- **elif-jobs**: Background job processing
- **elif-realtime**: WebSocket support

## 📖 Documentation

- **[Getting Started Guide](docs/getting-started.md)**: Step-by-step tutorial
- **[API Reference](docs/api-reference.md)**: Complete API documentation  
- **[AI Agent Guide](docs/ai-agents.md)**: Best practices for AI development
- **[Architecture Guide](docs/architecture.md)**: Deep dive into framework design
- **[Migration Guide](docs/migrations.md)**: Database schema management

## 🆘 Support

- **[GitHub Issues](https://github.com/yourusername/elif.rs/issues)**: Bug reports and feature requests
- **[Discussions](https://github.com/yourusername/elif.rs/discussions)**: Community support
- **[Discord](https://discord.gg/elif-rs)**: Real-time chat and help

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- **Axum Team**: For the excellent HTTP framework
- **SQLx Team**: For type-safe database operations  
- **Rust Community**: For the amazing ecosystem
- **AI Research Community**: For making AI-driven development possible

## 🎉 Contributors

<a href="https://github.com/yourusername/elif.rs/graphs/contributors">
  <img src="https://contrib.rocks/image?repo=yourusername/elif.rs" />
</a>

---

**Built with ❤️ for the future of AI-driven development**

> *"The best way to predict the future is to invent it."* - Alan Kay

---

<p align="center">
  <a href="#elif-rs">⬆ Back to Top</a>
</p>