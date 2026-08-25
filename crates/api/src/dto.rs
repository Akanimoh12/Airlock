//! Wire types.
//!
//! Separate from the core types on purpose. `Money.currency` is `[u8; 3]`,
//! which serde renders as `[78, 71, 78]` — correct, and unusable for the UI.
//! Anything crossing to the product surface gets a shape a frontend can read
//! without a decoder ring.
//!
//! `PlainReason` crosses as its variant name. That is a stable key for the
//! UI to switch on, not copy to display — the user sees "paused for a
//! moment", never `NovelRecipientUnsolicitedContact`.

use crate::store::TxnRecord;
use airlock_core::{Money, PlainReason, TransactionState, TxnId};

#[derive(Debug, Clone, serde::Serialize)]
pub struct MoneyView {
    pub minor_units: i64,
    pub currency: String,
}

impl From<Money> for MoneyView {
    fn from(m: Money) -> Self {
        MoneyView {
            minor_units: m.minor_units,
            currency: String::from_utf8_lossy(&m.currency).into_owned(),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct TxnView {
    pub id: TxnId,
    pub state: TransactionState,
    pub amount: MoneyView,
    /// Masked. The full number never leaves the store.
    pub recipient: String,
    pub recipient_established: bool,
    pub proposed_at: String,
    pub releases_at: Option<String>,
    pub reason: Option<PlainReason>,
    /// Whether the cooling period has elapsed, computed server-side.
    ///
    /// The UI may render a countdown from `releases_at` — that is a server
    /// timestamp, not an invented one — but whether release is actually
    /// permitted is this field, and the server enforces it again on the
    /// release call regardless of what the client believes.
    pub releasable: bool,
}

impl TxnView {
    pub fn of(record: &TxnRecord, now: airlock_core::Timestamp) -> Self {
        TxnView {
            id: record.id,
            state: record.state,
            amount: record.amount.into(),
            recipient: record.masked.0.clone(),
            recipient_established: record.recipient_established,
            proposed_at: record.proposed_at.to_rfc3339(),
            releases_at: record.releases_at.map(|t| t.to_rfc3339()),
            reason: record.reason,
            releasable: record.state == TransactionState::Held
                && record.releases_at.is_some_and(|t| now >= t),
        }
    }
}

#[derive(Debug, serde::Deserialize)]
pub struct InboundRequest {
    pub text: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct TransferRequest {
    pub recipient: String,
    pub amount_minor: i64,
    #[serde(default = "default_currency")]
    pub currency: String,
}

fn default_currency() -> String {
    "NGN".to_string()
}

#[derive(Debug, serde::Serialize)]
pub struct HealthView {
    pub status: &'static str,
    /// Whether the Reader process is answering. Beat six makes this go
    /// false on stage, and the UI can show it.
    pub reader_reachable: bool,
    pub reader_mode: &'static str,
    pub inbox_messages: usize,
}

#[derive(Debug, serde::Serialize)]
pub struct ApiError {
    pub error: String,
}
