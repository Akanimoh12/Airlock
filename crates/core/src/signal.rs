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

/// Which institution a message claimed to be from, as a closed set.
///
/// This is deliberately an enum and not the `authority_claim` string beside
/// it. The product surface needs to know *who* was impersonated so it can say
/// "MTN will never ask you to pay to reactivate your line" — and that has to
/// reach the screen without opening a text channel from an attacker to a
/// user. A bounded classification can cross; prose cannot.
///
/// Schema validation rejects anything outside these variants, so a Reader
/// that has been talked into cooperating can at worst pick the wrong one of a
/// fixed list. It cannot invent a new one and it cannot smuggle text.
///
/// `Unknown` means a claim was detected but is not one we hold counter-advice
/// for. `None` means no institutional claim at all. Both fall back to generic
/// copy on the product surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum ClaimedAuthority {
    Mtn,
    Airtel,
    Glo,
    NineMobile,
    Safaricom,
    /// A mobile-money brand rather than the network itself (M-Pesa, MoMo).
    MobileMoney,
    /// Any retail bank or wallet.
    Bank,
    /// A regulator or agency (CBN, EFCC, NIMC, NCC).
    Government,
    /// A claim was made, but not one we recognise.
    Unknown,
    #[default]
    None,
}

/// Reader output — raw text stops here.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PressureSignal {
    pub urgency: Urgency,
    pub authority_claim: Option<String>,
    /// The same claim as `authority_claim`, reduced to a closed set. This is
    /// the one that may cross to the product surface.
    #[serde(default)]
    pub claimed_authority: ClaimedAuthority,
    pub requested_action: RequestedAction,
    pub named_amount: Option<Money>,
    pub named_recipient: Option<MaskedMsisdn>,
    pub confidence: Confidence,
}
