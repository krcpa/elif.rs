//! Default configuration values

use crate::foundation::constants::*;

pub struct HttpDefaults;

impl HttpDefaults {
    pub const REQUEST_TIMEOUT_SECS: u64 = DEFAULT_REQUEST_TIMEOUT_SECS as u64;
    pub const KEEP_ALIVE_TIMEOUT_SECS: u64 = DEFAULT_KEEP_ALIVE_TIMEOUT_SECS as u64;
    pub const MAX_REQUEST_SIZE: usize = DEFAULT_MAX_REQUEST_SIZE;
    pub const ENABLE_TRACING: bool = true;
    pub const HEALTH_CHECK_PATH: &'static str = DEFAULT_HEALTH_CHECK_PATH;
    pub const SHUTDOWN_TIMEOUT_SECS: u64 = DEFAULT_SHUTDOWN_TIMEOUT_SECS as u64;
}
