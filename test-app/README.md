# test-app

Created with elif.rs - LLM-friendly Rust web framework.

## Quick Start

```bash
# Add a route
elif route add GET /hello hello_controller

# Add a model  
elif model add User name:string email:string

# Run the server
cargo run
```

## Available Commands

- `elif route add METHOD /path controller_name` - Add HTTP route
- `elif model add Name field:type` - Add database model
- `elif migrate` - Run database migrations
- `elif routes` - List all routes

## Structure

- `src/controllers/` - HTTP controllers
- `src/models/` - Database models  
- `src/routes/` - Route definitions
- `src/middleware/` - HTTP middleware
- `migrations/` - Database migrations
- `resources/` - Resource specifications
