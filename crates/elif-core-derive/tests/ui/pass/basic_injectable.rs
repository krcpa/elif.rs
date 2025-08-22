use elif_core_derive::injectable;
use std::sync::Arc;

struct UserRepository;
struct EmailService;

#[injectable]
pub struct UserService {
    user_repo: Arc<UserRepository>,
    email_service: Arc<EmailService>,
}

fn main() {}