//! Advanced module features - should compile successfully

use elif_http_derive::module;

// Mock types for testing
pub trait DatabaseService: Send + Sync {}
pub trait CacheService: Send + Sync {}
pub trait LoggingService: Send + Sync {}

pub struct PostgresService;
pub struct RedisService;
pub struct FileLogger;

pub struct UserController;
pub struct PostController;
pub struct AuthController;

impl DatabaseService for PostgresService {}
impl CacheService for RedisService {}
impl LoggingService for FileLogger {}

// Empty module sections
#[module(
    providers: [],
    controllers: [],
    imports: [],
    exports: []
)]
pub struct EmptyModule;

// Partial sections only
#[module(
    providers: [PostgresService]
)]
pub struct ProvidersOnlyModule;

#[module(
    controllers: [UserController]
)]
pub struct ControllersOnlyModule;

#[module(
    imports: [ProvidersOnlyModule]
)]
pub struct ImportsOnlyModule;

#[module(
    exports: [PostgresService]
)]
pub struct ExportsOnlyModule;

// Complex module with all features
#[module(
    providers: [
        PostgresService,
        dyn CacheService => RedisService,
        dyn LoggingService => FileLogger @ "file_logger"
    ],
    controllers: [UserController, PostController, AuthController],
    imports: [ProvidersOnlyModule, ControllersOnlyModule],
    exports: [PostgresService, dyn CacheService, dyn LoggingService]
)]
pub struct ComplexModule;

// Module with custom struct fields
#[module(
    providers: [PostgresService],
    controllers: [UserController]
)]
pub struct ModuleWithFields {
    pub name: String,
    pub version: u32,
}

// Module with custom implementation
#[module(
    providers: [PostgresService],
    controllers: [UserController]
)]
pub struct ModuleWithImpl;

impl ModuleWithImpl {
    pub fn new() -> Self {
        Self
    }
    
    pub fn get_name(&self) -> &'static str {
        "ModuleWithImpl"
    }
}

fn main() {}