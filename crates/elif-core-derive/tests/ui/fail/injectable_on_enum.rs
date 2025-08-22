use elif_core_derive::injectable;

#[injectable]
pub enum UserService {
    Active,
    Inactive,
}

fn main() {}