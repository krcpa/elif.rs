use elif_core_derive::injectable;
use std::sync::Arc;

struct UserRepository;
struct EmailService;
struct MetricsCollector;

#[injectable]
pub struct UserService {
    user_repo: Arc<UserRepository>,
    email_service: Arc<EmailService>,
    metrics: Option<Arc<MetricsCollector>>,
}

fn main() {}