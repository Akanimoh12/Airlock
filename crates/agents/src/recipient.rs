//! The Recipient agent: "Is this a risky account?"
//!
//! **Sees:** destination account age, count of first-time inbound payments in a recent window,
//! count of distinct payers in that window.
//! **Denied:** message text, message metadata, sender identity, payer's own history, amount.
//!
//! The Recipient agent turns the hold screen from advice into evidence: "this account was
//! opened 6 days ago and 4 other people paid it for the first time in the last hour."

use airlock_core::RecipientRisk;
use serde::{Deserialize, Serialize};

/// Everything the Recipient agent is allowed to see.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecipientView {
    /// Days since account creation.
    pub account_age_days: u32,
    /// Count of distinct payers making first-time payments in the correlation window.
    pub new_payer_count: u32,
    /// Count of inbound payments from new payers in the correlation window.
    pub new_payment_count: u32,
}

impl RecipientView {
    /// Evaluate recipient account risk based on activity patterns.
    pub fn assess(&self) -> RecipientRisk {
        // New account is high risk.
        if self.account_age_days < 14 {
            return RecipientRisk::NewAccount;
        }

        // Fanning pattern: multiple first-time payers in a short window.
        if self.new_payer_count >= 3 && self.new_payment_count >= 3 {
            return RecipientRisk::Fanning;
        }

        RecipientRisk::Unremarkable
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_account_is_risky() {
        let view = RecipientView {
            account_age_days: 7,
            new_payer_count: 1,
            new_payment_count: 1,
        };
        assert_eq!(view.assess(), RecipientRisk::NewAccount);
    }

    #[test]
    fn fanning_pattern_is_risky() {
        let view = RecipientView {
            account_age_days: 60,
            new_payer_count: 4,
            new_payment_count: 4,
        };
        assert_eq!(view.assess(), RecipientRisk::Fanning);
    }

    #[test]
    fn established_account_unremarkable() {
        let view = RecipientView {
            account_age_days: 120,
            new_payer_count: 0,
            new_payment_count: 0,
        };
        assert_eq!(view.assess(), RecipientRisk::Unremarkable);
    }

    #[test]
    fn single_new_payer_unremarkable() {
        let view = RecipientView {
            account_age_days: 60,
            new_payer_count: 1,
            new_payment_count: 1,
        };
        assert_eq!(view.assess(), RecipientRisk::Unremarkable);
    }
}
