//! Reader agent output. Raw message text stops here — everything downstream
//! of `PressureSignal` is typed data, never free-form prose.

/// How much manufactured urgency the message conveys.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Urgency {
    None,
    Low,
    High,
}

/// What action the message is pressuring the recipient to take.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum RequestedAction {
    SendMoney,
    ShareCredentials,
    CallNumber,
    Other(String),
}

/// A monetary amount in minor units (cents) plus an ISO 4217 currency code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Money {
    pub minor_units: i64,
    pub currency: [u8; 3],
}

/// A masked MSISDN (phone number) — never the full number, so a leaked
/// signal can't be used to re-identify a recipient.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MaskedMsisdn(pub String);

/// The Reader's confidence in its own extraction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub enum Confidence {
    Low,
    Medium,
    High,
}

/// Reader output — raw text stops here.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PressureSignal {
    pub urgency: Urgency,
    pub authority_claim: Option<String>,
    pub requested_action: RequestedAction,
    pub named_amount: Option<Money>,
    pub named_recipient: Option<MaskedMsisdn>,
    pub confidence: Confidence,
}
