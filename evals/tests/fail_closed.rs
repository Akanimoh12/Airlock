//! Component death must degrade toward holding, never toward passing.
//!
//! The policy engine's own fail-closed rule is A's test. What is checked here
//! is the path B owns: that a Reader which is dead, slow, or returning
//! nonsense actually produces `ScreeningOutcome::Unavailable`, rather than
//! some verdict that happens to let the transfer through.

use airlock_agents::{
    screen_supervised, supervised_screen, validate_reader_json, Linker, Reader, ScreenError,
    SchemaError, TransferFacts,
};
use airlock_core::{MaskedMsisdn, Money, Untrusted, Verdict};
use airlock_policy::{
    decide, DecisionInput, InboundContact, PolicyDecision, RecipientProfile, ScreeningOutcome,
};
use airlock_evals::epoch;
use chrono::Duration;
use std::time::Duration as StdDuration;

fn facts() -> TransferFacts {
    TransferFacts {
        amount: Money { minor_units: 500_000, currency: *b"NGN" },
        recipient: MaskedMsisdn("*******567".to_string()),
        recipient_established: false,
        minutes_since_contact: Some(4),
    }
}

/// The end-to-end version of demo beat six.
#[tokio::test]
async fn a_dead_reader_holds_a_novel_recipient() {
    let reader = Reader::remote("http://127.0.0.1:1", StdDuration::from_millis(200));
    let screening = screen_supervised(
        reader,
        Linker::Stub,
        Untrusted::new("MTN: account suspended today, call 08031234567".to_string()),
        facts(),
    )
    .await;
    assert_eq!(screening, ScreeningOutcome::Unavailable);

    let decision = decide(&DecisionInput {
        recipient: RecipientProfile { established: false },
        inbound_contact: Some(InboundContact { received_at: epoch() - Duration::minutes(4) }),
        screening,
        recipient_risk: airlock_core::RecipientRisk::Unremarkable,
        proposed_at: epoch(),
    });
    assert!(matches!(decision, PolicyDecision::Hold { .. }), "got {decision:?}");
}

#[tokio::test]
async fn malformed_reader_output_is_unavailable_not_a_verdict() {
    for body in [
        "",
        "{",
        r#"{"urgency":"High","authority_c"#,
        "null",
        "[]",
        r#"{"urgency":"Sideways","requested_action":"SendMoney","confidence":"High"}"#,
        "<html>502 Bad Gateway</html>",
    ] {
        assert!(
            matches!(validate_reader_json(body), Err(SchemaError::Malformed(_))),
            "{body:?} should be malformed"
        );
    }
}

#[tokio::test]
async fn a_reader_error_never_becomes_a_passing_verdict() {
    // The trap this guards: `Verdict::Unknown` is the only verdict that could
    // stand in for "screening failed", and `decide` passes on it. If an
    // error ever mapped to Completed{Unknown}, a dead Reader would clear a
    // novel-recipient transfer.
    for error in [
        ScreenError::Unreachable("connection refused".into()),
        ScreenError::Http(500),
        ScreenError::Schema(SchemaError::Malformed("truncated".into())),
        ScreenError::Schema(SchemaError::NegativeAmount(-1)),
    ] {
        let outcome =
            supervised_screen(async move { Err::<Verdict, _>(error) }, StdDuration::from_secs(1))
                .await;
        assert_eq!(outcome, ScreeningOutcome::Unavailable);
    }
}

#[tokio::test]
async fn a_hanging_reader_holds_rather_than_blocking_forever() {
    let started = std::time::Instant::now();
    let outcome = supervised_screen(
        async {
            tokio::time::sleep(StdDuration::from_secs(300)).await;
            Ok::<_, ScreenError>(Verdict::Unrelated)
        },
        StdDuration::from_millis(50),
    )
    .await;
    assert_eq!(outcome, ScreeningOutcome::Unavailable);
    assert!(started.elapsed() < StdDuration::from_secs(5));
}

#[tokio::test]
async fn a_panicking_reader_holds() {
    let outcome = supervised_screen(
        async {
            panic!("reader process died mid-read");
            #[allow(unreachable_code)]
            Ok::<_, ScreenError>(Verdict::Unrelated)
        },
        StdDuration::from_secs(1),
    )
    .await;
    assert_eq!(outcome, ScreeningOutcome::Unavailable);
}

/// The other half of honesty: failing closed must not mean failing on
/// everything. A dead Reader is not a reason to stop someone paying their
/// landlord.
#[tokio::test]
async fn a_dead_reader_still_lets_an_established_recipient_through() {
    let decision = decide(&DecisionInput {
        recipient: RecipientProfile { established: true },
        inbound_contact: Some(InboundContact { received_at: epoch() - Duration::minutes(4) }),
        screening: ScreeningOutcome::Unavailable,
        recipient_risk: airlock_core::RecipientRisk::Unremarkable,
        proposed_at: epoch(),
    });
    assert_eq!(decision, PolicyDecision::Pass);
}
