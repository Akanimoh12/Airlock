//! Deterministic policy rules — no model calls, fully tested, no network.

pub mod rules;

pub use rules::{
    decide, release, DecisionInput, InboundContact, PolicyDecision, RecipientProfile,
    ReleaseError, ScreeningOutcome, CORRELATION_WINDOW, HOLD_DURATION,
};
