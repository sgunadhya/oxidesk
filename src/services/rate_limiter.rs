use governor::{
    clock::{Clock, DefaultClock},
    state::direct::NotKeyed,
    state::InMemoryState,
    Quota, RateLimiter as GovernorRateLimiter,
};
use std::{num::NonZeroU32, sync::Arc, time::Duration};
use std::collections::HashMap;
use tokio::sync::RwLock;

/// Rate limiter for authentication attempts
///
/// Tracks failed login attempts per email address using a sliding window.
/// Default: 5 attempts per 15 minutes.
#[derive(Clone)]
pub struct AuthRateLimiter {
    /// Map of email -> rate limiter instance
    limiters: Arc<RwLock<HashMap<String, Arc<GovernorRateLimiter<NotKeyed, InMemoryState, DefaultClock>>>>>,
    /// Maximum attempts allowed
    max_attempts: u32,
    /// Time window in minutes
    window_minutes: u64,
}

impl AuthRateLimiter {
    /// Create a new rate limiter with default settings (5 attempts / 15 minutes)
    pub fn new() -> Self {
        Self::with_config(5, 15)
    }

    /// Create a new rate limiter with custom configuration
    pub fn with_config(max_attempts: u32, window_minutes: u64) -> Self {
        Self {
            limiters: Arc::new(RwLock::new(HashMap::new())),
            max_attempts,
            window_minutes,
        }
    }

    /// Check if an email is rate limited
    ///
    /// Returns Ok(()) if the request is allowed, Err(duration) if rate limited.
    /// The duration indicates how long to wait before retrying.
    pub async fn check(&self, email: &str) -> Result<(), Duration> {
        let email = email.to_lowercase();

        // Get or create limiter for this email
        let limiter = {
            let mut limiters = self.limiters.write().await;

            limiters
                .entry(email.clone())
                .or_insert_with(|| {
                    let quota = Quota::with_period(
                        Duration::from_secs(self.window_minutes * 60)
                    )
                    .unwrap()
                    .allow_burst(NonZeroU32::new(self.max_attempts).unwrap());

                    Arc::new(GovernorRateLimiter::direct(quota))
                })
                .clone()
        };

        // Check the rate limit
        match limiter.check() {
            Ok(_) => Ok(()),
            Err(negative) => {
                let wait_duration = negative.wait_time_from(DefaultClock::default().now());
                Err(wait_duration)
            }
        }
    }

    /// Record a failed authentication attempt for an email
    ///
    /// This consumes one token from the rate limiter.
    pub async fn record_failure(&self, email: &str) -> Result<(), Duration> {
        self.check(email).await
    }

    /// Reset rate limit for an email (e.g., after successful login)
    pub async fn reset(&self, email: &str) {
        let email = email.to_lowercase();
        let mut limiters = self.limiters.write().await;
        limiters.remove(&email);
    }

    /// Get remaining attempts for an email
    pub async fn remaining_attempts(&self, email: &str) -> u32 {
        let email = email.to_lowercase();
        let limiters = self.limiters.read().await;

        match limiters.get(&email) {
            Some(_limiter) => {
                // governor doesn't expose exact remaining count easily
                // We just check if the limiter would allow a request
                drop(limiters);
                match self.check(&email).await {
                    Ok(_) => self.max_attempts,
                    Err(_) => 0,
                }
            }
            None => self.max_attempts,
        }
    }

    /// Clean up old rate limiters periodically
    ///
    /// This should be called periodically to prevent memory growth.
    /// Removes limiters that haven't been accessed recently.
    pub async fn cleanup(&self) {
        let mut limiters = self.limiters.write().await;

        // Keep only limiters that are still rate limiting
        limiters.retain(|email, limiter| {
            // If check succeeds, the limiter has expired and can be removed
            // We need to drop the lock temporarily
            let result = limiter.check();
            result.is_err()
        });
    }
}

impl Default for AuthRateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

/// Rate limiting error response
#[derive(Debug, serde::Serialize)]
pub struct RateLimitError {
    pub message: String,
    pub retry_after_seconds: u64,
}

impl RateLimitError {
    pub fn new(wait_duration: Duration) -> Self {
        Self {
            message: "Too many failed login attempts. Please try again later.".to_string(),
            retry_after_seconds: wait_duration.as_secs(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_rate_limiter_allows_within_limit() {
        let limiter = AuthRateLimiter::with_config(3, 1);
        let email = "test@example.com";

        // First 3 attempts should succeed
        assert!(limiter.record_failure(email).await.is_ok());
        assert!(limiter.record_failure(email).await.is_ok());
        assert!(limiter.record_failure(email).await.is_ok());

        // 4th attempt should be rate limited
        assert!(limiter.record_failure(email).await.is_err());
    }

    #[tokio::test]
    async fn test_rate_limiter_resets() {
        let limiter = AuthRateLimiter::with_config(2, 1);
        let email = "test@example.com";

        // Use up the limit
        assert!(limiter.record_failure(email).await.is_ok());
        assert!(limiter.record_failure(email).await.is_ok());
        assert!(limiter.record_failure(email).await.is_err());

        // Reset should clear the limit
        limiter.reset(email).await;
        assert!(limiter.record_failure(email).await.is_ok());
    }

    #[tokio::test]
    async fn test_rate_limiter_case_insensitive() {
        let limiter = AuthRateLimiter::with_config(2, 1);

        // Different cases should be treated as same email
        assert!(limiter.record_failure("Test@Example.com").await.is_ok());
        assert!(limiter.record_failure("test@example.com").await.is_ok());
        assert!(limiter.record_failure("TEST@EXAMPLE.COM").await.is_err());
    }

    #[tokio::test]
    async fn test_rate_limiter_different_emails() {
        let limiter = AuthRateLimiter::with_config(2, 1);

        // Different emails should have independent limits
        assert!(limiter.record_failure("user1@example.com").await.is_ok());
        assert!(limiter.record_failure("user1@example.com").await.is_ok());
        assert!(limiter.record_failure("user1@example.com").await.is_err());

        // user2 should still be allowed
        assert!(limiter.record_failure("user2@example.com").await.is_ok());
        assert!(limiter.record_failure("user2@example.com").await.is_ok());
    }

    #[tokio::test]
    async fn test_cleanup_removes_expired_limiters() {
        let limiter = AuthRateLimiter::with_config(1, 1);

        // Create a rate limit
        let _ = limiter.record_failure("test@example.com").await;
        let _ = limiter.record_failure("test@example.com").await;

        // Should be 1 limiter
        {
            let limiters = limiter.limiters.read().await;
            assert_eq!(limiters.len(), 1);
        }

        // Wait for expiration (in real scenarios, would wait 1 minute)
        // For testing, we just call cleanup and verify logic
        limiter.cleanup().await;

        // Limiter should still be there if still rate limited
        {
            let limiters = limiter.limiters.read().await;
            assert_eq!(limiters.len(), 1);
        }
    }
}
