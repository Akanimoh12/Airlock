//! The screening pipeline and its supervisor.
//!
//! Reader → validate → project → Linker → `Verdict`, run under a timeout,
//! with every failure degrading to `ScreeningOutcome::Unavailable` so the
//! policy engine can fail closed.

use crate::linker::{Linker, LinkerView, TransferFacts};
use crate::reader::{Reader, ScreenError};
use airlock_core::{ClaimedAuthority, Untrusted, Verdict};
use airlock_policy::ScreeningOutcome;
use std::fmt::Display;
use std::future::Future;
use std::time::Duration;

/// How long screening gets before it is treated as dead.
pub const SCREENING_TIMEOUT: Duration = Duration::from_secs(3);

/// Supervise a screening future that can fail.
///
/// This extends `airlock_runtime::screen_with_timeout`, which requires
/// `Future<Output = Verdict>`. That signature has nowhere to put "the Reader
/// socket was refused": the only `Verdict` available for a failure is
/// `Unknown`, and `Unknown` makes `decide` **pass** — a fail-open path
/// through component death, which is precisely what the brief forbids. So
/// errors are handled as errors here. Merging this back into `runtime` by
/// widening that signature to `Result` is the tidier end state; see the
/// handoff note.
///
/// Panics and timeouts are handled the same way A's supervisor handles
/// them, and for the same reason.
pub async fn supervised_screen<F, E>(fut: F, timeout: Duration) -> ScreeningOutcome
where
    F: Future<Output = Result<Verdict, E>> + Send + 'static,
    E: Display + Send + 'static,
{
    let handle = tokio::spawn(fut);
    match tokio::time::timeout(timeout, handle).await {
        Ok(Ok(Ok(verdict))) => ScreeningOutcome::Completed { verdict },
        Ok(Ok(Err(error))) => {
            tracing::warn!(%error, outcome = "unavailable", "screening failed; failing closed");
            ScreeningOutcome::Unavailable
        }
        Ok(Err(join_error)) => {
            tracing::warn!(error = %join_error, outcome = "unavailable", "screening panicked; failing closed");
            ScreeningOutcome::Unavailable
        }
        Err(_elapsed) => {
            tracing::warn!(?timeout, outcome = "unavailable", "screening timed out; failing closed");
            ScreeningOutcome::Unavailable
        }
    }
}

/// One screening pass.
///
/// Note the shape of the argument list: the Reader receives the message and
/// nothing else, the Linker receives a `LinkerView` and nothing else. Neither
/// ever holds both the prose and the account facts.
pub async fn screen(
    reader: Reader,
    linker: Linker,
    message: Untrusted<String>,
    facts: TransferFacts,
) -> Result<Verdict, ScreenError> {
    let started = std::time::Instant::now();
    let (signal, report) = reader.read(&message).await?;
    tracing::info!(
        agent = "reader",
        latency_ms = started.elapsed().as_millis() as u64,
        sanitised = !report.is_clean(),
        "reader completed"
    );

    let view = LinkerView::project(&signal, &facts);
    let judged = std::time::Instant::now();
    let responsiveness = linker.judge(&view);
    tracing::info!(
        agent = "linker",
        latency_ms = judged.elapsed().as_millis() as u64,
        verdict = ?responsiveness.verdict,
        "linker completed"
    );

    Ok(responsiveness.verdict)
}

/// Screening's outcome plus the one piece of the signal the product surface
/// is allowed to see.
///
/// `ScreeningOutcome` lives in `airlock-policy` and stays exactly as it is —
/// the policy engine's inputs are not ours to widen. This wrapper carries the
/// extra field alongside it instead, so the API can tell the user *who* was
/// impersonated without the policy engine gaining a field it would ignore.
///
/// When screening did not complete there is no signal to report, so
/// `claimed_authority` is `None` and the surface falls back to generic copy —
/// which is right, because a fail-closed hold has its own explanation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScreeningReport {
    pub outcome: ScreeningOutcome,
    pub claimed_authority: ClaimedAuthority,
}

/// Screen under supervision. This is what the API calls: it always returns a
/// `ScreeningOutcome`, never an error, and the outcome for any kind of
/// failure is `Unavailable`.
pub async fn screen_supervised(
    reader: Reader,
    linker: Linker,
    message: Untrusted<String>,
    facts: TransferFacts,
) -> ScreeningOutcome {
    screen_reported(reader, linker, message, facts).await.outcome
}

/// As `screen_supervised`, but also reports the claimed authority so the hold
/// screen can name who was being impersonated.
pub async fn screen_reported(
    reader: Reader,
    linker: Linker,
    message: Untrusted<String>,
    facts: TransferFacts,
) -> ScreeningReport {
    // The authority is captured inside the screening task, which may panic or
    // be timed out, so it comes back through a channel rather than a return
    // value — a failed screening simply leaves it unset.
    let (tx, rx) = tokio::sync::oneshot::channel();
    let outcome = supervised_screen(
        screen_capturing(reader, linker, message, facts, tx),
        SCREENING_TIMEOUT,
    )
    .await;

    ScreeningReport {
        outcome,
        claimed_authority: match outcome {
            ScreeningOutcome::Completed { .. } => rx.await.unwrap_or_default(),
            ScreeningOutcome::Unavailable => ClaimedAuthority::None,
        },
    }
}

/// `screen`, with the claimed authority sent out as a side channel. Kept
/// private: `screen` remains the shape everything else builds against.
async fn screen_capturing(
    reader: Reader,
    linker: Linker,
    message: Untrusted<String>,
    facts: TransferFacts,
    authority: tokio::sync::oneshot::Sender<ClaimedAuthority>,
) -> Result<Verdict, ScreenError> {
    let (signal, report) = reader.read(&message).await?;
    tracing::info!(
        agent = "reader",
        sanitised = !report.is_clean(),
        "reader completed"
    );

    // Send before the Linker runs, so a Linker failure does not lose it.
    let _ = authority.send(signal.get().claimed_authority);

    let view = LinkerView::project(&signal, &facts);
    let responsiveness = linker.judge(&view);
    tracing::info!(agent = "linker", verdict = ?responsiveness.verdict, "linker completed");
    Ok(responsiveness.verdict)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::validate::mask_msisdn;
    use airlock_core::Money;

    fn facts() -> TransferFacts {
        TransferFacts {
            amount: Money { minor_units: 500_000, currency: *b"NGN" },
            recipient: mask_msisdn("08031234567").unwrap(),
            recipient_established: false,
            minutes_since_contact: Some(4),
        }
    }

    #[tokio::test]
    async fn the_stub_pipeline_screens_the_demo_scam_as_responsive() {
        let msg = Untrusted::new(
            "MTN Alert: your account will be suspended today. Call 08031234567 to reactivate."
                .to_string(),
        );
        let outcome = screen_supervised(Reader::Stub, Linker::Stub, msg, facts()).await;
        assert_eq!(outcome, ScreeningOutcome::Completed { verdict: Verdict::Responsive });
    }

    #[tokio::test]
    async fn an_unreachable_reader_fails_closed_immediately() {
        let reader = Reader::remote("http://127.0.0.1:1", Duration::from_millis(200));
        let msg = Untrusted::new("anything".to_string());
        let started = std::time::Instant::now();
        let outcome = screen_supervised(reader, Linker::Stub, msg, facts()).await;
        assert_eq!(outcome, ScreeningOutcome::Unavailable);
        // Fast, not "wait out the full timeout" — beat six has to land.
        assert!(started.elapsed() < SCREENING_TIMEOUT);
    }

    #[tokio::test]
    async fn a_panicking_screen_fails_closed() {
        let outcome = supervised_screen(
            async {
                panic!("reader task died");
                #[allow(unreachable_code)]
                Ok::<_, ScreenError>(Verdict::Unrelated)
            },
            Duration::from_secs(1),
        )
        .await;
        assert_eq!(outcome, ScreeningOutcome::Unavailable);
    }

    #[tokio::test]
    async fn a_hanging_screen_fails_closed() {
        let outcome = supervised_screen(
            async {
                tokio::time::sleep(Duration::from_secs(60)).await;
                Ok::<_, ScreenError>(Verdict::Unrelated)
            },
            Duration::from_millis(20),
        )
        .await;
        assert_eq!(outcome, ScreeningOutcome::Unavailable);
    }

    #[tokio::test]
    async fn failure_never_produces_unknown_which_would_pass() {
        // Guards the reasoning in `supervised_screen`'s doc comment: if a
        // failure ever became Completed{Unknown}, `decide` would pass a
        // novel-recipient transfer with a dead Reader.
        let outcome = supervised_screen(
            async { Err::<Verdict, _>(ScreenError::Http(500)) },
            Duration::from_secs(1),
        )
        .await;
        assert_eq!(outcome, ScreeningOutcome::Unavailable);
        assert_ne!(outcome, ScreeningOutcome::Completed { verdict: Verdict::Unknown });
    }
}
