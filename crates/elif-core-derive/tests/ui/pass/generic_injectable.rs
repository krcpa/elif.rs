use std::sync::Arc;
use elif_core_derive::injectable;

pub struct DatabaseService;
pub struct LoggingService;

// Test generic struct with single type parameter
#[injectable]
pub struct GenericService<T: Send + Sync + 'static> {
    db: Arc<DatabaseService>,
    logger: Arc<LoggingService>,
    _phantom: std::marker::PhantomData<T>,
}

// Test struct with multiple generics and where clause  
#[injectable]
pub struct ComplexService<T, U>
where 
    T: Clone + Send + Sync + 'static,
    U: std::fmt::Debug + Send + Sync + 'static,
{
    db: Arc<DatabaseService>,
    logger: Option<Arc<LoggingService>>,
    _phantom: std::marker::PhantomData<(T, U)>,
}

// Test struct with const generics
#[injectable]
pub struct ArrayService<const N: usize> {
    db: Arc<DatabaseService>,
}

fn main() {}