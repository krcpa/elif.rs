# elif-core-derive

Derive macros for elif-core dependency injection system.

## Injectable Macro

The `#[injectable]` attribute macro automatically implements the `Injectable` trait for structs by analyzing their field types.

### Usage

```rust
use elif_core_derive::injectable;
use std::sync::Arc;

struct UserRepository;
struct EmailService;
struct MetricsCollector;

#[injectable]
pub struct UserService {
    user_repo: Arc<UserRepository>,          // Required dependency
    email_service: Arc<EmailService>,        // Required dependency
    metrics: Option<Arc<MetricsCollector>>,  // Optional dependency
}
```

### Generated Code

The macro generates an `Injectable` implementation like this:

```rust
impl Injectable for UserService {
    fn dependencies() -> Vec<ServiceId> {
        vec![
            ServiceId::of::<UserRepository>(),
            ServiceId::of::<EmailService>(),
            ServiceId::of::<MetricsCollector>(),
        ]
    }
    
    fn create<R: DependencyResolver>(resolver: &R) -> Result<Self, CoreError>
    where
        Self: Sized,
    {
        Ok(Self {
            user_repo: resolver.resolve::<UserRepository>()?,
            email_service: resolver.resolve::<EmailService>()?,
            metrics: resolver.try_resolve::<MetricsCollector>(),
        })
    }
}
```

### Supported Field Types

- `Arc<T>` - Required dependency
- `Option<Arc<T>>` - Optional dependency  

### Usage with IoC Container

```rust
use elif_core::container::{IocContainerBuilder, ServiceBinder};

let mut builder = IocContainerBuilder::new();

builder
    .bind_factory::<UserRepository, _, _>(|| Ok(UserRepository::new()))
    .bind_factory::<EmailService, _, _>(|| Ok(EmailService::new()))
    .bind_factory::<MetricsCollector, _, _>(|| Ok(MetricsCollector::new()));

let container = builder.build()?;

// Create service using Injectable trait
let user_service = UserService::create(&container)?;
```

### Benefits

- **Zero boilerplate**: No need to manually implement `Injectable`
- **Type safety**: Compile-time dependency analysis
- **NestJS-style**: Familiar decorator pattern for AI agents and developers coming from NestJS
- **Optional dependencies**: Automatically handles `Option<Arc<T>>` fields
- **Clear errors**: Helpful compiler messages for invalid field types

### Error Examples

Using unsupported field types:

```rust
#[injectable]
pub struct BadService {
    invalid_field: String, // Error: must be Arc<T> or Option<Arc<T>>
}
```

Applying to non-struct types:

```rust
#[injectable] // Error: can only be applied to structs
pub enum MyEnum {
    Variant,
}
```