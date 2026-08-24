//! Orchestration, supervision and hold timers. Depends on `core` and
//! `policy` only — never on the agent implementation directly, so this
//! crate stays decoupled from Rig.

pub mod hold_timer;
pub mod supervisor;

pub use hold_timer::{remaining, wait_for_release};
pub use supervisor::screen_with_timeout;
