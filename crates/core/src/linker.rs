//! Linker and Recipient agent outputs. `PlainReason` is an enum, not a string: the model
//! can never write what the user reads, C can style each variant properly,
//! and attacker-controlled text cannot reach the display.

/// Whether a proposed transfer answers the pressure a message applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Verdict {
    Responsive,
    Unrelated,
    Unknown,
}

/// Risk assessment of a recipient account.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum RecipientRisk {
    /// Account appears normal. Established history or ordinary activity patterns.
    Unremarkable,
    /// Very new account (< 14 days old).
    NewAccount,
    /// Multiple first-time payers in a short window — classic fanning pattern.
    Fanning,
    /// Agent unavailable or unable to assess.
    Unknown,
}

/// The fixed set of reasons the UI is allowed to render. Deliberately not a
/// free-text field — the model cannot write what the user reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum PlainReason {
    EstablishedRecipient,
    NovelRecipientUnsolicitedContact,
    NovelRecipientHighRisk,
    ScreeningUnavailable,
    UserReleased,
    CoolingPeriodNotElapsed,
}

/// Linker output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Responsiveness {
    pub verdict: Verdict,
    pub rationale: PlainReason,
}
