//! In-memory stores: the ledger, the inbox, and transaction records.
//!
//! Deliberately not a database. The brief rules out deployment infra, and
//! everything here is demo-lifetime state.
//!
//! The one thing worth being careful about is where full MSISDNs live. They
//! are held here, server-side, and are masked on the way out to any agent and
//! on the way out to the UI. Nothing downstream of this module sees one.

use airlock_core::{MaskedMsisdn, Money, PlainReason, Timestamp, TransactionState, TxnId, Untrusted};
use airlock_policy::CORRELATION_WINDOW;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

/// An inbound message, held as `Untrusted` from the moment it arrives. The
/// Reader is the only thing that ever unwraps it.
pub struct InboundMessage {
    pub text: Untrusted<String>,
    pub received_at: Timestamp,
}

#[derive(Default)]
pub struct Inbox {
    messages: Vec<InboundMessage>,
}

impl Inbox {
    pub fn record(&mut self, text: String, received_at: Timestamp) {
        self.messages.push(InboundMessage {
            text: Untrusted::new(text),
            received_at,
        });
    }

    /// The most recent message inside the correlation window, if any. This
    /// is what makes a transfer "responsive to" something rather than
    /// merely following it.
    pub fn most_recent_within_window(&self, now: Timestamp) -> Option<&InboundMessage> {
        self.messages
            .iter()
            .filter(|m| {
                let age = now - m.received_at;
                age >= chrono::Duration::zero() && age <= CORRELATION_WINDOW
            })
            .max_by_key(|m| m.received_at)
    }

    pub fn len(&self) -> usize {
        self.messages.len()
    }

    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }
}

/// Recipients with an established payment history. Rule 1 turns on this and
/// nothing else.
pub struct Ledger {
    established: Vec<String>,
}

impl Ledger {
    /// Seeded with the recipients the demo wallet has "paid before". A judge
    /// sending to any other number is a novel recipient.
    pub fn seeded() -> Self {
        Ledger {
            established: vec![
                "08055512345".to_string(), // landlord — paid monthly
                "08099987654".to_string(), // airtime top-up
                "08033344556".to_string(), // sister
            ],
        }
    }

    pub fn is_established(&self, msisdn: &str) -> bool {
        let subscriber = subscriber_digits(msisdn);
        self.established
            .iter()
            .any(|e| subscriber_digits(e) == subscriber)
    }
}

/// How many trailing digits identify a subscriber regardless of how the
/// number was written.
///
/// A recipient can arrive as `08031234567`, `+2348031234567` or
/// `0803 123 4567`, and rule 1 turning on string equality would make the
/// same landlord "established" or "novel" depending on typing. Comparing the
/// trailing digits sidesteps country codes and trunk prefixes without
/// needing a table of dialling plans: nine covers Nigerian and Kenyan
/// numbering, which is as far as the demo goes.
const SUBSCRIBER_DIGITS: usize = 9;

fn subscriber_digits(s: &str) -> String {
    let digits: String = s.chars().filter(char::is_ascii_digit).collect();
    let start = digits.len().saturating_sub(SUBSCRIBER_DIGITS);
    digits[start..].to_string()
}

/// A transaction as stored. Uses `TransactionState` rather than A's
/// `Transaction<S>` typestate: a map holds transactions in several different
/// states at once, which is exactly the case A's module doc calls out as
/// needing the plain enum. Every change still goes through
/// `TransactionState::transition`, so the same edges are enforced.
#[derive(Debug, Clone)]
pub struct TxnRecord {
    pub id: TxnId,
    pub state: TransactionState,
    pub amount: Money,
    /// Full number. Never leaves this struct — see `masked`.
    pub recipient: String,
    pub masked: MaskedMsisdn,
    pub recipient_established: bool,
    pub proposed_at: Timestamp,
    pub releases_at: Option<Timestamp>,
    pub reason: Option<PlainReason>,
}

#[derive(Default)]
pub struct TxnStore {
    next_id: AtomicU64,
    records: HashMap<TxnId, TxnRecord>,
}

impl TxnStore {
    pub fn next_id(&self) -> TxnId {
        TxnId(self.next_id.fetch_add(1, Ordering::Relaxed) + 1)
    }

    pub fn insert(&mut self, record: TxnRecord) {
        self.records.insert(record.id, record);
    }

    pub fn get(&self, id: TxnId) -> Option<&TxnRecord> {
        self.records.get(&id)
    }

    pub fn get_mut(&mut self, id: TxnId) -> Option<&mut TxnRecord> {
        self.records.get_mut(&id)
    }

    /// All transactions, newest first. Backs the snapshot endpoint so a UI
    /// that connects mid-flight isn't left blank until the next event.
    pub fn all(&self) -> Vec<TxnRecord> {
        let mut all: Vec<_> = self.records.values().cloned().collect();
        all.sort_by_key(|r| std::cmp::Reverse(r.id.0));
        all
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};

    #[test]
    fn the_ledger_ignores_formatting_differences() {
        let ledger = Ledger::seeded();
        for spelling in [
            "08055512345",
            "0805 551 2345",
            "+2348055512345",
            "+234-805-551-2345",
        ] {
            assert!(ledger.is_established(spelling), "{spelling} should be known");
        }
        assert!(!ledger.is_established("08031234567"));
    }

    #[test]
    fn the_inbox_returns_the_most_recent_message_in_window() {
        let now = Utc::now();
        let mut inbox = Inbox::default();
        inbox.record("older".into(), now - Duration::minutes(8));
        inbox.record("newer".into(), now - Duration::minutes(2));
        let found = inbox.most_recent_within_window(now).unwrap();
        assert_eq!(found.text.expose_to_reader(), "newer");
    }

    #[test]
    fn a_message_older_than_the_window_does_not_count() {
        let now = Utc::now();
        let mut inbox = Inbox::default();
        inbox.record("stale".into(), now - CORRELATION_WINDOW - Duration::seconds(1));
        assert!(inbox.most_recent_within_window(now).is_none());
    }

    #[test]
    fn ids_are_unique_and_start_at_one() {
        let store = TxnStore::default();
        assert_eq!(store.next_id(), TxnId(1));
        assert_eq!(store.next_id(), TxnId(2));
    }
}
