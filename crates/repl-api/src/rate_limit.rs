use axum::{
    body::Body,
    extract::{ConnectInfo, Request},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Token bucket for rate limiting
#[derive(Debug, Clone)]
struct TokenBucket {
    tokens: f64,
    capacity: f64,
    refill_rate: f64, // tokens per second
    last_refill: Instant,
}

impl TokenBucket {
    fn new(capacity: f64, refill_rate: f64) -> Self {
        Self {
            tokens: capacity,
            capacity,
            refill_rate,
            last_refill: Instant::now(),
        }
    }

    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        let new_tokens = elapsed * self.refill_rate;

        self.tokens = (self.tokens + new_tokens).min(self.capacity);
        self.last_refill = now;
    }

    fn try_consume(&mut self, tokens: f64) -> bool {
        self.refill();

        if self.tokens >= tokens {
            self.tokens -= tokens;
            true
        } else {
            false
        }
    }

    fn time_until_available(&mut self, tokens: f64) -> Duration {
        self.refill();

        if self.tokens >= tokens {
            Duration::from_secs(0)
        } else {
            let needed = tokens - self.tokens;
            let seconds = needed / self.refill_rate;
            Duration::from_secs_f64(seconds)
        }
    }
}

/// Rate limiter state
#[derive(Clone)]
pub struct RateLimiter {
    buckets: Arc<RwLock<HashMap<String, TokenBucket>>>,
    capacity: f64,
    refill_rate: f64,
    cleanup_interval: Duration,
}

impl RateLimiter {
    /// Create a new rate limiter
    ///
    /// # Arguments
    /// * `requests_per_minute` - Maximum requests per minute per IP
    /// * `burst_size` - Maximum burst size (capacity)
    pub fn new(requests_per_minute: f64, burst_size: f64) -> Self {
        let refill_rate = requests_per_minute / 60.0; // convert to per-second rate

        let limiter = Self {
            buckets: Arc::new(RwLock::new(HashMap::new())),
            capacity: burst_size,
            refill_rate,
            cleanup_interval: Duration::from_secs(300), // cleanup every 5 minutes
        };

        // Spawn cleanup task
        let buckets_clone = limiter.buckets.clone();
        let cleanup_interval = limiter.cleanup_interval;
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(cleanup_interval);
            loop {
                interval.tick().await;
                let mut buckets = buckets_clone.write().await;

                // Remove buckets that are full and haven't been used recently
                buckets.retain(|_, bucket| {
                    let age = Instant::now().duration_since(bucket.last_refill);
                    !(bucket.tokens >= bucket.capacity && age > Duration::from_secs(600))
                });
            }
        });

        limiter
    }

    /// Check if a request from the given IP should be allowed
    pub async fn check_rate_limit(&self, ip: &str) -> Result<(), Duration> {
        let mut buckets = self.buckets.write().await;

        let bucket = buckets
            .entry(ip.to_string())
            .or_insert_with(|| TokenBucket::new(self.capacity, self.refill_rate));

        if bucket.try_consume(1.0) {
            Ok(())
        } else {
            Err(bucket.time_until_available(1.0))
        }
    }

    /// Get the current state for an IP (for monitoring/debugging)
    pub async fn get_bucket_state(&self, ip: &str) -> Option<(f64, f64)> {
        let buckets = self.buckets.read().await;
        buckets.get(ip).map(|b| (b.tokens, b.capacity))
    }
}

/// Middleware function for rate limiting
pub async fn rate_limit_middleware(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    request: Request,
    next: Next,
) -> Response {
    // Get rate limiter from request extensions
    let limiter = match request.extensions().get::<RateLimiter>() {
        Some(limiter) => limiter.clone(),
        None => {
            tracing::warn!("Rate limiter not found in request extensions");
            return next.run(request).await;
        }
    };

    let ip = addr.ip().to_string();

    match limiter.check_rate_limit(&ip).await {
        Ok(()) => {
            // Request allowed
            next.run(request).await
        }
        Err(retry_after) => {
            // Rate limit exceeded
            tracing::warn!("Rate limit exceeded for IP: {}", ip);

            let retry_seconds = retry_after.as_secs();
            let response = (
                StatusCode::TOO_MANY_REQUESTS,
                [("Retry-After", retry_seconds.to_string())],
                format!(
                    "Rate limit exceeded. Please retry after {} seconds.",
                    retry_seconds
                ),
            );

            response.into_response()
        }
    }
}

/// Extension trait for adding rate limiter to router
use axum::Router;

pub trait RateLimitExt {
    fn with_rate_limit(self, limiter: RateLimiter) -> Self;
}

impl RateLimitExt for Router {
    fn with_rate_limit(self, limiter: RateLimiter) -> Self {
        self.layer(axum::middleware::from_fn(move |req, next| {
            let limiter = limiter.clone();
            async move {
                // Add limiter to extensions so middleware can access it
                let mut req = req;
                req.extensions_mut().insert(limiter);
                rate_limit_middleware(
                    ConnectInfo(
                        req.extensions()
                            .get::<ConnectInfo<SocketAddr>>()
                            .copied()
                            .unwrap_or(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 0)))),
                    ),
                    req,
                    next,
                )
                .await
            }
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_bucket_consume() {
        let mut bucket = TokenBucket::new(10.0, 1.0);

        // Should be able to consume up to capacity
        assert!(bucket.try_consume(5.0));
        assert!(bucket.try_consume(5.0));
        assert!(!bucket.try_consume(1.0)); // Now empty

        assert_eq!(bucket.tokens, 0.0);
    }

    #[test]
    fn test_token_bucket_refill() {
        let mut bucket = TokenBucket::new(10.0, 10.0); // 10 tokens per second
        bucket.tokens = 0.0;

        // Manually advance time by setting last_refill in the past
        bucket.last_refill = Instant::now() - Duration::from_secs(1);

        bucket.refill();

        // Should have refilled ~10 tokens
        assert!((bucket.tokens - 10.0).abs() < 0.1);
    }

    #[test]
    fn test_token_bucket_max_capacity() {
        let mut bucket = TokenBucket::new(10.0, 10.0);

        // Set time far in the past
        bucket.last_refill = Instant::now() - Duration::from_secs(100);

        bucket.refill();

        // Should not exceed capacity
        assert_eq!(bucket.tokens, 10.0);
    }

    #[tokio::test]
    async fn test_rate_limiter_basic() {
        let limiter = RateLimiter::new(60.0, 10.0); // 60 req/min, burst of 10

        // Should allow burst
        for _ in 0..10 {
            assert!(limiter.check_rate_limit("1.1.1.1").await.is_ok());
        }

        // Should deny next request
        assert!(limiter.check_rate_limit("1.1.1.1").await.is_err());
    }

    #[tokio::test]
    async fn test_rate_limiter_different_ips() {
        let limiter = RateLimiter::new(60.0, 5.0);

        // Different IPs should have separate buckets
        for _ in 0..5 {
            assert!(limiter.check_rate_limit("1.1.1.1").await.is_ok());
        }
        assert!(limiter.check_rate_limit("1.1.1.1").await.is_err());

        // Different IP should still work
        assert!(limiter.check_rate_limit("1.1.1.2").await.is_ok());
    }

    #[tokio::test]
    async fn test_rate_limiter_refill() {
        let limiter = RateLimiter::new(600.0, 5.0); // 10 req/second for fast test

        // Exhaust bucket
        for _ in 0..5 {
            assert!(limiter.check_rate_limit("1.1.1.1").await.is_ok());
        }
        assert!(limiter.check_rate_limit("1.1.1.1").await.is_err());

        // Wait for refill
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Should be able to make another request
        assert!(limiter.check_rate_limit("1.1.1.1").await.is_ok());
    }
}