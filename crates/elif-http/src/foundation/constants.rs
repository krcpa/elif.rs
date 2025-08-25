pub const DEFAULT_PORT: u16 = 3000;
pub const DEFAULT_REQUEST_TIMEOUT_SECS: u32 = 30;
pub const DEFAULT_KEEP_ALIVE_TIMEOUT_SECS: u32 = 75;
pub const DEFAULT_MAX_REQUEST_SIZE: usize = 16 * 1024 * 1024; // 16MB
pub const DEFAULT_HEALTH_CHECK_PATH: &str = "/health";
pub const DEFAULT_SHUTDOWN_TIMEOUT_SECS: u32 = 10;

pub const HEADER_REQUEST_ID: &str = "x-request-id";
pub const HEADER_CONTENT_TYPE: &str = "content-type";
pub const HEADER_AUTHORIZATION: &str = "authorization";

pub const CONTENT_TYPE_JSON: &str = "application/json";
pub const CONTENT_TYPE_FORM: &str = "application/x-www-form-urlencoded";
pub const CONTENT_TYPE_TEXT: &str = "text/plain";
pub const CONTENT_TYPE_HTML: &str = "text/html";
