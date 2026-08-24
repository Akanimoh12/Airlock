//! The transaction lifecycle.
//!
//! `Transaction<S>` is the typestate machine: each state is a distinct
//! type, and only the methods below exist as edges, so an illegal
//! transition (e.g. calling `.clear()` on a `Transaction<Held>`) is a
//! compiler error, not a runtime one — see the lifecycle diagram in
//! README.md.
//!
//! `TransactionState` is the plain enum used at the two places a static
//! type can't survive: the wire (`AirlockEvent`) and storage. Its
//! `transition` method still checks the same edges at runtime for state
//! that was just deserialized and has no compile-time type to lean on.

use std::fmt;
use std::marker::PhantomData;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct TxnId(pub u64);

impl fmt::Display for TxnId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "txn:{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum TransactionState {
    Proposed,
    Screening,
    Cleared,
    Held,
    Released,
    Cancelled,
    Executed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("invalid transition: {from:?} -> {to:?}")]
pub struct InvalidTransition {
    pub from: TransactionState,
    pub to: TransactionState,
}

impl TransactionState {
    /// Attempt to move from `self` to `to`. Only the edges drawn in the
    /// README lifecycle diagram are legal:
    ///
    /// Proposed -> Screening
    /// Screening -> Cleared | Held
    /// Cleared -> Executed
    /// Held -> Released | Cancelled
    /// Released -> Executed
    pub fn transition(self, to: TransactionState) -> Result<TransactionState, InvalidTransition> {
        use TransactionState::*;
        let allowed = matches!(
            (self, to),
            (Proposed, Screening)
                | (Screening, Cleared)
                | (Screening, Held)
                | (Cleared, Executed)
                | (Held, Released)
                | (Held, Cancelled)
                | (Released, Executed)
        );
        if allowed {
            Ok(to)
        } else {
            Err(InvalidTransition { from: self, to })
        }
    }
}

/// Marker types for each lifecycle state, namespaced away from
/// `TransactionState`'s variants (which several tests below import via
/// glob). Never instantiated — they exist only to make `Transaction<S>` a
/// distinct type per state.
pub mod state {
    pub struct Proposed;
    pub struct Screening;
    pub struct Cleared;
    pub struct Held;
    pub struct Released;
    pub struct Cancelled;
    pub struct Executed;
}

/// Maps a marker type back to its wire representation.
pub trait StateTag {
    const STATE: TransactionState;
}

macro_rules! state_tag {
    ($marker:ty, $variant:ident) => {
        impl StateTag for $marker {
            const STATE: TransactionState = TransactionState::$variant;
        }
    };
}
state_tag!(state::Proposed, Proposed);
state_tag!(state::Screening, Screening);
state_tag!(state::Cleared, Cleared);
state_tag!(state::Held, Held);
state_tag!(state::Released, Released);
state_tag!(state::Cancelled, Cancelled);
state_tag!(state::Executed, Executed);

/// A transaction whose current lifecycle state is part of its type. Only
/// the transitions drawn in the README lifecycle diagram exist as methods,
/// so any other transition simply does not compile.
pub struct Transaction<S> {
    pub id: TxnId,
    _state: PhantomData<S>,
}

impl<S: StateTag> Transaction<S> {
    /// The wire/storage representation of this transaction's current state.
    pub fn state(&self) -> TransactionState {
        S::STATE
    }
}

impl Transaction<state::Proposed> {
    pub fn new(id: TxnId) -> Self {
        Transaction { id, _state: PhantomData }
    }

    pub fn into_screening(self) -> Transaction<state::Screening> {
        Transaction { id: self.id, _state: PhantomData }
    }
}

impl Transaction<state::Screening> {
    pub fn clear(self) -> Transaction<state::Cleared> {
        Transaction { id: self.id, _state: PhantomData }
    }

    pub fn hold(self) -> Transaction<state::Held> {
        Transaction { id: self.id, _state: PhantomData }
    }
}

impl Transaction<state::Cleared> {
    pub fn execute(self) -> Transaction<state::Executed> {
        Transaction { id: self.id, _state: PhantomData }
    }
}

impl Transaction<state::Held> {
    /// Rule 5 lives in `airlock-policy::release`, which must return `Ok`
    /// before callers may call this — there is no path from model output
    /// to this method.
    pub fn release(self) -> Transaction<state::Released> {
        Transaction { id: self.id, _state: PhantomData }
    }

    pub fn cancel(self) -> Transaction<state::Cancelled> {
        Transaction { id: self.id, _state: PhantomData }
    }
}

impl Transaction<state::Released> {
    pub fn execute(self) -> Transaction<state::Executed> {
        Transaction { id: self.id, _state: PhantomData }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use TransactionState::*;

    #[test]
    fn every_valid_edge_is_accepted() {
        let edges = [
            (Proposed, Screening),
            (Screening, Cleared),
            (Screening, Held),
            (Cleared, Executed),
            (Held, Released),
            (Held, Cancelled),
            (Released, Executed),
        ];
        for (from, to) in edges {
            assert_eq!(from.transition(to), Ok(to), "{from:?} -> {to:?} should be legal");
        }
    }

    #[test]
    fn skipping_screening_is_rejected() {
        assert!(Proposed.transition(Cleared).is_err());
        assert!(Proposed.transition(Held).is_err());
        assert!(Proposed.transition(Executed).is_err());
    }

    #[test]
    fn cannot_re_enter_or_go_backwards() {
        assert!(Executed.transition(Proposed).is_err());
        assert!(Cleared.transition(Screening).is_err());
        assert!(Held.transition(Screening).is_err());
        assert!(Cancelled.transition(Released).is_err());
    }

    #[test]
    fn cleared_cannot_be_held_and_held_cannot_be_cleared() {
        assert!(Cleared.transition(Held).is_err());
        assert!(Held.transition(Cleared).is_err());
    }

    #[test]
    fn typestate_happy_paths_produce_the_right_wire_state() {
        let txn = Transaction::<state::Proposed>::new(TxnId(1));
        let held = txn.into_screening().hold();
        assert_eq!(held.state(), TransactionState::Held);
        let released = held.release();
        assert_eq!(released.state(), TransactionState::Released);
        let executed = released.execute();
        assert_eq!(executed.state(), TransactionState::Executed);
    }

    #[test]
    fn typestate_cleared_path_executes_directly() {
        let txn = Transaction::<state::Proposed>::new(TxnId(2));
        let executed = txn.into_screening().clear().execute();
        assert_eq!(executed.state(), TransactionState::Executed);
    }

    // The following, if uncommented, must fail to compile — that's the
    // point: `Transaction<state::Held>` has no `.clear()` method, so
    // "hold then clear" is not merely rejected at runtime, it does not
    // exist as code.
    //
    // let bad = Transaction::<state::Proposed>::new(TxnId(3)).into_screening().hold().clear();
}

