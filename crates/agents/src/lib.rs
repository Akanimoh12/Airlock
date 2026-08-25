//! Reader and Linker agents. Owned by Track B.
//!
//! Raw message text stops at the Reader, and what the Reader emits is not
//! trusted either: `PressureSignal` carries free-text fields, and a Reader
//! that has been talked into cooperating can put anything in them. Every
//! signal passes through [`validate`] before anything downstream looks at
//! it.

pub mod validate;

pub use validate::{
    mask_msisdn, validate_reader_json, validate_reader_output, SanitisationReport, SchemaError,
};
