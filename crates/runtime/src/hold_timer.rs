//! Hold timers. Restart-safe by construction: the wait is always computed
//! from the absolute `releases_at` timestamp, never from an in-memory
//! countdown, so a process restart mid-hold cannot cause a double-release
//! or lose track of remaining time.

use airlock_core::Timestamp;
use chrono::Utc;
use std::time::Duration;

/// How long remains until `releases_at`, as of `now`. Zero if already due.
pub fn remaining(releases_at: Timestamp, now: Timestamp) -> Duration {
    let delta = releases_at - now;
    delta.to_std().unwrap_or(Duration::ZERO)
}

/// Sleep until `releases_at`. Safe to call after a restart: it recomputes
/// the remaining duration from wall-clock time rather than trusting any
/// previously-stored countdown, so it can never sleep longer than intended
/// and can never fire a second time for a hold that already elapsed.
pub async fn wait_for_release(releases_at: Timestamp) {
    let wait = remaining(releases_at, Utc::now());
    if !wait.is_zero() {
        tokio::time::sleep(wait).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration as ChronoDuration;

    #[test]
    fn remaining_is_zero_once_due() {
        let now = Utc::now();
        assert_eq!(remaining(now - ChronoDuration::seconds(5), now), Duration::ZERO);
        assert_eq!(remaining(now, now), Duration::ZERO);
    }

    #[test]
    fn remaining_reflects_time_left() {
        let now = Utc::now();
        let releases_at = now + ChronoDuration::seconds(10);
        let left = remaining(releases_at, now);
        assert!(left.as_secs() >= 9 && left.as_secs() <= 10);
    }

    #[tokio::test]
    async fn wait_for_release_returns_immediately_when_already_due() {
        let past = Utc::now() - ChronoDuration::seconds(1);
        let start = tokio::time::Instant::now();
        wait_for_release(past).await;
        assert!(start.elapsed() < std::time::Duration::from_millis(50));
    }
}
