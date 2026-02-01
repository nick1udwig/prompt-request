use std::time::{Duration, Instant};

use dashmap::DashMap;

use crate::error::ApiError;

#[derive(Clone)]
pub struct RateLimiter {
    window: Duration,
    entries: DashMap<String, Instant>,
}

impl RateLimiter {
    pub fn new(window: Duration) -> Self {
        Self {
            window,
            entries: DashMap::new(),
        }
    }

    pub fn check(&self, key: &str) -> Result<(), ApiError> {
        let now = Instant::now();
        if let Some(entry) = self.entries.get(key) {
            let elapsed = now.duration_since(*entry);
            if elapsed < self.window {
                let retry_after = (self.window - elapsed).as_secs().max(1);
                return Err(ApiError::RateLimited {
                    retry_after_secs: retry_after,
                });
            }
        }
        self.entries.insert(key.to_string(), now);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rate_limiter_blocks() {
        let limiter = RateLimiter::new(Duration::from_secs(1));
        assert!(limiter.check("a").is_ok());
        assert!(limiter.check("a").is_err());
    }
}
