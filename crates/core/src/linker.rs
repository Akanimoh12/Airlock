//! Linker agent output. `PlainReason` is an enum, not a string: the model
//! can never write what the user reads, C can style each variant properly,
//! and attacker-controlled text cannot reach the display.

/// Whether a proposed transfer answers the pressure a message applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Verdict {
    Responsive,
    Unrelated,
    Unknown,
}

/// The fixed set of reasons the UI is allowed to render. Deliberately not a
/// free-text field — the model cannot write what the user reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum PlainReason {
    EstablishedRecipient,
    NovelRecipientUnsolicitedContact,
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
