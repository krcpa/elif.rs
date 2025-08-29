use std::net::{TcpListener, SocketAddr};
use std::time::Duration;
use tokio::net::TcpStream;

/// Check if a port is currently in use on localhost
pub fn is_port_in_use(port: u16) -> bool {
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    TcpListener::bind(addr).is_err()
}

/// Parse Redis URL to extract host and port
/// Handles common Redis URL formats:
/// - redis://localhost:6379
/// - redis://user:pass@localhost:6379
/// - redis://localhost:6379/0
/// - localhost:6379
pub fn parse_redis_url(redis_url: &str) -> Option<(String, u16)> {
    if let Ok(parsed_url) = url::Url::parse(redis_url) {
        // Parsed as a full URL
        let host = parsed_url.host_str().unwrap_or("127.0.0.1").to_string();
        let port = parsed_url.port().unwrap_or(6379);
        Some((host, port))
    } else if redis_url.contains(':') {
        // Try to parse as "host:port"
        let parts: Vec<&str> = redis_url.splitn(2, ':').collect();
        if parts.len() == 2 {
            if let Ok(port) = parts[1].parse::<u16>() {
                return Some((parts[0].to_string(), port));
            }
        }
        None
    } else {
        // Just a hostname, use default port
        Some((redis_url.to_string(), 6379))
    }
}

/// Check if Redis is accessible at the given host and port
pub async fn is_redis_accessible(host: &str, port: u16) -> bool {
    // Try to establish a TCP connection to the Redis server
    let addr = match format!("{}:{}", host, port).parse::<SocketAddr>() {
        Ok(addr) => addr,
        Err(_) => return false,
    };
    
    // Use a short timeout for the connection attempt
    match tokio::time::timeout(Duration::from_millis(1000), async move {
        TcpStream::connect(addr).await
    }).await {
        Ok(Ok(_)) => true,
        _ => false,
    }
}