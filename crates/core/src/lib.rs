//! Deterministic core: trust types, the transaction state machine, and the
//! event contract. No model, no network, no I/O — this crate must compile
//! and pass its tests with the network unplugged.

pub mod events;
pub mod linker;
pub mod signal;
pub mod transaction;
pub mod trust;

pub use events::{AirlockEvent, Component, Timestamp};
pub use linker::{PlainReason, Responsiveness, Verdict};
pub use signal::{
    ClaimedAuthority, Confidence, MaskedMsisdn, Money, PressureSignal, RequestedAction, Urgency,
};
pub use transaction::{state, InvalidTransition, StateTag, Transaction, TransactionState, TxnId};
pub use trust::{Evidence, Source, Untrusted, Validated};
