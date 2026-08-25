//! The adversarial corpus and the harness that runs it.
//!
//! `cargo test -p airlock-evals` is the suite. It runs entirely offline
//! against stub mode, so it is a pre-merge check rather than something we
//! only remember to do when a key is loaded.
//!
//! The corpus lives in `fixtures/` as JSON so it can grow without touching
//! Rust, and so the legitimate-transfer half is visibly the same shape as
//! the scam half — no thumb on the scale.

use airlock_agents::{Linker, Reader, TransferFacts};
use airlock_core::{MaskedMsisdn, Money, Untrusted, Verdict};
use airlock_policy::{
    decide, DecisionInput, InboundContact, PolicyDecision, RecipientProfile, ScreeningOutcome,
};
use chrono::{DateTime, Duration, TimeZone, Utc};

/// One case: a message, and the transfer the user then attempts.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct Case {
    /// Short identifier used in failure output.
    pub name: String,
    /// What the corpus expects. `Hold` for scams, `Pass` for legitimate
    /// transfers — though a legitimate case that holds is a measured false
    /// positive, not a test failure. See `false_positive_rate`.
    pub expect: Expect,
    /// The inbound message, verbatim.
    pub message: String,
    /// Minutes between the message arriving and the transfer being proposed.
    pub minutes_later: i64,
    /// Amount in minor units.
    pub amount_minor: i64,
    /// ISO 4217 code.
    pub currency: String,
    /// The recipient's full MSISDN. Masked before it reaches any agent.
    pub recipient: String,
    /// Whether this recipient has an established payment history.
    #[serde(default)]
    pub established: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
pub enum Expect {
    Hold,
    Pass,
}

impl Case {
    pub fn money(&self) -> Money {
        let mut currency = *b"NGN";
        if self.currency.len() == 3 {
            currency.copy_from_slice(self.currency.as_bytes());
        }
        Money { minor_units: self.amount_minor, currency }
    }
}

/// A fixed instant, so every eval is reproducible.
pub fn epoch() -> DateTime<Utc> {
    Utc.timestamp_opt(1_700_000_000, 0).unwrap()
}

/// Run one case all the way through stub screening and the policy engine.
pub async fn run_case(case: &Case) -> PolicyDecision {
    let proposed_at = epoch();
    let received_at = proposed_at - Duration::minutes(case.minutes_later);

    let facts = TransferFacts {
        amount: case.money(),
        recipient: airlock_agents::mask_msisdn(&case.recipient)
            .unwrap_or_else(|| MaskedMsisdn("*".repeat(10))),
        recipient_established: case.established,
        minutes_since_contact: u32::try_from(case.minutes_later).ok(),
    };

    let screening = airlock_agents::screen_supervised(
        Reader::Stub,
        Linker::Stub,
        Untrusted::new(case.message.clone()),
        facts,
    )
    .await;

    decide(&DecisionInput {
        recipient: RecipientProfile { established: case.established },
        inbound_contact: Some(InboundContact { received_at }),
        screening,
        proposed_at,
    })
}

/// Screen a message without the policy engine, for tests that care about the
/// verdict itself rather than the decision.
pub async fn verdict_for(case: &Case) -> ScreeningOutcome {
    let facts = TransferFacts {
        amount: case.money(),
        recipient: airlock_agents::mask_msisdn(&case.recipient)
            .unwrap_or_else(|| MaskedMsisdn("*".repeat(10))),
        recipient_established: case.established,
        minutes_since_contact: u32::try_from(case.minutes_later).ok(),
    };
    airlock_agents::screen_supervised(
        Reader::Stub,
        Linker::Stub,
        Untrusted::new(case.message.clone()),
        facts,
    )
    .await
}

/// Load a corpus file from `fixtures/`.
pub fn load(name: &str) -> Vec<Case> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .join("fixtures")
        .join(name);
    let body = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("could not read {}: {e}", path.display()));
    serde_json::from_str(&body)
        .unwrap_or_else(|e| panic!("could not parse {}: {e}", path.display()))
}

/// Whether a decision held.
pub fn held(decision: &PolicyDecision) -> bool {
    matches!(decision, PolicyDecision::Hold { .. })
}

/// Verdicts that count as the Linker having linked the transfer to the
/// message.
pub fn is_responsive(outcome: &ScreeningOutcome) -> bool {
    matches!(outcome, ScreeningOutcome::Completed { verdict: Verdict::Responsive })
}
