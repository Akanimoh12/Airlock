//! Worker supervision. Screening runs as a spawned task with a timeout; a
//! panic or a timeout both degrade to `ScreeningOutcome::Unavailable`,
//! which the policy engine turns into a fail-closed hold. This crate does
//! not know anything about Rig or the agent implementation — it only ever
//! sees a `Future<Output = Verdict>`.

use airlock_core::Verdict;
use airlock_policy::ScreeningOutcome;
use std::future::Future;
use std::time::Duration;

/// Run a screening future under a timeout. Returns `Unavailable` if the
/// future panics, is cancelled, or does not complete in time — never lets
/// an error silently become a pass.
pub async fn screen_with_timeout<F>(fut: F, timeout: Duration) -> ScreeningOutcome
where
    F: Future<Output = Verdict> + Send + 'static,
{
    let handle = tokio::spawn(fut);
    match tokio::time::timeout(timeout, handle).await {
        Ok(Ok(verdict)) => ScreeningOutcome::Completed { verdict },
        Ok(Err(join_err)) => {
            tracing::warn!(error = %join_err, "screening task panicked; failing closed");
            ScreeningOutcome::Unavailable
        }
        Err(_elapsed) => {
            tracing::warn!(?timeout, "screening timed out; failing closed");
            ScreeningOutcome::Unavailable
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration as StdDuration;

    #[tokio::test]
    async fn completed_screening_returns_verdict() {
        let outcome = screen_with_timeout(async { Verdict::Responsive }, StdDuration::from_secs(1)).await;
        assert_eq!(outcome, ScreeningOutcome::Completed { verdict: Verdict::Responsive });
    }

    #[tokio::test]
    async fn dead_reader_produces_unavailable() {
        let outcome = screen_with_timeout(
            async {
                panic!("Reader process died");
                #[allow(unreachable_code)]
                Verdict::Unrelated
            },
            StdDuration::from_secs(1),
        )
        .await;
        assert_eq!(outcome, ScreeningOutcome::Unavailable);
    }

    #[tokio::test]
    async fn slow_reader_times_out_to_unavailable() {
        let outcome = screen_with_timeout(
            async {
                tokio::time::sleep(StdDuration::from_secs(60)).await;
                Verdict::Unrelated
            },
            StdDuration::from_millis(10),
        )
        .await;
        assert_eq!(outcome, ScreeningOutcome::Unavailable);
    }
}
