//! The Linker: "is this transfer responsive to that contact?"
//!
//! The brief promises the Linker "cannot be prompt-injected because it is
//! never shown attacker-controlled prose". `validate` makes the strings in
//! a `PressureSignal` safe; `LinkerView` goes further and removes them.
//!
//! **`LinkerView` contains no `String`, and no variant carrying one.** Every
//! field is an enum, a bool or an integer, so the entire input to the Linker
//! is drawn from a finite set this crate defines. The comparisons that
//! actually matter — did the message name this amount, did it name this
//! recipient — are computed here in Rust and handed over as
//! `FactMatch`, not as two strings for a model to compare.
//!
//! That is the difference between "we sanitise the prose" and "there is no
//! prose". `linker_view_has_no_free_text` in the eval suite holds the line.

use airlock_core::{
    Confidence, MaskedMsisdn, Money, PlainReason, PressureSignal, RequestedAction, Responsiveness,
    Urgency, Validated, Verdict,
};

/// `RequestedAction` with its free-text payload removed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ActionKind {
    SendMoney,
    ShareCredentials,
    CallNumber,
    Other,
}

impl From<&RequestedAction> for ActionKind {
    fn from(action: &RequestedAction) -> Self {
        match action {
            RequestedAction::SendMoney => ActionKind::SendMoney,
            RequestedAction::ShareCredentials => ActionKind::ShareCredentials,
            RequestedAction::CallNumber => ActionKind::CallNumber,
            RequestedAction::Other(_) => ActionKind::Other,
        }
    }
}

/// Whether a fact the message named lines up with the transfer in hand.
/// Computed in Rust — the Linker is told the answer, not asked to work it
/// out from two strings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum FactMatch {
    /// The message named this, and it matches the transfer.
    Matches,
    /// The message named something, and it is not what is being sent.
    Differs,
    /// The message named nothing of this kind.
    NotNamed,
}

/// Facts the API knows about the proposed transfer. Read from the ledger and
/// the user's own request — never from a message. Used only to compute the
/// `FactMatch` fields below; none of it is handed to the Linker directly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransferFacts {
    pub amount: Money,
    pub recipient: MaskedMsisdn,
    pub recipient_established: bool,
    pub minutes_since_contact: Option<u32>,
}

/// Everything the Linker is allowed to see. No `String`, by construction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct LinkerView {
    pub urgency: Urgency,
    pub action: ActionKind,
    /// The message claimed to be from some institution. *Which* institution
    /// is deliberately not carried — it is not needed to judge
    /// responsiveness, and carrying it would reopen the text channel.
    pub authority_claimed: bool,
    pub confidence: Confidence,
    pub amount: FactMatch,
    pub recipient: FactMatch,
    pub recipient_established: bool,
    pub minutes_since_contact: Option<u32>,
}

impl LinkerView {
    /// Project a validated signal and the transfer's own facts into the
    /// narrow view. This is the only constructor.
    pub fn project(signal: &Validated<PressureSignal>, facts: &TransferFacts) -> Self {
        let signal = signal.get();
        LinkerView {
            urgency: signal.urgency,
            action: ActionKind::from(&signal.requested_action),
            authority_claimed: signal.authority_claim.is_some(),
            confidence: signal.confidence,
            amount: compare(signal.named_amount.as_ref(), Some(&facts.amount)),
            recipient: compare(signal.named_recipient.as_ref(), Some(&facts.recipient)),
            recipient_established: facts.recipient_established,
            minutes_since_contact: facts.minutes_since_contact,
        }
    }
}

fn compare<T: PartialEq>(named: Option<&T>, actual: Option<&T>) -> FactMatch {
    match (named, actual) {
        (Some(n), Some(a)) if n == a => FactMatch::Matches,
        (Some(_), _) => FactMatch::Differs,
        (None, _) => FactMatch::NotNamed,
    }
}

/// The Linker. Stub mode is deterministic and offline; a model-backed
/// variant slots in behind the same `judge` call and still receives only a
/// `LinkerView`.
#[derive(Debug, Clone, Copy, Default)]
pub enum Linker {
    #[default]
    Stub,
}

impl Linker {
    pub fn judge(&self, view: &LinkerView) -> Responsiveness {
        match self {
            Linker::Stub => stub_judge(view),
        }
    }
}

/// Deterministic responsiveness rules.
///
/// The question is not "is this a scam" — it is "does this transfer answer
/// the pressure that message applied". Note *pressure*: *correspondence
/// alone is not responsiveness*, and getting that wrong is expensive.
///
/// An earlier version treated an exact amount or recipient match as
/// responsive on its own. It held "Invoice attached for the office supplies,
/// N45,000" and "My number is 08111222333" — which is not scam behaviour,
/// it is what arranging a payment looks like. Measured on the legitimate
/// corpus it cost 40% false positives.
///
/// So urgency gates everything, which is also what the README claims the
/// whole design rests on: "these scams run on manufactured urgency… remove
/// the hurry and most of them collapse." A message applying no pressure has
/// nothing for a transfer to be responsive *to*, however exactly the numbers
/// line up.
///
/// The rationale is a `PlainReason` from A's fixed enum, but note that the
/// policy engine picks its own reason and ignores this one — see the handoff
/// note. It is filled in consistently anyway so the field never misleads.
fn stub_judge(view: &LinkerView) -> Responsiveness {
    let unrelated = Responsiveness {
        verdict: Verdict::Unrelated,
        rationale: PlainReason::EstablishedRecipient,
    };

    // Nothing to be responsive to.
    if view.minutes_since_contact.is_none() {
        return Responsiveness {
            verdict: Verdict::Unknown,
            rationale: PlainReason::EstablishedRecipient,
        };
    }

    // No pressure, no responsiveness — whatever else matches.
    if view.urgency == Urgency::None {
        return unrelated;
    }

    // A channel to act on the pressure: send, call, hand over a code.
    let actionable = matches!(
        view.action,
        ActionKind::SendMoney | ActionKind::CallNumber | ActionKind::ShareCredentials
    );
    let names_this_transfer =
        view.amount == FactMatch::Matches || view.recipient == FactMatch::Matches;

    let responsive = match view.urgency {
        // Urgency plus a way to act on it is the shape all of these take.
        // Urgency plus this transfer's own details is the same thing said
        // more precisely.
        Urgency::High => actionable || names_this_transfer,
        // Mild pressure has to be corroborated before it counts.
        Urgency::Low => actionable && names_this_transfer,
        Urgency::None => unreachable!("handled above"),
    };

    if responsive {
        Responsiveness {
            verdict: Verdict::Responsive,
            rationale: PlainReason::NovelRecipientUnsolicitedContact,
        }
    } else {
        unrelated
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::validate::mask_msisdn;

    fn ngn(major: i64) -> Money {
        Money { minor_units: major * 100, currency: *b"NGN" }
    }

    fn facts() -> TransferFacts {
        TransferFacts {
            amount: ngn(5_000),
            recipient: mask_msisdn("08031234567").unwrap(),
            recipient_established: false,
            minutes_since_contact: Some(4),
        }
    }

    fn view() -> LinkerView {
        LinkerView {
            urgency: Urgency::High,
            action: ActionKind::CallNumber,
            authority_claimed: true,
            confidence: Confidence::High,
            amount: FactMatch::NotNamed,
            recipient: FactMatch::NotNamed,
            recipient_established: false,
            minutes_since_contact: Some(4),
        }
    }

    #[test]
    fn projection_drops_the_free_text_and_keeps_the_shape() {
        let signal = Validated::from_trusted_source(PressureSignal {
            urgency: Urgency::High,
            authority_claim: Some("MTN".to_string()),
            requested_action: RequestedAction::Other("whatever the reader wrote".to_string()),
            named_amount: Some(ngn(5_000)),
            named_recipient: None,
            confidence: Confidence::Medium,
        });
        let projected = LinkerView::project(&signal, &facts());
        assert!(projected.authority_claimed);
        assert_eq!(projected.action, ActionKind::Other);
        assert_eq!(projected.amount, FactMatch::Matches);
        assert_eq!(projected.recipient, FactMatch::NotNamed);
    }

    #[test]
    fn a_calm_message_naming_the_amount_is_coordination_not_pressure() {
        // "Invoice attached for the office supplies, N45,000 due end of
        // month", then that exact transfer. Correspondence without urgency.
        let mut v = view();
        v.urgency = Urgency::None;
        v.action = ActionKind::Other;
        v.amount = FactMatch::Matches;
        assert_eq!(Linker::Stub.judge(&v).verdict, Verdict::Unrelated);
    }

    #[test]
    fn a_calm_message_naming_the_recipient_is_also_not_pressure() {
        // "Thanks for lunch! My number is 08111222333."
        let mut v = view();
        v.urgency = Urgency::None;
        v.action = ActionKind::Other;
        v.recipient = FactMatch::Matches;
        assert_eq!(Linker::Stub.judge(&v).verdict, Verdict::Unrelated);
    }

    #[test]
    fn urgency_plus_this_transfers_own_details_is_responsive() {
        let mut v = view();
        v.action = ActionKind::Other;
        v.amount = FactMatch::Matches;
        assert_eq!(Linker::Stub.judge(&v).verdict, Verdict::Responsive);
    }

    #[test]
    fn mild_pressure_needs_corroborating_before_it_counts() {
        let mut v = view();
        v.urgency = Urgency::Low;
        v.action = ActionKind::SendMoney;
        assert_eq!(Linker::Stub.judge(&v).verdict, Verdict::Unrelated);

        v.amount = FactMatch::Matches;
        assert_eq!(Linker::Stub.judge(&v).verdict, Verdict::Responsive);
    }

    #[test]
    fn the_demo_scam_is_responsive() {
        // "MTN Alert: your account will be suspended today. Call this
        // number to reactivate." — no amount, no recipient, pure urgency.
        assert_eq!(Linker::Stub.judge(&view()).verdict, Verdict::Responsive);
    }

    #[test]
    fn a_calm_unrelated_message_is_not_responsive() {
        let mut v = view();
        v.urgency = Urgency::None;
        v.action = ActionKind::Other;
        assert_eq!(Linker::Stub.judge(&v).verdict, Verdict::Unrelated);
    }

    #[test]
    fn no_contact_means_nothing_to_be_responsive_to() {
        let mut v = view();
        v.minutes_since_contact = None;
        assert_eq!(Linker::Stub.judge(&v).verdict, Verdict::Unknown);
    }

    #[test]
    fn a_differing_amount_does_not_by_itself_make_it_responsive() {
        let mut v = view();
        v.urgency = Urgency::None;
        v.action = ActionKind::Other;
        v.amount = FactMatch::Differs;
        assert_eq!(Linker::Stub.judge(&v).verdict, Verdict::Unrelated);
    }
}
