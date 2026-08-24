//! What SSE emits, what the product surface renders. Every field here is
//! typed — no free text from a model reaches this enum.

use crate::linker::PlainReason;
use crate::transaction::{TransactionState, TxnId};
use chrono::{DateTime, Utc};

pub type Timestamp = DateTime<Utc>;

/// The component whose failure triggered a fail-closed hold.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Component {
    Reader,
    Linker,
    PolicyEngine,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type")]
pub enum AirlockEvent {
    StateChanged {
        txn: TxnId,
        from: TransactionState,
        to: TransactionState,
    },
    HoldOpened {
        txn: TxnId,
        reason: PlainReason,
        releases_at: Timestamp,
    },
    ScreenFailed {
        txn: TxnId,
        component: Component,
    },
}
