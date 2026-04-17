use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;
use tracing::{info, warn};

/// Simple per-player rate limiter. Tracks request counts in a sliding window.
/// This is a basic implementation; a production system would use a token bucket
/// or leaky bucket algorithm.
#[derive(Clone)]
pub struct RateLimiter {
    requests: Arc<Mutex<HashMap<String, Vec<Instant>>>>,
    max_per_second: u32,
}

impl RateLimiter {
    pub fn new(max_per_second: u32) -> Self {
        Self {
            requests: Arc::new(Mutex::new(HashMap::new())),
            max_per_second,
        }
    }

    pub async fn check(&self, key: &str) -> bool {
        let mut map = self.requests.lock().await;
        let now = Instant::now();
        let window = std::time::Duration::from_secs(1);

        let entries = map.entry(key.to_string()).or_default();
        entries.retain(|t| now.duration_since(*t) < window);

        if entries.len() >= self.max_per_second as usize {
            warn!(key = key, "rate limit exceeded");
            return false;
        }

        entries.push(now);
        true
    }
}

/// Middleware that logs every request with timing information
/// (Design Rule 8: observability).
pub async fn request_logging(request: Request, next: Next) -> Response {
    let method = request.method().clone();
    let uri = request.uri().clone();
    let start = Instant::now();

    info!(method = %method, uri = %uri, "incoming request");

    let response = next.run(request).await;

    let duration = start.elapsed();
    info!(
        method = %method,
        uri = %uri,
        status = %response.status(),
        duration_ms = duration.as_millis(),
        "request completed"
    );

    response
}

/// Placeholder auth middleware. In production this would validate JWTs or
/// session tokens. For now it just checks for the presence of an
/// X-Player-Id header.
pub async fn auth_check(request: Request, next: Next) -> Result<Response, StatusCode> {
    // Skip auth for static assets, health check, and favicon.
    let path = request.uri().path().to_string();
    if path.starts_with("/static")
        || path == "/health"
        || path == "/"
        || path == "/favicon.ico"
    {
        return Ok(next.run(request).await);
    }

    // Check for player ID header (placeholder auth).
    if request.headers().get("X-Player-Id").is_none() {
        warn!(path = %path, "missing X-Player-Id header");
        return Err(StatusCode::UNAUTHORIZED);
    }

    Ok(next.run(request).await)
}
