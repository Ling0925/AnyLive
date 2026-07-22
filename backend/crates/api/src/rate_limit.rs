//! Simple per-key sliding-window rate limiter (IP / email OTP abuse control).

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};

use anylive_common::{AppError, ErrorCode};
use tokio::sync::Mutex;

/// Default: 20 OTP requests per IP per minute.
pub const DEFAULT_OTP_IP_MAX: usize = 20;
/// Default window for OTP IP limiter.
pub const DEFAULT_OTP_IP_WINDOW_SECS: u64 = 60;

/// Sliding-window rate limiter keyed by an arbitrary string (IP, email, …).
#[derive(Debug, Clone)]
pub struct IpRateLimiter {
    max: usize,
    window: Duration,
    hits: Arc<Mutex<HashMap<String, VecDeque<Instant>>>>,
}

impl Default for IpRateLimiter {
    fn default() -> Self {
        Self::new(
            DEFAULT_OTP_IP_MAX,
            Duration::from_secs(DEFAULT_OTP_IP_WINDOW_SECS),
        )
    }
}

impl IpRateLimiter {
    pub fn new(max: usize, window: Duration) -> Self {
        Self {
            max: max.max(1),
            window,
            hits: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Pure check/record against an arbitrary clock instant (unit-testable).
    pub fn try_acquire_at(
        hits: &mut HashMap<String, VecDeque<Instant>>,
        key: &str,
        max: usize,
        window: Duration,
        now: Instant,
    ) -> bool {
        let queue = hits.entry(key.to_string()).or_default();
        while queue
            .front()
            .is_some_and(|t| now.duration_since(*t) >= window)
        {
            queue.pop_front();
        }
        if queue.len() >= max {
            return false;
        }
        queue.push_back(now);
        true
    }

    pub async fn check(&self, key: &str) -> Result<(), AppError> {
        let mut g = self.hits.lock().await;
        if Self::try_acquire_at(&mut g, key, self.max, self.window, Instant::now()) {
            Ok(())
        } else {
            Err(AppError::new(
                ErrorCode::RateLimited,
                "too many requests; try again later",
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_up_to_max_then_blocks() {
        let mut hits = HashMap::new();
        let now = Instant::now();
        let window = Duration::from_secs(60);
        for _ in 0..3 {
            assert!(IpRateLimiter::try_acquire_at(
                &mut hits, "1.2.3.4", 3, window, now
            ));
        }
        assert!(!IpRateLimiter::try_acquire_at(
            &mut hits, "1.2.3.4", 3, window, now
        ));
        // Different key is independent.
        assert!(IpRateLimiter::try_acquire_at(
            &mut hits, "5.6.7.8", 3, window, now
        ));
    }

    #[tokio::test]
    async fn async_check_rate_limits() {
        let lim = IpRateLimiter::new(2, Duration::from_secs(60));
        lim.check("k").await.unwrap();
        lim.check("k").await.unwrap();
        let err = lim.check("k").await.unwrap_err();
        assert_eq!(err.code, ErrorCode::RateLimited);
    }
}
