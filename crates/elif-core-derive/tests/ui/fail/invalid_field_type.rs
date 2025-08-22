use elif_core_derive::injectable;

#[injectable]
pub struct UserService {
    invalid_field: String, // Should be Arc<T> or Option<Arc<T>>
}

fn main() {}