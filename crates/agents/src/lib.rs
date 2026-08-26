//! Reader and Linker agents. Owned by Track B.
//!
//! Two rules shape everything here, both from docs/team_brief.md:
//!
//! - **Raw text stops at the Reader.** The Reader is the only component that
//!   sees attacker-controlled prose, and it has no account access and no
//!   path to the policy engine.
//! - **The Linker never sees prose at all.** It receives [`LinkerView`],
//!   which contains no `String` and no variant carrying one — every field is
//!   an enum, a bool or an integer this crate defines. The amount and
//!   recipient comparisons are computed in Rust and handed over as
//!   [`FactMatch`].
//!
//! Between those two sits [`validate`], because Reader output is not trusted
//! either: `PressureSignal` has three free-text fields, and a Reader that has
//! been talked into cooperating can put anything in them.
//!
//! Stub mode is offline and needs no API key. It is the same code path as the
//! model-backed Reader — same validation, same projection — so the offline
//! demo is not a different system.

pub mod linker;
pub mod reader;
pub mod recipient;
pub mod screening;
pub mod validate;

pub use linker::{ActionKind, FactMatch, Linker, LinkerView, TransferFacts};
pub use reader::{analyse_message, Reader, ScreenError};
pub use recipient::RecipientView;
pub use screening::{
    screen, screen_reported, screen_supervised, supervised_screen, ScreeningReport,
    SCREENING_TIMEOUT,
};
pub use validate::{
    mask_msisdn, validate_reader_json, validate_reader_output, SanitisationReport, SchemaError,
};
