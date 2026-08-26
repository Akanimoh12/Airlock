//! The six deterministic policy rules. Pure functions, no model, no I/O.
//! Every rule here has a corresponding test — see `tests` below.

use airlock_core::{PlainReason, RecipientRisk, Timestamp, Verdict};
use chrono::Duration;

/// Hold duration is fixed in code — no caller can pass in an override.
/// Placeholder value; see team_brief.md "Open items" (concrete duration
/// still needs to be agreed).
pub const HOLD_DURATION: Duration = Duration::seconds(60);

/// How far back an inbound message can be and still count as the cause of
/// a transfer. Placeholder value; see team_brief.md "Open items".
pub const CORRELATION_WINDOW: Duration = Duration::minutes(10);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecipientProfile {
    /// True if this recipient has an established payment history.
    pub established: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InboundContact {
    pub received_at: Timestamp,
}

/// The outcome of screening: either it completed with a verdict, or it
/// didn't complete at all (crash, timeout, malformed output).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScreeningOutcome {
    Completed { verdict: Verdict },
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DecisionInput {
    pub recipient: RecipientProfile,
    pub inbound_contact: Option<InboundContact>,
    pub screening: ScreeningOutcome,
    pub recipient_risk: RecipientRisk,
    pub proposed_at: Timestamp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyDecision {
    Pass,
    Hold {
        reason: PlainReason,
        releases_at: Timestamp,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ReleaseError {
    #[error("cooling period has not elapsed yet")]
    CoolingPeriodNotElapsed,
}

/// The six policy rules, in order:
/// 1. established recipient -> pass, regardless of any message.
/// 2. novel recipient + unsolicited inbound contact inside the correlation
///    window + responsive verdict -> hold.
/// 3. screening unavailable + novel recipient -> hold (fail-closed).
/// 4. hold duration is fixed (`HOLD_DURATION`); nothing here accepts a
///    caller-supplied override.
/// 5. release requires an explicit call to `release` after the cooling
///    period elapses (see below) — the model has no path to it.
/// 6. recipient risk signals (new account, fanning pattern) -> hold.
pub fn decide(input: &DecisionInput) -> PolicyDecision {
    // Rule 1.
    if input.recipient.established {
        return PolicyDecision::Pass;
    }

    // Rule 6: recipient risk (additive only — can add holds, never remove them).
    match input.recipient_risk {
        RecipientRisk::NewAccount | RecipientRisk::Fanning => {
            return PolicyDecision::Hold {
                reason: PlainReason::NovelRecipientHighRisk,
                releases_at: input.proposed_at + HOLD_DURATION,
            };
        }
        RecipientRisk::Unknown => {
            // Unknown recipient risk on a novel recipient -> hold (fail-closed).
            return PolicyDecision::Hold {
                reason: PlainReason::NovelRecipientHighRisk,
                releases_at: input.proposed_at + HOLD_DURATION,
            };
        }
        RecipientRisk::Unremarkable => {} // continue to next rules
    }

    // Rule 3: fail-closed. A novel recipient with no completed screening
    // always holds, no matter what else is true.
    let verdict = match input.screening {
        ScreeningOutcome::Unavailable => {
            return PolicyDecision::Hold {
                reason: PlainReason::ScreeningUnavailable,
                releases_at: input.proposed_at + HOLD_DURATION,
            };
        }
        ScreeningOutcome::Completed { verdict } => verdict,
    };

    // Rule 2: novel recipient + unsolicited contact in-window + responsive.
    if let Some(contact) = input.inbound_contact {
        let age = input.proposed_at - contact.received_at;
        let within_window = age >= Duration::zero() && age <= CORRELATION_WINDOW;
        if within_window && verdict == Verdict::Responsive {
            return PolicyDecision::Hold {
                reason: PlainReason::NovelRecipientUnsolicitedContact,
                releases_at: input.proposed_at + HOLD_DURATION,
            };
        }
    }

    PolicyDecision::Pass
}

/// Rule 5: releasing a held transfer requires the cooling period to have
/// elapsed. There is no path from model output to this function — only a
/// user action (handled by the runtime/API layer) calls it.
pub fn release(releases_at: Timestamp, now: Timestamp) -> Result<PlainReason, ReleaseError> {
    if now >= releases_at {
        Ok(PlainReason::UserReleased)
    } else {
        Err(ReleaseError::CoolingPeriodNotElapsed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn t(mins: i64) -> Timestamp {
        chrono::Utc.timestamp_opt(1_700_000_000, 0).unwrap() + Duration::minutes(mins)
    }

    fn base_input() -> DecisionInput {
        DecisionInput {
            recipient: RecipientProfile { established: false },
            inbound_contact: None,
            screening: ScreeningOutcome::Completed { verdict: Verdict::Unrelated },
            recipient_risk: RecipientRisk::Unremarkable,
            proposed_at: t(0),
        }
    }

    #[test]
    fn rule1_established_recipient_always_passes() {
        let mut input = base_input();
        input.recipient.established = true;
        input.screening = ScreeningOutcome::Unavailable; // even fail-closed conditions
        assert_eq!(decide(&input), PolicyDecision::Pass);
    }

    #[test]
    fn rule2_novel_recipient_responsive_in_window_holds() {
        let mut input = base_input();
        input.inbound_contact = Some(InboundContact { received_at: t(-4) });
        input.screening = ScreeningOutcome::Completed { verdict: Verdict::Responsive };
        match decide(&input) {
            PolicyDecision::Hold { reason, .. } => {
                assert_eq!(reason, PlainReason::NovelRecipientUnsolicitedContact)
            }
            other => panic!("expected Hold, got {other:?}"),
        }
    }

    #[test]
    fn rule2_does_not_hold_outside_correlation_window() {
        let mut input = base_input();
        input.inbound_contact = Some(InboundContact { received_at: t(-30) });
        input.screening = ScreeningOutcome::Completed { verdict: Verdict::Responsive };
        assert_eq!(decide(&input), PolicyDecision::Pass);
    }

    #[test]
    fn rule2_does_not_hold_when_verdict_unrelated() {
        let mut input = base_input();
        input.inbound_contact = Some(InboundContact { received_at: t(-4) });
        input.screening = ScreeningOutcome::Completed { verdict: Verdict::Unrelated };
        assert_eq!(decide(&input), PolicyDecision::Pass);
    }

    #[test]
    fn rule3_dead_reader_on_novel_recipient_fails_closed() {
        let mut input = base_input();
        input.screening = ScreeningOutcome::Unavailable;
        match decide(&input) {
            PolicyDecision::Hold { reason, .. } => {
                assert_eq!(reason, PlainReason::ScreeningUnavailable)
            }
            other => panic!("dead Reader must produce Held, got {other:?}"),
        }
    }

    #[test]
    fn rule4_hold_duration_is_fixed_and_applied() {
        let mut input = base_input();
        input.screening = ScreeningOutcome::Unavailable;
        match decide(&input) {
            PolicyDecision::Hold { releases_at, .. } => {
                assert_eq!(releases_at, input.proposed_at + HOLD_DURATION);
            }
            other => panic!("expected Hold, got {other:?}"),
        }
    }

    #[test]
    fn rule5_release_rejected_before_cooling_period_elapses() {
        let releases_at = t(1);
        let now = t(0);
        assert_eq!(release(releases_at, now), Err(ReleaseError::CoolingPeriodNotElapsed));
    }

    #[test]
    fn rule5_release_allowed_once_cooling_period_elapses() {
        let releases_at = t(1);
        let now = t(1);
        assert_eq!(release(releases_at, now), Ok(PlainReason::UserReleased));
    }

    #[test]
    fn rule6_new_account_risk_holds() {
        let mut input = base_input();
        input.recipient_risk = RecipientRisk::NewAccount;
        match decide(&input) {
            PolicyDecision::Hold { reason, .. } => {
                assert_eq!(reason, PlainReason::NovelRecipientHighRisk)
            }
            other => panic!("new account risk must hold, got {other:?}"),
        }
    }

    #[test]
    fn rule6_fanning_risk_holds() {
        let mut input = base_input();
        input.recipient_risk = RecipientRisk::Fanning;
        match decide(&input) {
            PolicyDecision::Hold { reason, .. } => {
                assert_eq!(reason, PlainReason::NovelRecipientHighRisk)
            }
            other => panic!("fanning risk must hold, got {other:?}"),
        }
    }

    #[test]
    fn rule6_unknown_risk_on_novel_recipient_holds() {
        let mut input = base_input();
        input.recipient_risk = RecipientRisk::Unknown;
        match decide(&input) {
            PolicyDecision::Hold { reason, .. } => {
                assert_eq!(reason, PlainReason::NovelRecipientHighRisk)
            }
            other => panic!("unknown risk on novel recipient must hold, got {other:?}"),
        }
    }

    #[test]
    fn rule6_unremarkable_risk_continues_to_other_rules() {
        let mut input = base_input();
        input.recipient_risk = RecipientRisk::Unremarkable;
        // Should pass since no other conditions trigger holds.
        assert_eq!(decide(&input), PolicyDecision::Pass);
    }
}
