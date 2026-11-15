//! The bridge into the core crate's debug logger.
//!
//! The core logger gates on `APHRODITE_DEBUG`, captured at launch. These leaves
//! OR in the session flag-file toggle so the `aphrodite_debug` tool can turn
//! logging on for a session tree even when the env var was never exported - and
//! so a caller can ask "is anything on?" before paying for timing capture.

pub(crate) mod Any;
pub(crate) mod Bridge;
pub(crate) mod Level;
pub(crate) mod Line;
